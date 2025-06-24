use anyhow::Result;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Calculate the total size and file count of a directory
pub fn calculate_directory_size(path: &Path) -> Result<(u64, u64)> {
    let mut total_size = 0;
    let mut file_count = 0;

    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
                file_count += 1;
            }
        }
    }

    Ok((file_count, total_size))
}

/// Check if a directory should be skipped during traversal
pub fn should_skip_directory(path: &Path) -> bool {
    let skip_dirs = [
        ".git",
        ".svn",
        ".hg",
        ".bzr",
        "node_modules/.bin",
        "target/release",
        "target/debug",
    ];

    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        return skip_dirs.iter().any(|&skip| name_str == skip);
    }

    false
}

/// Check if a path is ignored by git
pub fn is_git_ignored(path: &Path) -> bool {
    // Simple check for common git-ignored patterns
    let ignore_patterns = [
        ".DS_Store",
        "Thumbs.db",
        "desktop.ini",
        ".vscode",
        ".idea",
        "*.swp",
        "*.swo",
        "*~",
    ];

    if let Some(name) = path.file_name() {
        let name_str = name.to_string_lossy();
        for pattern in &ignore_patterns {
            if pattern.contains('*') {
                // Simple glob matching
                let pattern_no_star = pattern.replace("*", "");
                if name_str.contains(&pattern_no_star) {
                    return true;
                }
            } else if name_str == *pattern {
                return true;
            }
        }
    }

    false
}

/// Check if a path is safe to delete
pub fn is_safe_to_delete(path: &Path) -> bool {
    // Safety checks to prevent accidental deletion of important directories
    let dangerous_paths = [
        "/",
        "/usr",
        "/bin",
        "/sbin",
        "/etc",
        "/var",
        "/home",
        "/root",
        "/boot",
        "/dev",
        "/proc",
        "/sys",
        "/tmp",
        "/Library",
        "/System",
        "/Applications",
        "/Users",
        "/Volumes",
        "C:\\",
        "C:\\Windows",
        "C:\\Program Files",
        "C:\\Program Files (x86)",
        "C:\\Users",
    ];

    let path_str = path.to_string_lossy();
    
    // Check if it's a dangerous system path
    for dangerous in &dangerous_paths {
        if path_str == *dangerous {
            return false;
        }
    }

    // Check if it's too close to root
    if path.components().count() < 3 {
        return false;
    }

    // Additional safety: don't delete if it contains important files
    if path.is_dir() {
        for entry in std::fs::read_dir(path).unwrap_or_else(|_| {
            return std::fs::read_dir(".").unwrap(); // Fallback to current dir
        }) {
            if let Ok(entry) = entry {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                
                // Check for important files that suggest this isn't a cache directory
                let important_files = [
                    "main.rs",
                    "lib.rs",
                    "index.js",
                    "package.json",
                    "Cargo.toml",
                    "go.mod",
                    "requirements.txt",
                    "setup.py",
                    "Makefile",
                    "README.md",
                    "LICENSE",
                ];

                if important_files.iter().any(|&important| name_str == important) {
                    return false;
                }
            }
        }
    }

    true
}

/// Get the size of a file or directory in a human-readable format
pub fn format_size(size: u64) -> String {
    humansize::format_size(size, humansize::BINARY)
}

/// Create a symbolic link
pub fn create_symlink(original: &Path, link: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(original, link)?;
    }
    
    #[cfg(windows)]
    {
        if original.is_dir() {
            std::os::windows::fs::symlink_dir(original, link)?;
        } else {
            std::os::windows::fs::symlink_file(original, link)?;
        }
    }
    
    Ok(())
}

/// Check if a path exists and is accessible
pub fn is_accessible(path: &Path) -> bool {
    path.exists() && path.metadata().is_ok()
}

/// Get the parent directory of a path
pub fn get_parent_dir(path: &Path) -> Option<PathBuf> {
    path.parent().map(|p| p.to_path_buf())
}

/// Normalize a path (resolve .. and . components)
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {
                // Skip current directory components
            }
            _ => {
                components.push(component);
            }
        }
    }
    
    components.iter().collect()
} 