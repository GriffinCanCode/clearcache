# Cache Type System

ClearCache implements a sophisticated pattern-based system for identifying and managing cache artifacts across diverse development ecosystems. The system balances comprehensive coverage with safety through precise pattern matching, extensible architecture, and intelligent separation of safe caches from libraries requiring reinstallation.

## System Architecture

### Pattern-Based Classification with Safety Levels

**Cache Type Abstraction**: Each supported development ecosystem is represented as a distinct cache type with specific patterns, safety rules, and metadata. This abstraction enables targeted cleaning while maintaining ecosystem-specific knowledge and distinguishing between safe temporary caches and libraries requiring reinstallation.

**Two-Tier Cleaning System**: The system operates in two distinct modes:
- **Safe Mode (Default)**: Cleans only temporary caches that regenerate automatically without requiring package installation
- **Library Mode**: Additionally cleans dependencies and libraries that require explicit reinstallation

**Pattern Compilation**: Cache patterns are compiled into optimized matching structures during initialization, eliminating runtime compilation overhead and enabling efficient pattern matching across large directory structures.

**Hierarchical Organization**: Cache types are organized hierarchically, allowing for both broad ecosystem-level operations and fine-grained pattern-specific targeting with safety classification.

### Core Components

**CacheType Enumeration**: Defines supported development ecosystems including Node.js, Rust, Go, Python, Docker, and general cache categories.

**CachePattern Structure**: Encapsulates individual cache patterns with associated metadata including pattern strings, descriptions, directory flags, safety indicators, and library classification.

**Library Classification**: Each pattern includes an `is_library` flag that determines whether the cache represents:
- **Safe Cache** (`is_library: false`): Temporary files that regenerate automatically
- **Library Cache** (`is_library: true`): Dependencies requiring explicit reinstallation

**Pattern Matching Engine**: Optimized pattern matching implementation supporting both exact string matching and glob-style pattern matching with regex compilation.

## Supported Ecosystems with Safety Classification

### Node.js Ecosystem

**Safe Caches** (Default cleaning):
- `.npm`: NPM global cache directory - regenerates automatically
- `.next`: Next.js build artifacts and incremental compilation cache
- `.nuxt`: Nuxt.js build output and server-side rendering cache
- `.output`: Nuxt.js production build output
- `.turbo`: Turborepo build cache and task artifacts
- `.parcel-cache`: Parcel bundler cache and transformed assets
- `.yarn/cache`: Yarn v1 cache storage

**Library Dependencies** (Require `--include-libraries` flag):
- `node_modules`: Primary dependency storage - requires `npm install` or equivalent to restore

**Rationale**: Node.js ecosystem generates substantial cache data through package managers and build tools. Safe caches can be regenerated during the next build or development session, while `node_modules` contains installed packages that require explicit reinstallation. Note that `node_modules` is typically in `.gitignore`, but ClearCache ignores `.gitignore` by default to ensure cache directories are still cleaned.

### Rust Ecosystem

**Library Dependencies** (Require `--include-libraries` flag):
- `target`: Cargo build output including debug, release, and dependency compilation artifacts

**Safe Caches** (Default cleaning):
- `Cargo.lock`: Dependency lock file (with safety restrictions for active projects)

**Safety Considerations**: Rust projects often have valuable incremental compilation data in target directories. The system carefully validates project context before cleaning to preserve active development work. Target directories are classified as libraries since they contain compiled dependencies that take significant time to rebuild. Note that `target/` is typically in `.gitignore`, but ClearCache ignores `.gitignore` by default to ensure cache directories are still cleaned.

### Go Ecosystem

**Library Dependencies** (Require `--include-libraries` flag):
- `pkg/mod`: Go module cache containing downloaded dependencies

**Safe Caches** (Default cleaning):
- `go-build`: Go compiler build cache for faster subsequent compilations

**Module Management**: Go's module system creates predictable cache structures. The build cache regenerates quickly, while the module cache contains downloaded dependencies that would need to be re-downloaded.

### Python Ecosystem

