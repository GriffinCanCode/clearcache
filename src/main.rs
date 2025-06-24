use clap::{Arg, Command};
use colored::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

mod cache_cleaner;
mod cache_types;
mod utils;

use cache_cleaner::CacheCleaner;
use cache_types::CacheType;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = Command::new("clearcache")
        .version("0.1.0")
        .author("Cache Cleaner")
        .about("Extremely efficient cache clearing system for development directories")
        .arg(
            Arg::new("directory")
                .help("Directory to clean (default: current directory)")
                .value_name("DIR")
                .index(1),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .short('n')
                .help("Show what would be deleted without actually deleting")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("recursive")
                .long("recursive")
                .short('r')
                .help("Recursively clean all subdirectories")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("types")
                .long("types")
                .short('t')
                .help("Comma-separated list of cache types to clean (node,rust,go,python,docker,all)")
                .value_name("TYPES")
                .default_value("all"),
        )
        .arg(
            Arg::new("parallel")
                .long("parallel")
                .short('p')
                .help("Number of parallel threads (default: CPU count)")
                .value_name("NUM"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("force")
                .long("force")
                .short('f')
                .help("Force deletion without confirmation")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let directory = matches
        .get_one::<String>("directory")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());

    let dry_run = matches.get_flag("dry-run");
    let recursive = matches.get_flag("recursive");
    let verbose = matches.get_flag("verbose");
    let _force = matches.get_flag("force");

    let cache_types = parse_cache_types(matches.get_one::<String>("types").unwrap())?;

    let parallel_threads = matches
        .get_one::<String>("parallel")
        .map(|s| s.parse::<usize>().unwrap_or(num_cpus::get()))
        .unwrap_or(num_cpus::get());

    println!(
        "{}",
        "🧹 ClearCache - Extremely Efficient Cache Cleaner".bright_cyan().bold()
    );
    println!("Directory: {}", directory.display().to_string().bright_yellow());
    println!("Cache types: {}", format_cache_types(&cache_types).bright_green());
    println!("Threads: {}", parallel_threads.to_string().bright_blue());
    
    if dry_run {
        println!("{}", "🔍 DRY RUN MODE - No files will be deleted".bright_yellow().bold());
    }

    let cleaner = CacheCleaner::new(
        directory,
        cache_types,
        parallel_threads,
        recursive,
        dry_run,
        verbose,
    );

    let total_size = Arc::new(AtomicU64::new(0));
    let total_files = Arc::new(AtomicU64::new(0));

    let result = cleaner.clean(total_size.clone(), total_files.clone()).await?;

    println!("\n{}", "📊 Summary".bright_cyan().bold());
    println!("Files processed: {}", total_files.load(Ordering::Relaxed).to_string().bright_green());
    println!("Space freed: {}", humansize::format_size(total_size.load(Ordering::Relaxed), humansize::BINARY).bright_green());
    println!("Directories cleaned: {}", result.directories_cleaned.to_string().bright_green());
    
    if result.errors.is_empty() {
        println!("{}", "✅ All operations completed successfully!".bright_green().bold());
    } else {
        println!("{}", "⚠️  Some errors occurred:".bright_yellow().bold());
        for error in &result.errors {
            println!("  • {}", error.bright_red());
        }
    }

    Ok(())
}

fn parse_cache_types(types_str: &str) -> anyhow::Result<Vec<CacheType>> {
    if types_str == "all" {
        return Ok(vec![
            CacheType::Node,
            CacheType::Rust,
            CacheType::Go,
            CacheType::Python,
            CacheType::Docker,
            CacheType::General,
        ]);
    }

    let mut types = Vec::new();
    for type_str in types_str.split(',') {
        match type_str.trim().to_lowercase().as_str() {
            "node" | "nodejs" | "npm" | "yarn" | "pnpm" => types.push(CacheType::Node),
            "rust" | "cargo" => types.push(CacheType::Rust),
            "go" | "golang" => types.push(CacheType::Go),
            "python" | "py" | "pip" => types.push(CacheType::Python),
            "docker" => types.push(CacheType::Docker),
            "general" | "cache" => types.push(CacheType::General),
            _ => return Err(anyhow::anyhow!("Unknown cache type: {}", type_str)),
        }
    }

    Ok(types)
}

fn format_cache_types(types: &[CacheType]) -> String {
    types
        .iter()
        .map(|t| format!("{:?}", t))
        .collect::<Vec<_>>()
        .join(", ")
} 