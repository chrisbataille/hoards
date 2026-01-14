pub mod ai;
pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod github;
pub mod history;
pub mod models;
pub mod scanner;
pub mod sources;
pub mod updates;

pub use cli::{
    AiCommands, AiConfigCommands, BundleCommands, Cli, Commands, ConfigCommands, DiscoverCommands,
    GhCommands, InsightsCommands, UsageCommands,
};
pub use commands::{
    SafeCommand, cmd_ai_categorize, cmd_ai_describe, cmd_ai_set, cmd_ai_show,
    cmd_ai_suggest_bundle, cmd_ai_test, cmd_bundle_add, cmd_bundle_create, cmd_bundle_delete,
    cmd_bundle_install, cmd_bundle_list, cmd_bundle_remove, cmd_bundle_show, cmd_bundle_update,
    cmd_config_edit, cmd_config_link, cmd_config_list, cmd_config_show, cmd_config_status,
    cmd_config_sync, cmd_config_unlink, cmd_doctor, cmd_edit, cmd_export, cmd_gh_backfill,
    cmd_gh_fetch, cmd_gh_info, cmd_gh_rate_limit, cmd_gh_search, cmd_gh_sync, cmd_import,
    cmd_install, cmd_labels, cmd_recommend, cmd_uninstall, cmd_unused, cmd_upgrade, cmd_usage_scan,
    cmd_usage_show, cmd_usage_tool, get_install_command, get_safe_install_command,
    get_safe_uninstall_command, validate_package_name,
};
pub use config::{AiProvider, HoardConfig};
pub use db::Database;
pub use models::{Bundle, Config, InstallSource, Interest, Tool};
pub use scanner::{
    KNOWN_TOOLS, is_installed, scan_known_tools, scan_missing_tools, scan_path_tools,
};
pub use sources::{PackageSource, all_sources, get_source, source_for};