**Safe Caches** (All Python caches are safe - default cleaning):
- `__pycache__`: Python bytecode compilation cache
- `*.pyc`, `*.pyo`: Individual bytecode files with pattern-based matching
- `.pytest_cache`: Pytest test runner cache and coverage data
- `.mypy_cache`: MyPy type checker cache and incremental analysis
- `.pip`: Pip package manager cache directory

**Pattern Complexity**: Python cache patterns require careful handling due to the distributed nature of bytecode files and varying cache directory structures across different tools. All Python caches regenerate automatically and don't require package reinstallation.

### Docker Ecosystem

**System Integration**: Docker cache management requires system-level operations rather than filesystem pattern matching, utilizing Docker API calls for safe and comprehensive cache removal.

**Safe Caches** (Default cleaning):
- Container images and layers (via Docker commands)
- Build cache and intermediate layers
- Volume data and network configurations
- System-wide Docker storage

**Classification**: Docker caches are classified as safe since they don't require package manager operations to restore - images can be pulled again as needed.

### General Cache Patterns

**Safe Caches** (Default cleaning):
- `.cache`, `cache`, `@cache`: Generic cache directories used by various tools
- `.temp`, `temp`, `@temp`, `.tmp`, `tmp`: Temporary file storage
- `build`, `dist`, `out`, `.build`: Build output directories
- `*.log`, `logs`, `.log`: Log files and logging directories
- `.exporter`: Exporter cache directories used by Exporter tool

**Scope**: These patterns handle cache artifacts from tools and frameworks not covered by ecosystem-specific patterns, providing comprehensive coverage across diverse development environments. All general patterns are classified as safe caches.

## Safety Classification System

### Safe Cache Criteria

**Automatic Regeneration**: Files that are automatically recreated during normal development workflow without explicit user action.

**No Installation Required**: Caches that don't require package managers or installation commands to restore.

**Low Recreation Cost**: Files that can be regenerated quickly without significant time or bandwidth investment.

**Examples**: Bytecode caches, build artifacts, temporary files, compiler caches

### Library Cache Criteria

**Manual Installation Required**: Dependencies that require explicit package manager commands to restore.

**High Recreation Cost**: Files that require significant time, bandwidth, or computation to recreate.

**Dependency Storage**: Directories containing installed packages or compiled dependencies.

**Examples**: `node_modules`, `target` directories, module caches

## Usage Modes

### Safe Mode (Default)

**Command**: `clearcache` (no additional flags needed)

**Behavior**: Cleans only patterns marked with `is_library: false`

**Use Cases**:
- Daily development cleanup
- CI/CD pipeline optimization
- Quick space recovery without reinstallation
- Safe automated cleaning

**Benefits**:
- No reinstallation required
- Fast execution
- Safe for automated scripts
- Preserves development environment setup

### Library Mode

**Command**: `clearcache --include-libraries` or `clearcache -l`

**Behavior**: Cleans all patterns (both `is_library: false` and `is_library: true`)

**Use Cases**:
- Deep cleaning before project archival
- Resolving dependency conflicts
- Maximum space recovery
- Environment reset

**Considerations**:
- Requires reinstallation after cleaning
- Longer execution time for rebuilds
- Should be used intentionally
- May require network access for re-downloading

## Pattern Matching Implementation

### Matching Algorithms

**Exact String Matching**: Direct string comparison for well-defined cache directory names, providing optimal performance for common cases.

**Glob Pattern Matching**: Support for wildcard patterns enabling flexible matching of file extensions and naming patterns while maintaining performance through compiled regex.

**Context-Aware Matching**: Pattern matching considers directory context, project type, and safety classification to reduce false positives and improve accuracy.

### Performance Optimizations

**Pattern Compilation**: Regular expressions and glob patterns are compiled once during initialization and reused across all matching operations.

**Early Termination**: Pattern matching uses short-circuit evaluation to minimize unnecessary computation when patterns clearly do not match.

