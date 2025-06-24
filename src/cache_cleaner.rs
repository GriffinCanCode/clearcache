use crate::cache_types::{CachePattern, CacheType};
use crate::utils::{calculate_directory_size, is_git_ignored, should_skip_directory};
use anyhow::Result;
use colored::*;

use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use std::time::Instant;
use tokio::fs;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct CleanResult {
    pub directories_cleaned: usize,
    pub files_deleted: u64,
    pub space_freed: u64,
    pub errors: Vec<String>,
}

pub struct CacheCleaner {
    root_directory: PathBuf,
    cache_types: Vec<CacheType>,
    parallel_threads: usize,
    recursive: bool,
    dry_run: bool,
    verbose: bool,
}

#[derive(Debug, Clone)]
struct CleanTask {
    path: PathBuf,
    pattern: CachePattern,
    cache_type: CacheType,
}

impl CacheCleaner {
    pub fn new(
        root_directory: PathBuf,
        cache_types: Vec<CacheType>,
        parallel_threads: usize,
        recursive: bool,
        dry_run: bool,
        verbose: bool,
    ) -> Self {
        Self {
            root_directory,
            cache_types,
            parallel_threads,
            recursive,
            dry_run,
            verbose,
        }
    }

    pub async fn clean(
        &self,
        total_size: Arc<AtomicU64>,
        total_files: Arc<AtomicU64>,
    ) -> Result<CleanResult> {
        let start_time = Instant::now();

        // Setup progress bar
        let progress = ProgressBar::new_spinner();
        progress.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        progress.set_message("Scanning directories...");

        // Collect all cache patterns
        let mut all_patterns = Vec::new();
        for cache_type in &self.cache_types {
            for pattern in cache_type.get_patterns() {
                all_patterns.push((cache_type.clone(), pattern));
            }
        }

        // Find all cache directories/files
        let tasks = self.find_cache_items(&all_patterns, &progress).await?;
        
        progress.set_message(format!("Found {} cache items to clean", tasks.len()));

        if tasks.is_empty() {
            progress.finish_with_message("No cache items found to clean");
            return Ok(CleanResult {
                directories_cleaned: 0,
                files_deleted: 0,
                space_freed: 0,
                errors: Vec::new(),
            });
        }

        // Handle Docker cleaning separately (requires Docker commands)
        let docker_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| t.cache_type == CacheType::Docker)
            .collect();

