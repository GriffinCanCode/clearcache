use crate::cache_types::{CachePattern, CacheType};
use anyhow::Result;
use ignore::{WalkBuilder, WalkState};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct TraversalConfig {
    pub max_depth: usize,
    pub follow_links: bool,
    pub ignore_hidden: bool,
    pub respect_gitignore: bool,
    pub respect_clearcacheignore: bool,
    pub parallel: bool,
}

impl Default for TraversalConfig {
    fn default() -> Self {
        Self {
            max_depth: 20,
            follow_links: false,
            ignore_hidden: true,
            respect_gitignore: true,
            respect_clearcacheignore: true,
            parallel: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FoundCacheItem {
    pub path: PathBuf,
    pub pattern: CachePattern,
    pub cache_type: CacheType,
    pub size: u64,
    pub is_directory: bool,
}

pub struct CacheTraversal {
    config: TraversalConfig,
    patterns: Vec<(CacheType, CachePattern)>,
}

impl CacheTraversal {
    pub fn new(config: TraversalConfig, patterns: Vec<(CacheType, CachePattern)>) -> Self {
        Self { config, patterns }
    }

    /// Find all cache items using the most efficient traversal method
    pub fn find_cache_items<P: AsRef<Path>>(&self, root: P) -> Result<Vec<FoundCacheItem>> {
        let root = root.as_ref();
        
        if self.config.parallel && self.config.respect_clearcacheignore {
            // Use ignore crate for parallel traversal with .clearcacheignore support
            self.find_with_ignore_parallel(root)
        } else if self.config.respect_clearcacheignore {
            // Use ignore crate for sequential traversal with .clearcacheignore support  
            self.find_with_ignore_sequential(root)
        } else {
            // Use walkdir for maximum performance when ignores aren't needed
            self.find_with_walkdir(root)
        }
    }

    /// Ultra-fast traversal using walkdir (no .clearcacheignore support)
    fn find_with_walkdir<P: AsRef<Path>>(&self, root: P) -> Result<Vec<FoundCacheItem>> {
        let mut found_items = Vec::new();
        let mut visited = HashSet::new();

        let walker = WalkDir::new(root)
            .max_depth(self.config.max_depth)
            .follow_links(self.config.follow_links)
            .into_iter()
            .filter_entry(|e| {
                if self.config.ignore_hidden {
                    !is_hidden(e.path())
                } else {
                    true
                }
            });

        for entry in walker.filter_map(|e| e.ok()) {
            let path = entry.path();
            
            // Skip if we've already processed this path (handles symlink loops)
            let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            if !visited.insert(canonical.clone()) {
                continue;
            }

            // Check against all patterns
            for (cache_type, pattern) in &self.patterns {
                if self.matches_pattern(path, pattern) {
                    let metadata = entry.metadata().ok();
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                    let is_directory = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);

                    found_items.push(FoundCacheItem {
                        path: canonical.clone(),
                        pattern: pattern.clone(),
                        cache_type: cache_type.clone(),
                        size,
                        is_directory,
                    });
                    break; // Only match first pattern to avoid duplicates
                }
            }
        }

        Ok(found_items)
    }

    /// Parallel traversal with .clearcacheignore support using ignore crate
    fn find_with_ignore_parallel<P: AsRef<Path>>(&self, root: P) -> Result<Vec<FoundCacheItem>> {
        let found_items = Arc::new(std::sync::Mutex::new(Vec::new()));
        let patterns = Arc::new(self.patterns.clone());

        let walker = WalkBuilder::new(root)
            .max_depth(Some(self.config.max_depth))
            .follow_links(self.config.follow_links)
            .hidden(!self.config.ignore_hidden)
            .git_ignore(self.config.respect_gitignore)
            .add_custom_ignore_filename(".clearcacheignore")
            .build_parallel();

        walker.run(|| {
            let found_items = Arc::clone(&found_items);
            let patterns = Arc::clone(&patterns);
            
            Box::new(move |result| {
                if let Ok(entry) = result {
                    let path = entry.path();
                    
                    // Check against all patterns
                    for (cache_type, pattern) in patterns.iter() {
                        if matches_pattern_static(path, pattern) {
                            let metadata = entry.metadata().ok();
                            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                            let is_directory = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);

                            let item = FoundCacheItem {
                                path: path.to_path_buf(),
                                pattern: pattern.clone(),
                                cache_type: cache_type.clone(),
                                size,
                                is_directory,
                            };

                            if let Ok(mut items) = found_items.lock() {
                                items.push(item);
                            }
                            break; // Only match first pattern to avoid duplicates
                        }
                    }
                }
                WalkState::Continue
            })
        });

        let items = found_items.lock().unwrap().clone();
        Ok(items)
    }

    /// Sequential traversal with .clearcacheignore support using ignore crate
    fn find_with_ignore_sequential<P: AsRef<Path>>(&self, root: P) -> Result<Vec<FoundCacheItem>> {
        let mut found_items = Vec::new();

        let walker = WalkBuilder::new(root)
            .max_depth(Some(self.config.max_depth))
            .follow_links(self.config.follow_links)
            .hidden(!self.config.ignore_hidden)
            .git_ignore(self.config.respect_gitignore)
            .add_custom_ignore_filename(".clearcacheignore")
            .build();

        for result in walker {
            if let Ok(entry) = result {
                let path = entry.path();
                
                // Check against all patterns
                for (cache_type, pattern) in &self.patterns {
                    if matches_pattern_static(path, pattern) {
                        let metadata = entry.metadata().ok();
                        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                        let is_directory = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);

                        found_items.push(FoundCacheItem {
                            path: path.to_path_buf(),
                            pattern: pattern.clone(),
                            cache_type: cache_type.clone(),
                            size,
                            is_directory,
                        });
                        break; // Only match first pattern to avoid duplicates
                    }
                }
            }
        }

        Ok(found_items)
    }

