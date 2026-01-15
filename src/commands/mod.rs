//! Command implementations for hoard CLI
//!
//! Each submodule handles a group of related commands.

pub mod ai;
pub mod bundle;
pub mod completions;
pub mod config;
pub mod github;
pub mod install;
pub mod misc;
pub mod usage;

// Re-export commonly used items
pub use install::{
    SafeCommand, cmd_install, cmd_uninstall, cmd_upgrade, get_install_command,
    get_install_command_versioned, get_safe_install_command, get_safe_uninstall_command,
    validate_package_name, validate_version,
};

pub use bundle::{
    cmd_bundle_add, cmd_bundle_create, cmd_bundle_delete, cmd_bundle_install, cmd_bundle_list,
    cmd_bundle_remove, cmd_bundle_show, cmd_bundle_update,
};

pub use ai::{
    cmd_ai_bundle_cheatsheet, cmd_ai_categorize, cmd_ai_cheatsheet, cmd_ai_describe,
    cmd_ai_extract, cmd_ai_set, cmd_ai_show, cmd_ai_suggest_bundle, cmd_ai_test,
    invalidate_cheatsheet_cache,
};

pub use github::{
    cmd_gh_backfill, cmd_gh_fetch, cmd_gh_info, cmd_gh_rate_limit, cmd_gh_search, cmd_gh_sync,
};

pub use usage::{
    cmd_labels, cmd_recommend, cmd_unused, cmd_usage_config, cmd_usage_init, cmd_usage_log,
    cmd_usage_reset, cmd_usage_scan, cmd_usage_show, cmd_usage_tool, ensure_usage_configured,
};

pub use misc::{cmd_doctor, cmd_edit, cmd_export, cmd_import};

pub use config::{
    cmd_config_edit, cmd_config_link, cmd_config_list, cmd_config_show, cmd_config_status,
    cmd_config_sync, cmd_config_unlink,
};

pub use completions::{cmd_completions_install, cmd_completions_status, cmd_completions_uninstall};
