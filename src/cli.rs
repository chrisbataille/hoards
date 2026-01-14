use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hoard")]
#[command(about = "A tool management system with SQLite database and AI-assisted discovery")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a tool to the database
    Add {
        /// Tool name
        name: String,

        /// Description of the tool
        #[arg(short, long)]
        description: Option<String>,

        /// Category (e.g., search, files, git)
        #[arg(short, long)]
        category: Option<String>,

        /// Installation source (cargo, apt, snap, npm, pip, manual)
        #[arg(short, long)]
        source: Option<String>,

        /// Install command
        #[arg(short, long)]
        install_cmd: Option<String>,

        /// Binary name (if different from tool name)
        #[arg(short, long)]
        binary: Option<String>,

        /// Mark as installed
        #[arg(long)]
        installed: bool,
    },

    /// List tools in the database
    List {
        /// Show only installed tools
        #[arg(short, long)]
        installed: bool,

        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Filter by label
        #[arg(short = 'L', long)]
        label: Option<String>,

        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Search tools by name or description
    Search {
        /// Search query
        query: String,
    },

    /// Show a specific tool
    Show {
        /// Tool name
        name: String,
    },

    /// Remove a tool from the database
    Remove {
        /// Tool name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Scan system for installed tools and add them to database
    Scan {
        /// Only show what would be added (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Sync database with system (check what's installed)
    Sync {
        /// Only show what would change (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Fetch missing descriptions from package registries (PyPI, npm, crates.io)
    FetchDescriptions {
        /// Only show what would be updated (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Show suggestions for tools you don't have
    Suggest {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Show database statistics
    Stats,

    /// Show database file location
    Info,

    /// List all categories
    Categories,

    /// Check for available updates
    Updates {
        /// Filter by source (cargo, pip, npm, apt, brew)
        #[arg(short, long)]
        source: Option<String>,

        /// Check if apt/snap tools have newer versions on other sources
        #[arg(short = 'x', long)]
        cross: bool,

        /// Only show updates for tools tracked in hoard database
        #[arg(short, long)]
        tracked: bool,

        /// Show all available newer versions (not just latest)
        #[arg(short = 'a', long)]
        all_versions: bool,
    },

    /// Install a tool
    Install {
        /// Tool name to install
        name: String,

        /// Installation source (cargo, pip, npm, apt, brew, snap)
        #[arg(short, long)]
        source: Option<String>,

        /// Install a specific version
        #[arg(short = 'V', long)]
        version: Option<String>,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall a tool
    Uninstall {
        /// Tool name to uninstall
        name: String,

        /// Also remove from database
        #[arg(short, long)]
        remove: bool,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Upgrade a tool (update or switch sources)
    Upgrade {
        /// Tool name to upgrade
        name: String,

        /// Switch to a different source (cargo, pip, npm, apt, brew)
        #[arg(short, long)]
        to: Option<String>,

        /// Install a specific version
        #[arg(short = 'V', long)]
        version: Option<String>,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Manage tool bundles
    #[command(subcommand)]
    Bundle(BundleCommands),

    /// Configure AI provider for smart features
    #[command(subcommand)]
    Ai(AiCommands),

    /// GitHub integration for fetching repo info and topics
    #[command(subcommand)]
    Gh(GhCommands),

    /// List all labels
    Labels,

    /// Track and show tool usage from shell history
    #[command(subcommand)]
    Usage(UsageCommands),

    /// Find installed tools you never use
    Unused,

    /// Get tool recommendations based on your usage
    Recommend {
        /// Number of recommendations to show
        #[arg(short, long, default_value = "5")]
        count: usize,
    },

    /// Export tools database to a file
    Export {
        /// Output file path (supports .json or .toml)
        #[arg(short, long)]
        output: Option<String>,

        /// Export format (json or toml)
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Only export installed tools
        #[arg(short, long)]
        installed: bool,
    },

    /// Import tools from a file
    Import {
        /// Input file path (.json or .toml)
        file: String,

        /// Skip tools that already exist
        #[arg(short, long)]
        skip_existing: bool,

        /// Only show what would be imported (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Check database health and find issues
    Doctor {
        /// Automatically fix issues where possible
        #[arg(short, long)]
        fix: bool,
    },

    /// Edit a tool's metadata interactively
    Edit {
        /// Tool name to edit
        name: String,
    },
}

#[derive(Subcommand)]
pub enum UsageCommands {
    /// Scan shell history and update usage counts
    Scan {
        /// Only show what would be recorded (dry run)
        #[arg(short, long)]
        dry_run: bool,

        /// Reset usage counts before scanning
        #[arg(long)]
        reset: bool,
    },

    /// Show usage statistics
    Show {
        /// Number of top tools to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show usage for a specific tool
    Tool {
        /// Tool name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum AiCommands {
    /// Set the AI provider to use
    Set {
        /// AI provider (claude, gemini, codex, opencode)
        provider: String,
    },

    /// Show current AI configuration
    Show,

    /// Test AI connection
    Test,

    /// Auto-categorize uncategorized tools
    Categorize {
        /// Only show what would be changed (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Suggest tool bundles based on relationships
    SuggestBundle {
        /// Number of bundle suggestions to generate
        #[arg(short, long, default_value = "5")]
        count: usize,
    },

    /// Generate descriptions for tools missing them
    Describe {
        /// Only show what would be changed (dry run)
        #[arg(short, long)]
        dry_run: bool,

        /// Maximum number of tools to process
        #[arg(short, long)]
        limit: Option<usize>,
    },
}

#[derive(Subcommand)]
pub enum GhCommands {
    /// Sync tools with GitHub (fetch topics, descriptions, stars)
    Sync {
        /// Only show what would be changed (dry run)
        #[arg(short, long)]
        dry_run: bool,

        /// Maximum number of tools to process
        #[arg(short, long)]
        limit: Option<usize>,

        /// Delay between API calls in milliseconds (default: 2000 for Search API limit)
        #[arg(long, default_value = "2000")]
        delay: u64,
    },

    /// Show GitHub API rate limit status
    RateLimit,

    /// Backfill missing descriptions from cached GitHub data (no API calls)
    Backfill {
        /// Only show what would be changed (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Fetch GitHub info for a specific tool
    Fetch {
        /// Tool name
        name: String,
    },

    /// Search GitHub for a tool
    Search {
        /// Search query
        query: String,

        /// Maximum results
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show GitHub info for a tool
    Info {
        /// Tool name
        name: String,
    },
}

#[derive(Subcommand)]
pub enum BundleCommands {
    /// Create a new bundle
    Create {
        /// Bundle name
        name: String,

        /// Tools to include in the bundle
        #[arg(required = true)]
        tools: Vec<String>,

        /// Bundle description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// List all bundles
    List,

    /// Show tools in a bundle
    Show {
        /// Bundle name
        name: String,
    },

    /// Install all tools in a bundle
    Install {
        /// Bundle name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Add tools to an existing bundle
    Add {
        /// Bundle name
        name: String,

        /// Tools to add
        #[arg(required = true)]
        tools: Vec<String>,
    },

    /// Remove tools from a bundle
    Remove {
        /// Bundle name
        name: String,

        /// Tools to remove
        #[arg(required = true)]
        tools: Vec<String>,
    },

    /// Delete a bundle
    Delete {
        /// Bundle name
        name: String,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Update tools in a bundle (interactive per-tool choices)
    Update {
        /// Bundle name
        name: String,

        /// Auto-update all to latest without prompting
        #[arg(short, long)]
        yes: bool,
    },
}
