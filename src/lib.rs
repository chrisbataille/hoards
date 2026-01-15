pub mod ai;
pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod github;
pub mod history;
pub mod http;
pub mod icons;
pub mod models;
pub mod scanner;
pub mod sources;
pub mod updates;

pub use cli::{
    AiCommands, AiConfigCommands, BundleCommands, Cli, Commands, CompletionsCommands,
    ConfigCommands, DiscoverCommands, GhCommands, InsightsCommands, UsageCommands,
};

// Core commands
pub use commands::{cmd_add, cmd_list, cmd_remove, cmd_search, cmd_show};

// Sync commands
pub use commands::{cmd_fetch_descriptions, cmd_scan, cmd_sync_status};

// Discover commands
pub use commands::{cmd_similar, cmd_suggest, cmd_trending};

// Insights commands
pub use commands::{cmd_categories, cmd_info, cmd_overview, cmd_stats};

// Workflow commands
pub use commands::{cmd_cleanup, cmd_init, cmd_maintain};

// Updates commands
pub use commands::{cmd_updates, cmd_updates_cross, cmd_updates_tracked};

// Install commands
pub use commands::{
    SafeCommand, cmd_install, cmd_uninstall, cmd_upgrade, get_install_command,
    get_safe_install_command, get_safe_uninstall_command, validate_package_name,
};

// AI commands
pub use commands::{
    cmd_ai_analyze, cmd_ai_bundle_cheatsheet, cmd_ai_categorize, cmd_ai_cheatsheet,
    cmd_ai_describe, cmd_ai_discover, cmd_ai_extract, cmd_ai_migrate, cmd_ai_set, cmd_ai_show,
    cmd_ai_suggest_bundle, cmd_ai_test,
};

// Bundle commands
pub use commands::{
    cmd_bundle_add, cmd_bundle_create, cmd_bundle_delete, cmd_bundle_install, cmd_bundle_list,
    cmd_bundle_remove, cmd_bundle_show, cmd_bundle_update,
};

// GitHub commands
pub use commands::{
    cmd_gh_backfill, cmd_gh_fetch, cmd_gh_info, cmd_gh_rate_limit, cmd_gh_search, cmd_gh_sync,
};

// Usage commands
pub use commands::{
    cmd_labels, cmd_recommend, cmd_unused, cmd_usage_config, cmd_usage_init, cmd_usage_log,
    cmd_usage_reset, cmd_usage_scan, cmd_usage_show, cmd_usage_tool, ensure_usage_configured,
};

// Misc commands
pub use commands::{cmd_doctor, cmd_edit, cmd_export, cmd_import};

// Config commands
pub use commands::{
    cmd_config_edit, cmd_config_link, cmd_config_list, cmd_config_show, cmd_config_status,
    cmd_config_sync, cmd_config_unlink,
};

// Completions commands
pub use commands::{cmd_completions_install, cmd_completions_status, cmd_completions_uninstall};

// Config types
pub use config::{AiProvider, HoardConfig};

// Database
pub use db::{CachedExtraction, Database};

// Models
pub use models::{Bundle, Config, InstallSource, Interest, Tool};

// Scanner
pub use scanner::{
    KNOWN_TOOLS, is_installed, scan_known_tools, scan_missing_tools, scan_path_tools,
};

// Sources
pub use sources::{PackageSource, all_sources, get_source, source_for};