**Cache-Friendly Ordering**: Patterns are ordered by likelihood of matching and safety classification to optimize CPU cache utilization and reduce average matching time.

**Safety-First Filtering**: Library patterns are filtered out in safe mode before expensive filesystem operations, improving performance for default usage.

### Safety Integration

**Conservative Matching**: Pattern matching errs on the side of caution, requiring high confidence before identifying directories as cache artifacts.

**Context Validation**: Matched patterns undergo additional validation to ensure they represent genuine cache data rather than user-created directories with similar names.

**Multi-Criteria Validation**: Multiple pattern criteria must align before a directory is considered for deletion, reducing false positive rates.

**Library Classification Validation**: Additional checks ensure library patterns are only processed when explicitly requested.

## Extensibility Framework

### Adding New Cache Types

**Pattern Definition**: New cache types can be added by defining pattern structures with appropriate matching criteria, safety metadata, and library classification.

**Safety Classification**: New patterns must specify whether they represent safe caches or libraries requiring reinstallation.

**Integration Points**: The system provides clear integration points for new cache types without requiring modifications to core processing logic.

**Validation Requirements**: New patterns must include appropriate safety validation, library classification, and testing to ensure reliable operation.

### Custom Pattern Extensions

**User-Defined Patterns**: Advanced users can define custom patterns for specialized development environments or proprietary tools with appropriate safety classification.

**Configuration Integration**: Custom patterns integrate with the existing configuration system, safety validation framework, and library classification system.

**Override Mechanisms**: Users can override or disable built-in patterns for specialized environments while maintaining safety guarantees and proper classification.

### Ecosystem Evolution

**Version Compatibility**: The pattern system accommodates evolution in development tools and changing cache storage conventions while maintaining safety classification accuracy.

**Backward Compatibility**: New pattern additions maintain compatibility with existing cache structures while supporting emerging conventions and proper safety classification.

**Community Contributions**: Framework supports community contributions of new patterns and ecosystem support with mandatory safety classification.

## Advanced Features

### Intelligent Mode Selection

**Context-Aware Recommendations**: System can analyze project structure to recommend appropriate cleaning mode based on detected dependencies and cache types.

**Safety Warnings**: Clear warnings when library mode would affect critical dependencies or require significant reinstallation time.

**Usage Analytics**: Optional tracking of cleaning patterns to optimize default behavior and safety recommendations.

### Progressive Cleaning

**Staged Cleaning**: Option to clean safe caches first, then prompt for library cleaning with impact assessment.

**Impact Assessment**: Pre-cleaning analysis showing reinstallation requirements and estimated time costs.

**Selective Library Cleaning**: Fine-grained control over which library types to include in cleaning operations.

## Configuration and Customization

### Mode Configuration

**Default Mode Setting**: Users can configure their preferred default mode (safe vs library) for their environment.

**Project-Specific Settings**: Support for project-level configuration files specifying preferred cleaning modes and patterns.

**Environment Variables**: Recognition of environment-specific settings for automated deployment scenarios.

### Safety Overrides

**Custom Library Classification**: Advanced users can reclassify patterns between safe and library categories for specialized workflows.

**Pattern Exclusions**: Ability to exclude specific patterns from either safe or library cleaning modes.

**Custom Safety Rules**: Extension points for additional safety validation specific to organizational requirements.

## Benefits of Dual-Mode System

**Developer Safety**: Default safe mode prevents accidental removal of dependencies requiring reinstallation.

**Flexibility**: Library mode provides comprehensive cleaning when needed for deep maintenance or troubleshooting.

**Performance Optimization**: Safe mode enables fast, frequent cleaning without workflow disruption.

**Automation Friendly**: Safe mode is suitable for automated scripts and CI/CD integration without risk.

**Educational Value**: Clear distinction helps developers understand their project's cache architecture and dependencies.

**Resource Management**: Intelligent classification enables better disk space management strategies.

The dual-mode cache type system represents a significant advancement in development environment maintenance, providing both safety and comprehensive cleaning capabilities through intelligent pattern classification and user-controlled operation modes. 