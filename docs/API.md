# Hoards Library API Reference

This document provides API documentation for using hoards as a Rust library.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
hoards = "0.1"
```

Or from git:

```toml
[dependencies]
hoards = { git = "https://github.com/chrisbataille/hoards" }
```

## Core Types

### `InstallSource`

Enum representing package installation sources.

```rust
use hoards::InstallSource;

pub enum InstallSource {
    Cargo,    // Rust packages via cargo
    Apt,      // Debian/Ubuntu packages
    Snap,     // Snap packages
    Flatpak,  // Flatpak packages
    Npm,      // Node.js packages
    Pip,      // Python packages
    Brew,     // Homebrew packages (macOS/Linux)
    Manual,   // Manually installed
    Unknown,  // Unknown source
}

// String conversion
let source = InstallSource::from("cargo");  // -> InstallSource::Cargo
let s = InstallSource::Cargo.to_string();   // -> "cargo"
```

### `VersionPolicy`

Enum representing update policies for tools.

```rust
use hoards::VersionPolicy;

#[derive(Default)]
pub enum VersionPolicy {
    Latest,   // Accept any version update (major, minor, patch)
    #[default]
    Stable,   // Only minor/patch updates (skip major)
    Pinned,   // Never update
}

// String conversion
let policy = VersionPolicy::from("stable");  // -> VersionPolicy::Stable
let s = VersionPolicy::Stable.to_string();   // -> "stable"
```

### `Tool`

Represents a tracked CLI tool.

```rust
use hoards::{Tool, InstallSource, VersionPolicy};

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
    pub installed_version: Option<String>,
    pub available_version: Option<String>,
    pub version_policy: Option<VersionPolicy>,
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
    .with_version_policy(VersionPolicy::Stable)
    .installed();
```

### `Bundle`

Represents a collection of tools.

```rust
use hoards::{Bundle, VersionPolicy};

pub struct Bundle {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub tools: Vec<String>,
    pub version_policy: Option<VersionPolicy>,
    pub created_at: DateTime<Utc>,
}

// Builder pattern
let bundle = Bundle::new("search-tools", vec![
    "ripgrep".into(),
    "fd".into(),
    "bat".into(),
])
.with_description("Modern search tools")
.with_version_policy(VersionPolicy::Latest);
```

## Database Operations

### `Database`

Main database interface.

```rust
use hoards::Database;
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
use hoards::{Database, Tool, InstallSource};

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
use hoards::{Database, Bundle};

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
use hoards::Database;

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

    // Match a command to a tracked tool (for shell hooks)
    if let Some(tool_name) = db.match_command_to_tool("rg")? {
        println!("Command 'rg' maps to tool: {}", tool_name);
    }

    // Clear usage data
    db.clear_usage()?;

    Ok(())
}
```

### Labels

```rust
use hoards::Database;

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
use hoards::Database;

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
use hoards::{PackageSource, Tool};
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
use hoards::{all_sources, get_source, source_for, InstallSource};

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
use hoards::{is_installed, scan_known_tools, scan_missing_tools, scan_path_tools, KNOWN_TOOLS};
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
use hoards::{validate_package_name, validate_version, SafeCommand};
use hoards::{get_safe_install_command, get_safe_uninstall_command};
use hoards::InstallSource;

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
use hoards::{
    // Workflow commands
    cmd_init, cmd_maintain, cmd_cleanup,

    // Sync command
    cmd_sync,

    // Discover commands
    cmd_discover_list, cmd_discover_search, cmd_discover_categories,
    cmd_discover_labels, cmd_discover_similar, cmd_discover_trending,
    cmd_discover_recommended, cmd_discover_missing,

    // Insights commands
    cmd_insights_overview, cmd_insights_usage, cmd_insights_unused,
    cmd_insights_health, cmd_insights_stats,

    // Usage tracking commands
    cmd_usage_scan, cmd_usage_show, cmd_usage_tool,
    cmd_usage_log, cmd_usage_init, cmd_usage_config, cmd_usage_reset,
    ensure_usage_configured,

    // Completions commands
    cmd_completions_install, cmd_completions_status, cmd_completions_uninstall,

    // Tool management
    cmd_add, cmd_show, cmd_remove,
    cmd_install, cmd_uninstall, cmd_upgrade,

    // Bundle commands
    cmd_bundle_create, cmd_bundle_list, cmd_bundle_show,
    cmd_bundle_install, cmd_bundle_add, cmd_bundle_remove,
    cmd_bundle_delete,

    // Policy commands
    cmd_policy_set, cmd_policy_clear, cmd_policy_show,
    cmd_policy_set_source, cmd_policy_clear_source,
    cmd_policy_set_default, cmd_policy_set_bundle, cmd_policy_clear_bundle,

    // AI commands
    cmd_ai_config_set, cmd_ai_config_show, cmd_ai_config_test,
    cmd_ai_enrich, cmd_ai_extract,

    // GitHub commands (advanced)
    cmd_gh_fetch, cmd_gh_backfill,

    // Misc commands
    cmd_export, cmd_import, cmd_edit,

    // Configuration
    HoardConfig,

    // Version Policy
    VersionPolicy, resolve_policy, policy_source,
    should_update, UpdateDecision, classify_change, VersionChange,

    Database,
};