        let file_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| t.cache_type != CacheType::Docker)
            .collect();

        let mut errors = Vec::new();
        let mut directories_cleaned = 0;

        // Clean Docker caches if present
        if !docker_tasks.is_empty() {
            progress.set_message("Cleaning Docker caches...");
            match self.clean_docker_caches().await {
                Ok(_) => directories_cleaned += 1,
                Err(e) => errors.push(format!("Docker cleaning failed: {}", e)),
            }
        }

        // Clean file system caches in parallel
        if !file_tasks.is_empty() {
            progress.set_message("Cleaning file system caches...");
            
            // Convert to owned tasks for parallel processing
            let owned_tasks: Vec<CleanTask> = file_tasks.iter().map(|t| (*t).clone()).collect();
            
            // Process tasks in parallel
            let chunk_size = (owned_tasks.len() / self.parallel_threads).max(1);
            
            let results: Vec<Result<(usize, u64, u64, Vec<String>), anyhow::Error>> = owned_tasks
                .par_chunks(chunk_size)
                .map(|chunk| {
                    self.process_chunk(chunk, total_size.clone(), total_files.clone())
                })
                .collect();

            // Aggregate results
            for result in results {
                match result {
                    Ok((dirs, _files, _size, errs)) => {
                        directories_cleaned += dirs;
                        errors.extend(errs);
                    }
                    Err(e) => errors.push(e.to_string()),
                }
            }
        }

        let duration = start_time.elapsed();
        progress.finish_with_message(format!(
            "Completed in {:.2}s",
            duration.as_secs_f64()
        ));

        Ok(CleanResult {
            directories_cleaned,
            files_deleted: total_files.load(Ordering::Relaxed),
            space_freed: total_size.load(Ordering::Relaxed),
            errors,
        })
    }

    async fn find_cache_items(
        &self,
        patterns: &[(CacheType, CachePattern)],
        progress: &ProgressBar,
    ) -> Result<Vec<CleanTask>> {
        let mut tasks = Vec::new();
        let mut visited = HashSet::new();

        if self.recursive {
            // Recursive search
            for entry in WalkDir::new(&self.root_directory)
                .follow_links(false)
                .max_depth(10) // Reasonable depth limit
            {
                let entry = entry?;
                let path = entry.path();

                if should_skip_directory(path) || is_git_ignored(path) {
                    continue;
                }

                for (cache_type, pattern) in patterns {
                    if self.matches_pattern(path, pattern) {
                        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
                        if visited.insert(canonical.clone()) {
                            tasks.push(CleanTask {
                                path: canonical,
                                pattern: pattern.clone(),
                                cache_type: cache_type.clone(),
                            });
                        }
                    }
                }

                if tasks.len() % 100 == 0 {
                    progress.set_message(format!("Scanning... found {} items", tasks.len()));
                }
            }
        } else {
            // Non-recursive search (current directory only)
            let entries = fs::read_dir(&self.root_directory).await?;
            let mut entries = entries;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                for (cache_type, pattern) in patterns {
                    if self.matches_pattern(&path, pattern) {
                        tasks.push(CleanTask {
                            path: path.clone(),
                            pattern: pattern.clone(),
                            cache_type: cache_type.clone(),
                        });
                    }
                }
            }
        }

        Ok(tasks)
    }

    fn matches_pattern(&self, path: &Path, pattern: &CachePattern) -> bool {
        let file_name = path.file_name().unwrap_or_default().to_string_lossy();
        let path_str = path.to_string_lossy();

        for pattern_str in &pattern.patterns {
            if pattern_str.contains('*') {
                // Glob pattern
                if let Ok(regex) = glob_to_regex(pattern_str) {
                    if regex.is_match(&file_name) || regex.is_match(&path_str) {
                        return true;
                    }
                }
            } else {
                // Exact match
                if file_name == pattern_str.as_str() || path_str.ends_with(pattern_str) {
                    return true;
                }
            }
        }

        false
    }

    fn process_chunk(
        &self,
        tasks: &[CleanTask],
        total_size: Arc<AtomicU64>,
        total_files: Arc<AtomicU64>,
    ) -> Result<(usize, u64, u64, Vec<String>)> {
        let mut directories_cleaned = 0;
        let mut files_deleted = 0;
        let mut space_freed = 0;
        let mut errors = Vec::new();

        for task in tasks {
            if self.verbose {
                println!(
                    "Processing: {} ({})",
                    task.path.display().to_string().bright_blue(),
                    task.pattern.description.bright_yellow()
                );
            }

            match self.clean_item(task) {
                Ok((files, size)) => {
                    directories_cleaned += 1;
                    files_deleted += files;
                    space_freed += size;
                    total_files.fetch_add(files, Ordering::Relaxed);
                    total_size.fetch_add(size, Ordering::Relaxed);

                    if self.verbose || self.dry_run {
                        println!(
                            "  {} {} ({} files, {})",
                            if self.dry_run { "Would delete:" } else { "Deleted:" },
                            task.path.display().to_string().bright_green(),
                            files.to_string().bright_cyan(),
                            humansize::format_size(size, humansize::BINARY).bright_cyan()
                        );
                    }
                }
                Err(e) => {
                    errors.push(format!("Failed to clean {}: {}", task.path.display(), e));
                }
            }
        }

        Ok((directories_cleaned, files_deleted, space_freed, errors))
    }

    fn clean_item(&self, task: &CleanTask) -> Result<(u64, u64)> {
        if !task.path.exists() {
            return Ok((0, 0));
        }

        let (files, size) = if task.path.is_dir() {
            calculate_directory_size(&task.path)?
        } else {
            let metadata = std::fs::metadata(&task.path)?;
            (1, metadata.len())
        };

        if !self.dry_run {
            if task.path.is_dir() {
                std::fs::remove_dir_all(&task.path)?;
            } else {
                std::fs::remove_file(&task.path)?;
            }
        }

        Ok((files, size))
    }

    async fn clean_docker_caches(&self) -> Result<()> {
        if self.dry_run {
            println!("{}", "Would run Docker cleanup commands:".bright_yellow());
            println!("  docker system prune -af");
            println!("  docker volume prune -f");
            return Ok(());
        }

        // Check if Docker is available
        let output = tokio::process::Command::new("docker")
            .args(&["--version"])
            .output()
            .await;

        if output.is_err() {
            return Err(anyhow::anyhow!("Docker is not available"));
        }

        // Clean Docker system
        let output = tokio::process::Command::new("docker")
            .args(&["system", "prune", "-af"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Docker system prune failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Clean Docker volumes
        let output = tokio::process::Command::new("docker")
            .args(&["volume", "prune", "-f"])
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Docker volume prune failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        if self.verbose {
            println!("{}", "Docker caches cleaned successfully".bright_green());
        }

        Ok(())
    }
}

fn glob_to_regex(pattern: &str) -> Result<Regex> {
    let regex_pattern = pattern
        .replace(".", r"\.")
        .replace("*", ".*")
        .replace("?", ".");
    
    Ok(Regex::new(&format!("^{}$", regex_pattern))?)
} 