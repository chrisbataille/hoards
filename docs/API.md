# Hoard Library API Reference

This document provides API documentation for using hoard as a Rust library.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hoard = { path = "../hoard" }
```

## Core Types

### `InstallSource`

Enum representing package installation sources.

```rust
use hoard::InstallSource;

pub enum InstallSource {
    Cargo,   // Rust packages via cargo
    Apt,     // Debian/Ubuntu packages
    Snap,    // Snap packages
    Npm,     // Node.js packages
    Pip,     // Python packages
    Brew,    // Homebrew packages (macOS/Linux)
    Manual,  // Manually installed
    Unknown, // Unknown source
}

// String conversion
let source = InstallSource::from("cargo");  // -> InstallSource::Cargo
let s = InstallSource::Cargo.to_string();   // -> "cargo"
```

### `Tool`

Represents a tracked CLI tool.

```rust
use hoard::{Tool, InstallSource};

pub struct Tool {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub source: InstallSource,
    pub install_command: Option<String>,
    pub binary_name: Option<String>,
    pub is_installed: bool,
    pub is_favorite: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Builder pattern
let tool = Tool::new("ripgrep")
    .with_source(InstallSource::Cargo)
    .with_description("Fast grep replacement")
    .with_category("search")
    .with_binary("rg")
    .with_install_command("cargo install ripgrep")
    .installed();
```

### `Bundle`

Represents a collection of tools.

```rust
use hoard::Bundle;

pub struct Bundle {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub tools: Vec<String>,
    pub created_at: DateTime<Utc>,
}

// Builder pattern
let bundle = Bundle::new("search-tools", vec![
    "ripgrep".into(),
    "fd".into(),
    "bat".into(),
]).with_description("Modern search tools");
```

## Database Operations

### `Database`

Main database interface.

```rust
use hoard::Database;
use anyhow::Result;

fn main() -> Result<()> {
    // Open or create database
    let db = Database::open()?;

    // Get database path
    let path = Database::db_path()?;
    println!("Database at: {}", path.display());

    Ok(())
}
```

### Tool CRUD

```rust
use hoard::{Database, Tool, InstallSource};

fn tool_operations(db: &Database) -> Result<()> {
    // Insert a tool
    let tool = Tool::new("ripgrep")
        .with_source(InstallSource::Cargo)
        .with_description("Fast grep")
        .installed();
    db.insert_tool(&tool)?;

    // Get by name
    if let Some(tool) = db.get_tool_by_name("ripgrep")? {
        println!("Found: {}", tool.name);
    }

    // List tools (with filters)
    let all = db.list_tools(false, None)?;
    let installed = db.list_tools(true, None)?;
    let search_tools = db.list_tools(false, Some("search"))?;

    // Search
    let results = db.search_tools("grep")?;

    // Update
    let mut tool = db.get_tool_by_name("ripgrep")?.unwrap();
    tool.description = Some("Updated description".into());
    db.update_tool(&tool)?;

    // Update specific fields
    db.update_tool_description("ripgrep", "New description")?;
    db.update_tool_category("ripgrep", "search")?;
    db.set_tool_installed("ripgrep", true)?;

    // Delete
    db.delete_tool("ripgrep")?;

    Ok(())
}
```

### Bundle Operations

```rust
use hoard::{Database, Bundle};

fn bundle_operations(db: &Database) -> Result<()> {
    // Create bundle
    let bundle = Bundle::new("dev-tools", vec!["cargo".into(), "rustfmt".into()])
        .with_description("Rust development tools");
    db.insert_bundle(&bundle)?;

    // List bundles
    let bundles = db.list_bundles()?;

    // Get by name
    if let Some(bundle) = db.get_bundle_by_name("dev-tools")? {
        println!("Bundle has {} tools", bundle.tools.len());
    }

    // Modify tools
    db.add_to_bundle("dev-tools", &["clippy".into()])?;
    db.remove_from_bundle("dev-tools", &["rustfmt".into()])?;

    // Delete
    db.delete_bundle("dev-tools")?;

    Ok(())
}
```

### Usage Tracking

```rust
use hoard::Database;

fn usage_operations(db: &Database) -> Result<()> {
    // Record usage
    db.record_usage("ripgrep", 42, None)?;

    // Get usage for a tool
    if let Some(usage) = db.get_usage("ripgrep")? {
        println!("Used {} times", usage.use_count);
    }

    // Get all usage (sorted by count descending)
    let all_usage = db.get_all_usage()?;

    // Get unused tools
    let unused = db.get_unused_tools()?;

    // Clear usage data
    db.clear_usage()?;

    Ok(())
}
```

### Labels

```rust
use hoard::Database;

fn label_operations(db: &Database) -> Result<()> {
    // Add labels
    db.add_labels("ripgrep", &["search".into(), "rust".into()])?;

    // Get labels for tool
    let labels = db.get_labels("ripgrep")?;

    // List tools by label
    let tools = db.list_tools_by_label("rust")?;

    // Get all labels with counts
    let label_counts = db.get_label_counts()?;
    for (label, count) in label_counts {
        println!("{}: {} tools", label, count);
    }

    // Clear labels
    db.clear_labels("ripgrep")?;

    Ok(())
}
```

### Statistics

```rust
use hoard::Database;

fn stats(db: &Database) -> Result<()> {
    // Get overall stats
    let (total, installed, favorites) = db.get_stats()?;
    println!("Total: {}, Installed: {}, Favorites: {}", total, installed, favorites);

    // Get categories
    let categories = db.get_categories()?;

    // Get category counts (efficient single query)
    let category_counts = db.get_category_counts()?;

    Ok(())
}
```

## Package Sources

### `PackageSource` Trait

```rust
use hoard::{PackageSource, Tool};
use anyhow::Result;

pub trait PackageSource {
    /// Source name (e.g., "cargo", "pip")
    fn name(&self) -> &'static str;