fn programmatic_usage() -> Result<()> {
    let db = Database::open()?;

    // Run workflow commands
    cmd_maintain(&db, false)?;  // auto=false for interactive

    // Run sync with options
    cmd_sync(&db, true, true, true, true, false)?;  // scan, github, usage, descriptions, dry_run

    // Run insights
    cmd_insights_overview(&db)?;
    cmd_insights_unused(&db)?;

    Ok(())
}
```

## Configuration

### `HoardConfig`

Application configuration loaded from `~/.config/hoards/config.toml`.

```rust
use hoards::{HoardConfig, AiProvider};
use hoards::config::{UsageConfig, UsageMode};

fn config_examples() -> Result<()> {
    // Load config (creates default if missing)
    let config = HoardConfig::load()?;

    // Check AI provider
    println!("AI provider: {}", config.ai.provider);

    // Check usage tracking mode
    match &config.usage.mode {
        Some(UsageMode::Scan) => println!("Using history scan mode"),
        Some(UsageMode::Hook) => println!("Using shell hook mode"),
        None => println!("Usage tracking not configured"),
    }

    // Modify and save
    let mut config = HoardConfig::load()?;
    config.ai.provider = AiProvider::Claude;
    config.save()?;

    Ok(())
}
```

### Config File Format

```toml
# ~/.config/hoards/config.toml

[ai]
provider = "claude"  # claude, gemini, codex, none

[usage]
mode = "hook"        # scan, hook
shell = "fish"       # fish, bash, zsh (for hook mode)

[version_policy]
default = "stable"   # latest, stable, pinned

[version_policy.sources]
cargo = "latest"
apt = "stable"
pip = "pinned"
```

## Version Policy

### Policy Resolution

Policies cascade from most specific to least specific:

```rust
use hoards::{resolve_policy, policy_source, should_update, UpdateDecision};
use hoards::{Tool, Bundle, HoardConfig, VersionPolicy};

fn policy_examples(tool: &Tool, bundles: &[Bundle], config: &HoardConfig) {
    // Resolve effective policy for a tool
    // Checks: tool override → bundle policy → source default → global default
    let policy = resolve_policy(tool, bundles, config);
    println!("Effective policy: {}", policy);

    // Get where the policy comes from
    let source = policy_source(tool, bundles, config);
    println!("Policy from: {}", source);
    // Output: "tool override", "bundle: dev-tools", "cargo default", or "global default"
}
```

### Update Decision

```rust
use hoards::{should_update, UpdateDecision, VersionPolicy};

fn update_examples() {
    // Check if update should be applied
    let decision = should_update(
        Some("1.0.0"),  // current version
        Some("2.0.0"),  // available version
        &VersionPolicy::Stable
    );

    match decision {
        UpdateDecision::Update => println!("Update available and allowed"),
        UpdateDecision::SkipMajor => println!("Major update skipped (stable policy)"),
        UpdateDecision::Pinned => println!("Tool is pinned"),
        UpdateDecision::UpToDate => println!("Already up to date"),
        UpdateDecision::Unknown => println!("Cannot determine (missing version)"),
    }

    // Get icon for display
    println!("Icon: {}", decision.icon());
    // "↑" (Update), "⚠" (SkipMajor), "📌" (Pinned), "" (UpToDate/Unknown)
}
```

### Version Classification

```rust
use hoards::{classify_change, VersionChange};

fn classify_examples() {
    let change = classify_change(Some("1.0.0"), Some("2.0.0"));
    assert_eq!(change, VersionChange::Major);

    let change = classify_change(Some("1.0.0"), Some("1.1.0"));
    assert_eq!(change, VersionChange::Minor);

    let change = classify_change(Some("1.0.0"), Some("1.0.1"));
    assert_eq!(change, VersionChange::Patch);

    // Get label for display
    println!("Change type: {}", change.label());  // "major", "minor", "patch", "update"
}
```

### Database Policy Operations

```rust
use hoards::{Database, VersionPolicy};

fn db_policy_operations(db: &Database) -> Result<()> {
    // Set tool version policy
    db.set_tool_version_policy("ripgrep", Some(&VersionPolicy::Latest))?;

    // Clear tool policy (inherit from bundle/source/global)
    db.set_tool_version_policy("ripgrep", None)?;

    // Set bundle version policy
    db.set_bundle_version_policy("dev-tools", Some(&VersionPolicy::Stable))?;

    // Clear bundle policy
    db.set_bundle_version_policy("dev-tools", None)?;

    // Update tool versions
    db.update_tool_versions("ripgrep", Some("14.0.3"), Some("14.1.0"))?;

    Ok(())
}
```

## Error Handling

All functions return `anyhow::Result<T>` for flexible error handling:

```rust
use anyhow::{Result, Context};
use hoards::Database;

fn with_context() -> Result<()> {
    let db = Database::open()
        .context("Failed to open hoards database")?;

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
