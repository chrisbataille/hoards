//! Command implementations for hoard CLI
//!
//! Each submodule handles a group of related commands.

pub mod ai;
pub mod bundle;
pub mod github;
pub mod install;
pub mod misc;
pub mod usage;

// Re-export commonly used items
pub use install::{
    cmd_install, cmd_uninstall, cmd_upgrade,
    get_install_command, get_install_command_versioned,
    get_safe_install_command, get_safe_uninstall_command,
    validate_package_name, validate_version,
    SafeCommand,
};

pub use bundle::{
    cmd_bundle_add, cmd_bundle_create, cmd_bundle_delete, cmd_bundle_install,
    cmd_bundle_list, cmd_bundle_remove, cmd_bundle_show, cmd_bundle_update,
};

pub use ai::{
    cmd_ai_categorize, cmd_ai_describe, cmd_ai_set, cmd_ai_show,
    cmd_ai_suggest_bundle, cmd_ai_test,
};

pub use github::{
    cmd_gh_backfill, cmd_gh_fetch, cmd_gh_info, cmd_gh_rate_limit,
    cmd_gh_search, cmd_gh_sync,
};

pub use usage::{
    cmd_labels, cmd_recommend, cmd_unused, cmd_usage_scan,
    cmd_usage_show, cmd_usage_tool,
};

pub use misc::{
    cmd_doctor, cmd_edit, cmd_export, cmd_import,
};