    /// Scan for installed packages
    fn scan(&self) -> Result<Vec<Tool>>;

    /// Get install command for a package
    fn install_command(&self, package: &str) -> String;

    /// Get uninstall command for a package
    fn uninstall_command(&self, package: &str) -> String;

    /// Fetch description from registry
    fn fetch_description(&self, package: &str) -> Option<String>;

    /// Whether this source supports update checking
    fn supports_updates(&self) -> bool;

    /// Check for newer version
    fn check_update(&self, package: &str) -> Result<Option<String>>;
}
```

### Using Sources

```rust
use hoard::{all_sources, get_source, source_for, InstallSource};

fn source_examples() {
    // Get all available sources
    let sources = all_sources();
    for source in &sources {
        println!("{}: supports_updates={}", source.name(), source.supports_updates());
    }

    // Get source by name
    if let Some(cargo) = get_source("cargo") {
        let tools = cargo.scan().unwrap();
        println!("Found {} cargo tools", tools.len());
    }

    // Get source for InstallSource enum
    if let Some(source) = source_for(&InstallSource::Pip) {
        let cmd = source.install_command("requests");
        println!("Install: {}", cmd);
    }
}
```

## Scanner Utilities

```rust
use hoard::{is_installed, scan_known_tools, scan_missing_tools, scan_path_tools, KNOWN_TOOLS};
use std::collections::HashSet;

fn scanner_examples() {
    // Check if a binary is installed
    if is_installed("rg") {
        println!("ripgrep is installed");
    }

    // Scan for known tools
    let known = scan_known_tools();
    println!("Found {} known tools", known.len());

    // Find missing recommended tools
    let missing = scan_missing_tools();
    for tool in missing {
        println!("Missing: {} - {}", tool.name, tool.description.unwrap_or_default());
    }

    // Scan PATH for untracked tools
    let tracked: HashSet<String> = HashSet::new();
    let path_tools = scan_path_tools(&tracked).unwrap();

    // Access curated tool list
    println!("Known tools database has {} entries", KNOWN_TOOLS.len());
}
```

## Safe Command Execution

```rust
use hoard::{validate_package_name, validate_version, SafeCommand};
use hoard::{get_safe_install_command, get_safe_uninstall_command};
use hoard::InstallSource;

fn safe_execution() -> Result<()> {
    // Validate user input
    validate_package_name("ripgrep")?;  // Ok
    validate_package_name("rg; rm -rf /")?;  // Error: invalid characters

    validate_version("1.0.0")?;  // Ok
    validate_version("1.0.0 && echo pwned")?;  // Error

    // Get safe install command
    let cmd = get_safe_install_command(&InstallSource::Cargo, "ripgrep", None)?;
    println!("Command: {:?}", cmd);

    // Get safe uninstall command
    let cmd = get_safe_uninstall_command(&InstallSource::Cargo, "ripgrep")?;
    cmd.execute()?;

    Ok(())
}
```

## Command Functions

All command implementations are exported for programmatic use:

```rust
use hoard::{
    // Install commands
    cmd_install, cmd_uninstall, cmd_upgrade,

    // Bundle commands
    cmd_bundle_create, cmd_bundle_list, cmd_bundle_show,
    cmd_bundle_install, cmd_bundle_add, cmd_bundle_remove,
    cmd_bundle_delete, cmd_bundle_update,

    // AI commands
    cmd_ai_set, cmd_ai_show, cmd_ai_test,
    cmd_ai_categorize, cmd_ai_describe, cmd_ai_suggest_bundle,

    // GitHub commands
    cmd_gh_sync, cmd_gh_fetch, cmd_gh_search,
    cmd_gh_info, cmd_gh_rate_limit, cmd_gh_backfill,

    // Usage commands
    cmd_usage_scan, cmd_usage_show, cmd_usage_tool,
    cmd_labels, cmd_unused, cmd_recommend,

    // Misc commands
    cmd_export, cmd_import, cmd_doctor, cmd_edit,

    Database,
};

fn programmatic_usage() -> Result<()> {
    let db = Database::open()?;

    // Run commands programmatically
    cmd_usage_scan(&db, false, false)?;  // scan, no dry-run, no reset
    cmd_unused(&db)?;
    cmd_recommend(&db, 5)?;

    Ok(())
}
```

## Error Handling

All functions return `anyhow::Result<T>` for flexible error handling:

```rust
use anyhow::{Result, Context};
use hoard::Database;

fn with_context() -> Result<()> {
    let db = Database::open()
        .context("Failed to open hoard database")?;

    let tool = db.get_tool_by_name("ripgrep")
        .context("Database query failed")?
        .context("Tool not found")?;

    Ok(())
}
```

## Feature Flags

Currently no optional features. All functionality is included by default.

## Thread Safety

- `Database` wraps a `rusqlite::Connection` which is `Send` but not `Sync`
- For multi-threaded access, create separate `Database` instances per thread
- Shell history parsing uses `std::thread::scope` for parallel processing
