use crate::cache_types::{CachePattern, CacheType};
use crate::traversal::{CacheTraversal, TraversalConfig};
use crate::utils::calculate_directory_size;
use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

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
    include_libraries: bool,
    no_ignore: bool,
    respect_gitignore: bool,
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
        include_libraries: bool,
        no_ignore: bool,
        respect_gitignore: bool,
    ) -> Self {
        Self {
            root_directory,
            cache_types,
            parallel_threads,
            recursive,
            dry_run,
            verbose,
            include_libraries,
            no_ignore,
            respect_gitignore,
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

        // Collect patterns based on include_libraries flag
        let mut all_patterns = Vec::new();
        for cache_type in &self.cache_types {
            let patterns = if self.include_libraries {
                // Include all patterns (both safe caches and libraries)
                cache_type.get_patterns()
            } else {
                // Only include safe patterns (exclude libraries)
                cache_type.get_safe_patterns()
            };
            
            for pattern in patterns {
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
        progress.set_message("Initializing efficient cache traversal...");

        // Configure traversal based on user preferences
        // By default, we don't respect .gitignore for cache cleaning since many
        // cache directories (target/, node_modules/, etc.) are in .gitignore
        // but we still want to clean them. We still respect .clearcacheignore
        // for user-specific exclusions.
        let config = TraversalConfig {
            max_depth: if self.recursive { 20 } else { 1 },
            follow_links: false, // Don't follow symlinks for safety
            ignore_hidden: false, // We want to find cache dirs that start with .
            respect_gitignore: self.respect_gitignore, // User can opt-in to respect .gitignore
            respect_clearcacheignore: !self.no_ignore,
            parallel: self.parallel_threads > 1,
        };

        // Create traversal engine
        let traversal = CacheTraversal::new(config, patterns.to_vec());
        
        progress.set_message("Scanning directories with optimized traversal...");
        
        // Use the new efficient traversal system
        let found_items = traversal.find_cache_items(&self.root_directory)?;
        
        progress.set_message(format!("Found {} cache items", found_items.len()));

        // Convert FoundCacheItem to CleanTask
        let tasks: Vec<CleanTask> = found_items
            .into_iter()
            .map(|item| CleanTask {
                path: item.path,
                pattern: item.pattern,
                cache_type: item.cache_type,
            })
            .collect();

        Ok(tasks)
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
                let library_indicator = if task.pattern.is_library { " [LIBRARY]" } else { "" };
                println!(
                    "Processing: {} ({}{})",
                    task.path.display().to_string().bright_blue(),
                    task.pattern.description.bright_yellow(),
                    library_indicator.bright_red()
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
                        let library_indicator = if task.pattern.is_library { " [LIBRARY]" } else { "" };
                        println!(
                            "  {} {} ({} files, {}{})",
                            if self.dry_run { "Would delete:" } else { "Deleted:" },
                            task.path.display().to_string().bright_green(),
                            files.to_string().bright_cyan(),
                            humansize::format_size(size, humansize::BINARY).bright_cyan(),
                            library_indicator.bright_red()
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