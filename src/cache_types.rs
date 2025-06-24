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
}

impl CacheType {
    pub fn get_patterns(&self) -> Vec<CachePattern> {
        match self {
            CacheType::Node => vec![
                CachePattern {
                    name: "node_modules".to_string(),
                    patterns: vec!["node_modules".to_string()],
                    description: "Node.js dependencies".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "npm_cache".to_string(),
                    patterns: vec![".npm".to_string()],
                    description: "NPM cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "next_build".to_string(),
                    patterns: vec![".next".to_string()],
                    description: "Next.js build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "nuxt_build".to_string(),
                    patterns: vec![".nuxt".to_string(), ".output".to_string()],
                    description: "Nuxt.js build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "yarn_cache".to_string(),
                    patterns: vec![".yarn/cache".to_string()],
                    description: "Yarn cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "pnpm_cache".to_string(),
                    patterns: vec![".pnpm-store".to_string()],
                    description: "PNPM cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "turbo_cache".to_string(),
                    patterns: vec![".turbo".to_string()],
                    description: "Turbo build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "parcel_cache".to_string(),
                    patterns: vec![".parcel-cache".to_string()],
                    description: "Parcel build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
            ],
            CacheType::Rust => vec![
                CachePattern {
                    name: "cargo_target".to_string(),
                    patterns: vec!["target".to_string()],
                    description: "Cargo build artifacts".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "cargo_lock".to_string(),
                    patterns: vec!["Cargo.lock".to_string()],
                    description: "Cargo lock file (in some cases)".to_string(),
                    is_directory: false,
                    recursive_safe: false, // Be careful with lock files
                },
            ],
            CacheType::Go => vec![
                CachePattern {
                    name: "go_build_cache".to_string(),
                    patterns: vec!["go-build".to_string()],
                    description: "Go build cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "go_mod_cache".to_string(),
                    patterns: vec!["pkg/mod".to_string()],
                    description: "Go module cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
            ],
            CacheType::Python => vec![
                CachePattern {
                    name: "python_cache".to_string(),
                    patterns: vec!["__pycache__".to_string()],
                    description: "Python bytecode cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "python_bytecode".to_string(),
                    patterns: vec!["*.pyc".to_string(), "*.pyo".to_string()],
                    description: "Python bytecode files".to_string(),
                    is_directory: false,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "pytest_cache".to_string(),
                    patterns: vec![".pytest_cache".to_string()],
                    description: "Pytest cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "mypy_cache".to_string(),
                    patterns: vec![".mypy_cache".to_string()],
                    description: "MyPy cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
                CachePattern {
                    name: "pip_cache".to_string(),
                    patterns: vec![".pip".to_string()],
                    description: "Pip cache".to_string(),
                    is_directory: true,
                    recursive_safe: true,
                },
            ],
            CacheType::Docker => vec![
                // Note: Docker cache cleaning requires Docker commands, not file system operations
                CachePattern {
                    name: "docker_system".to_string(),
                    patterns: vec![], // Special case - handled by Docker commands
                    description: "Docker system cache (containers, images, volumes)".to_string(),
                    is_directory: false,
                    recursive_safe: false,
                },
            ],
            CacheType::General => vec![
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
                },
            ],
        }
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