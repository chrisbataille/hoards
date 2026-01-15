//! Shared helper functions for command implementations

use anyhow::Result;
use colored::Colorize;

use crate::models::Tool;
use crate::sources::{ManualSource, source_for};

/// Prompt user for confirmation
pub fn confirm(prompt: &str) -> Result<bool> {
    print!("{} [y/N] ", prompt);
    std::io::Write::flush(&mut std::io::stdout())?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    Ok(input.trim().eq_ignore_ascii_case("y"))
}

/// Extract package name from install command (e.g., "cargo install git-delta" -> "git-delta")
pub fn extract_package_from_install_cmd(cmd: &str) -> Option<String> {
    let prefixes = [
        "cargo install ",
        "pip install ",
        "npm install -g ",
        "brew install ",
        "sudo apt install ",
    ];

    for prefix in prefixes {
        if let Some(rest) = cmd.strip_prefix(prefix) {
            let pkg = rest.split_whitespace().next().unwrap_or("");
            if !pkg.is_empty() && !pkg.starts_with('-') {
                return Some(pkg.to_string());
            }
        }
    }
    None
}

/// Fetch description for a single tool, trying multiple sources
pub fn fetch_tool_description(tool: &Tool) -> Option<(String, &'static str)> {
    let binary = tool.binary_name.as_deref().unwrap_or(&tool.name);

    // Extract actual package name from install command if available
    let pkg = tool
        .install_command
        .as_ref()
        .and_then(|c| extract_package_from_install_cmd(c))
        .unwrap_or_else(|| tool.name.clone());

    // Try package registry first based on source
    if let Some(source) = source_for(&tool.source)
        && let Some(desc) = source.fetch_description(&pkg)
    {
        return Some((desc, source.name()));
    }

    // Fallback to man page, then --help
    ManualSource::fetch_man_description(binary)
        .map(|d| (d, "man"))
        .or_else(|| ManualSource::fetch_help_description(binary).map(|d| (d, "--help")))
}

/// Print a status change line
pub fn print_status_change(name: &str, old_installed: bool, new_installed: bool) {
    let status = if new_installed {
        "installed".green()
    } else {
        "missing".red()
    };

    if old_installed != new_installed {
        println!("  {} {} -> {}", "~".yellow(), name, status);
    }
}
