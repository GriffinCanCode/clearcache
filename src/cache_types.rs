use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheType {
    Node,
    Rust,
    Go,
    Python,
    Docker,
    General,
}

#[derive(Debug, Clone)]
pub struct CachePattern {
    pub name: String,
    pub patterns: Vec<String>,
    pub description: String,
    pub is_directory: bool,
    pub recursive_safe: bool, // Safe to delete recursively
    pub is_library: bool, // True if this is a library/dependency that requires reinstallation
}

impl CacheType {
    pub fn get_patterns(&self) -> Vec<CachePattern> {
        match self {
            CacheType::Node => vec![
                // Libraries (require reinstallation)
                CachePattern {
                    name: "node_modules".to_string(),
                    patterns: vec!["node_modules".to_string()],
                    description: "Node.js dependencies".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: true,
                },
                // Safe caches (can be removed without reinstallation)
                CachePattern {
                    name: "npm_cache".to_string(),
                    patterns: vec![".npm".to_string()],
                    description: "NPM cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "next_build".to_string(),
                    patterns: vec![".next".to_string()],
                    description: "Next.js build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "nuxt_build".to_string(),
                    patterns: vec![".nuxt".to_string(), ".output".to_string()],
                    description: "Nuxt.js build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "yarn_cache".to_string(),
                    patterns: vec![".yarn/cache".to_string()],
                    description: "Yarn cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "pnpm_cache".to_string(),
                    patterns: vec![".pnpm-store".to_string()],
                    description: "PNPM cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "turbo_cache".to_string(),
                    patterns: vec![".turbo".to_string()],
                    description: "Turbo build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "parcel_cache".to_string(),
                    patterns: vec![".parcel-cache".to_string()],
                    description: "Parcel build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
            ],
            CacheType::Rust => vec![
                // Libraries (require reinstallation)
                CachePattern {
                    name: "cargo_target".to_string(),
                    patterns: vec!["target".to_string()],
                    description: "Cargo build artifacts".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: true,
                },
                // Safe caches (lock files are generally safe to regenerate but be careful)
                CachePattern {
                    name: "cargo_lock".to_string(),
                    patterns: vec!["Cargo.lock".to_string()],
                    description: "Cargo lock file (in some cases)".to_string(),
                    is_directory: false,
                    recursive_safe: false, // Be careful with lock files
                    is_library: false,
                },
            ],
            CacheType::Go => vec![
                // Libraries (require reinstallation)
                CachePattern {
                    name: "go_mod_cache".to_string(),
                    patterns: vec!["pkg/mod".to_string()],
                    description: "Go module cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: true,
                },
                // Safe caches
                CachePattern {
                    name: "go_build_cache".to_string(),
                    patterns: vec!["go-build".to_string()],
                    description: "Go build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
            ],
            CacheType::Python => vec![
                // All Python caches are safe - they don't require reinstallation
                CachePattern {
                    name: "python_cache".to_string(),
                    patterns: vec!["__pycache__".to_string()],
                    description: "Python bytecode cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "python_bytecode".to_string(),
                    patterns: vec!["*.pyc".to_string(), "*.pyo".to_string()],
                    description: "Python bytecode files".to_string(),
                    is_directory: false,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "pytest_cache".to_string(),
                    patterns: vec![".pytest_cache".to_string()],
                    description: "Pytest cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "mypy_cache".to_string(),
                    patterns: vec![".mypy_cache".to_string()],
                    description: "MyPy cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "pip_cache".to_string(),
                    patterns: vec![".pip".to_string()],
                    description: "Pip cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
            ],
            CacheType::Docker => vec![
                // Docker system cache - not exactly a library but requires Docker commands
                CachePattern {
                    name: "docker_system".to_string(),
                    patterns: vec![], // Special case - handled by Docker commands
                    description: "Docker system cache (containers, images, volumes)".to_string(),
                    is_directory: false,
                    recursive_safe: false,
                    is_library: false,
                },
            ],
            CacheType::General => vec![
                // All general caches are safe
                CachePattern {
                    name: "cache_dirs".to_string(),
                    patterns: vec![
                        ".cache".to_string(),
                        "cache".to_string(),
                        "@cache".to_string(),
                    ],
                    description: "General cache directories".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "temp_dirs".to_string(),
                    patterns: vec![
                        ".temp".to_string(),
                        "temp".to_string(),
                        "@temp".to_string(),
                        ".tmp".to_string(),
                        "tmp".to_string(),
                    ],
                    description: "Temporary directories".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "build_dirs".to_string(),
                    patterns: vec![
                        "build".to_string(),
                        "dist".to_string(),
                        "out".to_string(),
                        ".build".to_string(),
                    ],
                    description: "Build output directories".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "log_files".to_string(),
                    patterns: vec![
                        "*.log".to_string(),
                        "logs".to_string(),
                        ".log".to_string(),
                    ],
                    description: "Log files and directories".to_string(),
                    is_directory: false,
                    recursive_safe: true,
                    is_library: false,
                },
                CachePattern {
                    name: "exporter_dirs".to_string(),
                    patterns: vec![
                        ".exporter".to_string(),
                    ],
                    description: "Data exporter cache directories".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                    is_library: false,
                },
            ],
        }
    }

    pub fn get_safe_patterns(&self) -> Vec<CachePattern> {
        self.get_patterns()
            .into_iter()
            .filter(|pattern| !pattern.is_library)
            .collect()
    }

    pub fn get_library_patterns(&self) -> Vec<CachePattern> {
        self.get_patterns()
            .into_iter()
            .filter(|pattern| pattern.is_library)
            .collect()
    }

    pub fn get_all_patterns() -> HashMap<CacheType, Vec<CachePattern>> {
        let mut patterns = HashMap::new();
        let cache_types = vec![
            CacheType::Node,
            CacheType::Rust,
            CacheType::Go,
            CacheType::Python,
            CacheType::Docker,
            CacheType::General,
        ];

        for cache_type in cache_types {
            patterns.insert(cache_type.clone(), cache_type.get_patterns());
        }

        patterns
    }
} 