    /// Check if a path matches a cache pattern
    fn matches_pattern(&self, path: &Path, pattern: &CachePattern) -> bool {
        matches_pattern_static(path, pattern)
    }
}

/// Static function to check if a path matches a cache pattern (for use in closures)
fn matches_pattern_static(path: &Path, pattern: &CachePattern) -> bool {
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();

    for pattern_str in &pattern.patterns {
        if pattern_str.contains('*') {
            // Glob pattern
            if let Ok(glob_pattern) = glob::Pattern::new(pattern_str) {
                if glob_pattern.matches(&file_name) {
                    return true;
                }
            }
        } else {
            // Exact match
            if file_name == pattern_str.as_str() {
                return true;
            }
        }
    }

    false
}

/// Check if a path is hidden (starts with .)
fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

/// Create a default .clearcacheignore file content
pub fn create_default_clearcacheignore() -> String {
    r#"# ClearCache ignore patterns
# This file uses the same syntax as .gitignore
# Patterns here will be excluded from cache cleaning

# Version control directories
.git/
.svn/
.hg/
.bzr/

# IDE and editor directories  
.vscode/
.idea/
*.swp
*.swo
*~

# OS generated files
.DS_Store
.DS_Store?
._*
.Spotlight-V100
.Trashes
ehthumbs.db
Thumbs.db

# Important project files
package.json
Cargo.toml
go.mod
requirements.txt
setup.py
Makefile
CMakeLists.txt

# Documentation
README*
LICENSE*
CHANGELOG*
CONTRIBUTING*
docs/
doc/

# Source code (be careful with these)
src/
lib/
include/

# Configuration files
config/
conf/
settings/
"#.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_cache_traversal_basic() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test structure
        fs::create_dir_all(root.join("__pycache__")).unwrap();
        fs::create_dir_all(root.join(".exporter")).unwrap();
        fs::create_dir_all(root.join("normal_dir")).unwrap();
        
        fs::write(root.join("__pycache__/test.pyc"), "test").unwrap();
        fs::write(root.join(".exporter/data.json"), "data").unwrap();
        fs::write(root.join("normal_dir/file.txt"), "content").unwrap();

        // Create patterns
        let patterns = vec![
            (CacheType::Python, CachePattern {
                name: "python_cache".to_string(),
                patterns: vec!["__pycache__".to_string()],
                description: "Python cache".to_string(),
                is_directory: true,
                recursive_safe: true,
                is_library: false,
            }),
            (CacheType::General, CachePattern {
                name: "exporter_dirs".to_string(),
                patterns: vec![".exporter".to_string()],
                description: "Exporter cache".to_string(),
                is_directory: true,
                recursive_safe: true,
                is_library: false,
            }),
        ];

        let config = TraversalConfig::default();
        let traversal = CacheTraversal::new(config, patterns);
        let results = traversal.find_cache_items(root).unwrap();

        assert_eq!(results.len(), 2);
        
        let paths: HashSet<_> = results.iter().map(|r| r.path.file_name().unwrap()).collect();
        assert!(paths.contains(&std::ffi::OsStr::new("__pycache__")));
        assert!(paths.contains(&std::ffi::OsStr::new(".exporter")));
    }

    #[test]
    fn test_clearcacheignore_content() {
        let content = create_default_clearcacheignore();
        assert!(content.contains(".git/"));
        assert!(content.contains("package.json"));
        assert!(content.contains("README*"));
    }
} 