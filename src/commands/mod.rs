//! Command implementations for hoard CLI
//!
//! Each submodule handles a group of related commands.

pub mod ai;
pub mod bundle;
pub mod completions;
pub mod config;
pub mod core;
pub mod discover;
pub mod github;
pub mod helpers;
pub mod insights;
pub mod install;
pub mod label;
pub mod misc;
pub mod policy;
pub mod sync;
pub mod updates_cmd;
pub mod usage;
pub mod workflow;

// Re-export commonly used items from install
pub use install::{
    ProcessAction, SafeCommand, cmd_install, cmd_uninstall, cmd_upgrade, get_install_command,
    get_install_command_versioned, get_safe_install_command, get_safe_install_command_with_url,
    get_safe_uninstall_command, handle_running_process, validate_binary_name,
    validate_package_name, validate_version,
};

// Re-export core commands
pub use core::{cmd_add, cmd_list, cmd_remove, cmd_search, cmd_show};

// Re-export sync commands
pub use sync::{cmd_fetch_descriptions, cmd_scan, cmd_sync_status};

// Re-export discover commands
pub use discover::{cmd_similar, cmd_suggest, cmd_trending};

// Re-export insights commands
pub use insights::{cmd_categories, cmd_info, cmd_overview, cmd_stats};

// Re-export workflow commands
pub use workflow::{cmd_cleanup, cmd_init, cmd_maintain};

// Re-export updates commands
pub use updates_cmd::{cmd_updates, cmd_updates_cross, cmd_updates_tracked};

// Re-export helpers
pub use helpers::{confirm, extract_package_from_install_cmd, fetch_tool_description};

// Re-export bundle commands
pub use bundle::{
    cmd_bundle_add, cmd_bundle_create, cmd_bundle_delete, cmd_bundle_install, cmd_bundle_list,
    cmd_bundle_remove, cmd_bundle_show, cmd_bundle_update,
};

// Re-export AI commands
pub use ai::{
    cmd_ai_analyze, cmd_ai_bundle_cheatsheet, cmd_ai_categorize, cmd_ai_cheatsheet,
    cmd_ai_describe, cmd_ai_discover, cmd_ai_extract, cmd_ai_migrate, cmd_ai_model, cmd_ai_set,
    cmd_ai_show, cmd_ai_suggest_bundle, cmd_ai_test, invalidate_cheatsheet_cache,
};

// Re-export GitHub commands
pub use github::{
    cmd_gh_backfill, cmd_gh_fetch, cmd_gh_info, cmd_gh_rate_limit, cmd_gh_search, cmd_gh_sync,
};

// Re-export usage commands
pub use usage::{
    cmd_labels, cmd_recommend, cmd_unused, cmd_usage_config, cmd_usage_init, cmd_usage_log,
    cmd_usage_reset, cmd_usage_scan, cmd_usage_show, cmd_usage_tool, ensure_usage_configured,
};

// Re-export misc commands
pub use misc::{cmd_doctor, cmd_edit, cmd_export, cmd_import};

// Re-export config commands
pub use config::{
    cmd_config_edit, cmd_config_link, cmd_config_list, cmd_config_show, cmd_config_status,
    cmd_config_sync, cmd_config_unlink,
};

// Re-export completions commands
pub use completions::{cmd_completions_install, cmd_completions_status, cmd_completions_uninstall};

// Re-export policy commands
pub use policy::{
    cmd_policy_clear, cmd_policy_clear_bundle, cmd_policy_clear_source, cmd_policy_set,
    cmd_policy_set_bundle, cmd_policy_set_default, cmd_policy_set_source, cmd_policy_show,
};

// Re-export label commands
pub use label::{cmd_label_add, cmd_label_auto, cmd_label_clear, cmd_label_list, cmd_label_remove};
