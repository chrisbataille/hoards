use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "hoards")]
#[command(about = "AI-powered CLI tool manager with usage analytics and multi-source tracking")]
#[command(version)]
#[command(after_help = "Use 'hoards <command> --help' for more information about a command.")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // ============================================
    // CORE COMMANDS
    // ============================================
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

    /// Show a specific tool's details
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

    /// Edit a tool's metadata interactively
    Edit {
        /// Tool name to edit
        name: String,
    },

    // ============================================
    // SYNC - Unified sync command
    // ============================================
    /// Sync database with system state
    ///
    /// By default, only checks installation status.
    /// Use flags to include additional sync operations.
    #[command(after_help = "Examples:
  hoard sync                 # Check installation status
  hoard sync --scan          # Also discover new tools
  hoard sync --github        # Also fetch GitHub data
  hoard sync --usage         # Also scan shell history
  hoard sync --all           # Do everything")]
    Sync {
        /// Only show what would change (dry run)
        #[arg(short, long)]
        dry_run: bool,

        /// Also scan system for new tools
        #[arg(long)]
        scan: bool,

        /// Also sync GitHub data (stars, topics, descriptions)
        #[arg(long)]
        github: bool,

        /// Also scan shell history for usage data
        #[arg(long)]
        usage: bool,

        /// Also fetch missing descriptions from registries
        #[arg(long)]
        descriptions: bool,

        /// Perform all sync operations (scan + github + usage + descriptions)
        #[arg(short, long)]
        all: bool,

        /// Maximum tools to process for GitHub sync
        #[arg(long)]
        limit: Option<usize>,

        /// Delay between GitHub API calls in ms (default: 2000)
        #[arg(long, default_value = "2000")]
        delay: u64,
    },

    // ============================================
    // DISCOVER - Tool discovery command group
    // ============================================
    /// Discover and explore tools
    #[command(subcommand)]
    Discover(DiscoverCommands),

    // ============================================
    // INSIGHTS - Analytics and health command group
    // ============================================
    /// View usage analytics and system health
    #[command(subcommand)]
    Insights(InsightsCommands),

    // ============================================
    // AI - AI-powered features
    // ============================================
    /// AI-powered tool management
    #[command(subcommand)]
    Ai(AiCommands),

    // ============================================
    // WORKFLOW COMMANDS
    // ============================================
    /// First-time setup wizard
    ///
    /// Guides you through initial setup:
    /// 1. Scan system for installed tools
    /// 2. Sync installation status
    /// 3. Fetch descriptions from registries
    /// 4. Optionally: GitHub sync, AI categorization
    Init {
        /// Run non-interactively with defaults
        #[arg(long)]
        auto: bool,
    },

    /// Daily/weekly maintenance routine
    ///
    /// Performs routine maintenance:
    /// 1. Sync installation status
    /// 2. Check for available updates
    /// 3. Scan shell history for usage
    /// 4. Show any health issues
    Maintain {
        /// Run non-interactively
        #[arg(long)]
        auto: bool,

        /// Only show what would be done
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Cleanup wizard for unused tools and issues
    ///
    /// Helps clean up your system:
    /// 1. Show unused installed tools
    /// 2. Show orphaned database entries
    /// 3. Fix health issues
    /// 4. Optionally remove unused tools
    Cleanup {
        /// Skip confirmations
        #[arg(short, long)]
        force: bool,

        /// Only show what would be done
        #[arg(short, long)]
        dry_run: bool,
    },

    // ============================================
    // INSTALL/UNINSTALL/UPGRADE
    // ============================================
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

    // ============================================
    // BUNDLES & CONFIG
    // ============================================
    /// Manage tool bundles
    #[command(subcommand)]
    Bundle(BundleCommands),

    /// Manage dotfiles and tool configurations
    #[command(subcommand)]
    Config(ConfigCommands),

    // ============================================
    // IMPORT/EXPORT
    // ============================================
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

    // ============================================
    // GITHUB (power user commands)
    // ============================================
    /// GitHub integration (advanced)
    #[command(subcommand)]
    Gh(GhCommands),

    // ============================================
    // SHELL COMPLETIONS
    // ============================================
    /// Manage shell completions
    #[command(subcommand)]
    Completions(CompletionsCommands),

    // ============================================
    // ALIASES (hidden, for backward compatibility)
    // ============================================
    /// List tools in the database
    #[command(hide = true)]
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
    #[command(hide = true)]
    Search {
        /// Search query
        query: String,
    },

    /// Scan system for installed tools (use 'sync --scan' instead)
    #[command(hide = true)]
    Scan {
        /// Only show what would be added (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Fetch missing descriptions (use 'sync --descriptions' instead)
    #[command(name = "fetch-descriptions", hide = true)]
    FetchDescriptions {
        /// Only show what would be updated (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Show suggestions (use 'discover missing' instead)
    #[command(hide = true)]
    Suggest {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Show database statistics (use 'insights stats' instead)
    #[command(hide = true)]
    Stats,

    /// Show database file location (use 'insights stats' instead)
    #[command(hide = true)]
    Info,

    /// List all categories (use 'discover categories' instead)
    #[command(hide = true)]
    Categories,

    /// List all labels (use 'discover labels' instead)
    #[command(hide = true)]
    Labels,

    /// Track and show tool usage (use 'insights usage' instead)
    #[command(subcommand, hide = true)]
    Usage(UsageCommands),

    /// Find unused tools (use 'insights unused' instead)
    #[command(hide = true)]
    Unused,

    /// Get recommendations (use 'discover recommended' instead)
    #[command(hide = true)]
    Recommend {
        /// Number of recommendations to show
        #[arg(short, long, default_value = "5")]
        count: usize,
    },

    /// Check database health (use 'insights health' instead)
    #[command(hide = true)]
    Doctor {
        /// Automatically fix issues where possible
        #[arg(short, long)]
        fix: bool,
    },
}

// ============================================
// DISCOVER SUBCOMMANDS
// ============================================

#[derive(Subcommand)]
pub enum DiscoverCommands {
    /// List tools in the database
    #[command(alias = "ls")]
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

        /// Also search GitHub
        #[arg(long)]
        github: bool,

        /// Maximum results for GitHub search
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Browse tools by category
    Categories,

    /// Browse tools by label
    Labels,

    /// Find tools you don't have from our curated list
    Missing {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,
    },

    /// Get AI-powered recommendations based on your usage
    Recommended {
        /// Number of recommendations to show
        #[arg(short, long, default_value = "5")]
        count: usize,
    },

    /// Find tools similar to one you already use
    Similar {
        /// Tool name to find similar tools for
        tool: String,
    },

    /// Show trending tools by GitHub stars
    Trending {
        /// Filter by category
        #[arg(short, long)]
        category: Option<String>,

        /// Number of tools to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
}

// ============================================
// INSIGHTS SUBCOMMANDS
// ============================================

#[derive(Subcommand)]
pub enum InsightsCommands {
    /// Show usage statistics
    Usage {
        /// Show usage for a specific tool
        tool: Option<String>,

        /// Number of top tools to show
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Find installed tools you never use
    Unused,

    /// Check database health and find issues
    Health {
        /// Automatically fix issues where possible
        #[arg(short, long)]
        fix: bool,
    },

    /// Show database statistics
    Stats,

    /// Show combined overview dashboard
    Overview,
}

// ============================================
// AI SUBCOMMANDS
// ============================================

#[derive(Subcommand)]
pub enum AiCommands {
    /// Configure AI provider
    #[command(subcommand)]
    Config(AiConfigCommands),

    /// Enrich tool data using AI
    ///
    /// Automatically categorize and describe tools using AI.
    Enrich {
        /// Categorize uncategorized tools
        #[arg(long)]
        categorize: bool,

        /// Generate descriptions for tools missing them
        #[arg(long)]
        describe: bool,

        /// Do both categorize and describe
        #[arg(short, long)]
        all: bool,

        /// Only show what would be changed (dry run)
        #[arg(short, long)]
        dry_run: bool,

        /// Maximum number of tools to process
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Suggest tool bundles based on your installed tools
    SuggestBundle {
        /// Number of bundle suggestions to generate
        #[arg(short, long, default_value = "5")]
        count: usize,
    },

    /// Extract tool info from GitHub repository README
    ///
    /// Uses AI to parse README and extract tool metadata (name, binary, source, description).
    /// Results are cached per repository version to avoid repeated API calls.
    Extract {
        /// GitHub repository URLs (e.g., https://github.com/BurntSushi/ripgrep)
        #[arg(required = true)]
        urls: Vec<String>,

        /// Add extracted tools to database without confirmation
        #[arg(short, long)]
        yes: bool,

        /// Only show what would be extracted (dry run)
        #[arg(short, long)]
        dry_run: bool,

        /// Delay between API calls in milliseconds (for batch mode)
        #[arg(long, default_value = "1000")]
        delay: u64,
    },

    /// Generate a quick reference cheatsheet for a tool
    ///
    /// Uses AI to analyze the tool's --help output and create a concise,
    /// categorized cheatsheet of the most useful commands.
    Cheatsheet {
        /// Tool name (must be installed, omit if using --bundle)
        tool: Option<String>,

        /// Generate combined cheatsheet for all tools in a bundle
        #[arg(short, long, conflicts_with = "tool")]
        bundle: Option<String>,

        /// Refresh cached cheatsheet
        #[arg(short, long)]
        refresh: bool,
    },

    // Hidden aliases for backward compatibility
    /// Set the AI provider (use 'ai config set' instead)
    #[command(hide = true)]
    Set {
        /// AI provider (claude, gemini, codex, opencode)
        provider: String,
    },

    /// Show current AI configuration (use 'ai config show' instead)
    #[command(name = "show", hide = true)]
    ShowConfig,

    /// Test AI connection (use 'ai config test' instead)
    #[command(hide = true)]
    Test,

    /// Auto-categorize tools (use 'ai enrich --categorize' instead)
    #[command(hide = true)]
    Categorize {
        /// Only show what would be changed (dry run)
        #[arg(short, long)]
        dry_run: bool,
    },

    /// Generate descriptions (use 'ai enrich --describe' instead)
    #[command(hide = true)]
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
pub enum AiConfigCommands {
    /// Set the AI provider to use
    Set {
        /// AI provider (claude, gemini, codex, opencode)
        provider: String,
    },

    /// Show current AI configuration
    Show,

    /// Test AI connection
    Test,
}

// ============================================
// GITHUB SUBCOMMANDS (power user)
// ============================================

#[derive(Subcommand)]
pub enum GhCommands {
    /// Sync tools with GitHub (use 'sync --github' instead for most cases)
    #[command(hide = true)]
    Sync {
        /// Only show what would be changed (dry run)
        #[arg(short, long)]
        dry_run: bool,

        /// Maximum number of tools to process
        #[arg(short, long)]
        limit: Option<usize>,

        /// Delay between API calls in milliseconds
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

// ============================================
// USAGE SUBCOMMANDS (hidden, use insights usage)
// ============================================

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

    /// Log a single command usage (for shell hooks)
    Log {
        /// Command that was executed
        command: String,
    },

    /// Show shell hook setup instructions
    Init {
        /// Shell type (auto-detected if omitted)
        #[arg(value_parser = ["fish", "bash", "zsh"])]
        shell: Option<String>,
    },

    /// View or change usage tracking configuration
    Config {
        /// Set tracking mode
        #[arg(long, value_parser = ["scan", "hook"])]
        mode: Option<String>,
    },

    /// Reset all usage counters to zero
    Reset {
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum CompletionsCommands {
    /// Generate completions and print to stdout
    Generate {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Install completions for detected shells
    Install {
        /// Specific shell to install for (auto-detects if omitted)
        #[arg(value_parser = ["fish", "bash", "zsh"])]
        shell: Option<String>,

        /// Overwrite existing completions
        #[arg(short, long)]
        force: bool,
    },

    /// Show completion installation status
    Status,

    /// Remove installed completions
    Uninstall {
        /// Specific shell to uninstall for (all detected if omitted)
        #[arg(value_parser = ["fish", "bash", "zsh"])]
        shell: Option<String>,
    },
}

// ============================================
// BUNDLE SUBCOMMANDS
// ============================================

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

// ============================================
// CONFIG SUBCOMMANDS
// ============================================

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Link a config directory to be managed by hoard
    Link {
        /// Config name (e.g., "fish", "nvim", "alacritty")
        name: String,

        /// Target path where config should be symlinked (e.g., ~/.config/fish)
        target: String,

        /// Source path in your dotfiles repo (e.g., ./shell/fish)
        source: String,

        /// Associate with a tool
        #[arg(short, long)]
        tool: Option<String>,
    },

    /// Unlink a config (removes from database, optionally removes symlink)
    Unlink {
        /// Config name
        name: String,

        /// Also remove the symlink
        #[arg(short, long)]
        remove_symlink: bool,

        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// List all managed configs
    List {
        /// Show only configs with broken symlinks
        #[arg(short, long)]
        broken: bool,

        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },

    /// Show details for a specific config
    Show {
        /// Config name
        name: String,
    },

    /// Create symlinks for all managed configs
    Sync {
        /// Only show what would be done (dry run)
        #[arg(short, long)]
        dry_run: bool,

        /// Force overwrite existing files (not symlinks)
        #[arg(short, long)]
        force: bool,
    },

    /// Check status of all config symlinks
    Status,

    /// Edit a config's paths
    Edit {
        /// Config name
        name: String,

        /// New target path
        #[arg(short, long)]
        target: Option<String>,

        /// New source path
        #[arg(short, long)]
        source: Option<String>,

        /// Associate with a tool
        #[arg(long)]
        tool: Option<String>,
    },
}
