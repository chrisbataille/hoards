//! Install, uninstall, and upgrade commands
//!
//! Provides safe command execution without shell interpolation.

use anyhow::{Context, Result};
use colored::Colorize;
use std::process::Command;

use crate::{Database, InstallSource, Tool, is_installed};

// ==================== Safe Command Execution ====================

/// A command with its arguments, for safe execution without shell interpolation
#[derive(Debug, Clone)]
pub struct SafeCommand {
    /// The program to run (e.g., "cargo", "sudo")
    pub program: &'static str,
    /// Arguments to pass to the program
    pub args: Vec<String>,
    /// Human-readable description for display
    pub display: String,
}

impl SafeCommand {
    /// Execute the command and return its exit status
    pub fn execute(&self) -> Result<std::process::ExitStatus> {
        Command::new(self.program)
            .args(&self.args)
            .status()
            .with_context(|| format!("Failed to execute: {}", self.display))
    }
}

impl std::fmt::Display for SafeCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

// ==================== Input Validation ====================

/// Validate a package name to prevent command injection
/// Returns an error if the name contains dangerous characters
pub fn validate_package_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Package name cannot be empty");
    }
    if name.len() > 200 {
        anyhow::bail!("Package name too long (max 200 characters)");
    }
    // Allow alphanumeric, dash, underscore, dot, and @ (for scoped npm packages)
    // Also allow / for npm scoped packages like @types/node
    let valid = name.chars().all(|c| {
        c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '@' || c == '/'
    });
    if !valid {
        anyhow::bail!(
            "Package name '{}' contains invalid characters. \
             Only alphanumeric, dash, underscore, dot, @, and / are allowed.",
            name
        );
    }
    // Prevent path traversal
    if name.contains("..") {
        anyhow::bail!("Package name cannot contain '..'");
    }
    Ok(())
}

/// Validate a version string
pub fn validate_version(version: &str) -> Result<()> {
    if version.is_empty() {
        anyhow::bail!("Version cannot be empty");
    }
    if version.len() > 50 {
        anyhow::bail!("Version too long (max 50 characters)");
    }
    // Allow alphanumeric, dash, dot, plus (for semver build metadata)
    let valid = version
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '+');
    if !valid {
        anyhow::bail!(
            "Version '{}' contains invalid characters. \
             Only alphanumeric, dash, dot, and + are allowed.",
            version
        );
    }
    Ok(())
}

// ==================== Command Generation ====================

/// Get install command string (for display/storage)
pub fn get_install_command(name: &str, source: &str) -> Option<String> {
    get_install_command_versioned(name, source, None)
}

/// Get install command string with optional version (for display/storage)
pub fn get_install_command_versioned(
    name: &str,
    source: &str,
    version: Option<&str>,
) -> Option<String> {
    match (source, version) {
        ("cargo", Some(v)) => Some(format!("cargo install {}@{}", name, v)),
        ("cargo", None) => Some(format!("cargo install {}", name)),
        ("pip", Some(v)) => Some(format!("pip install {}=={}", name, v)),
        ("pip", None) => Some(format!("pip install --upgrade {}", name)),
        ("npm", Some(v)) => Some(format!("npm install -g {}@{}", name, v)),
        ("npm", None) => Some(format!("npm install -g {}", name)),
        ("apt", _) => Some(format!("sudo apt install -y {}", name)),
        ("brew", Some(v)) => Some(format!("brew install {}@{}", name, v)),
        ("brew", None) => Some(format!("brew install {}", name)),
        ("snap", _) => Some(format!("sudo snap install {}", name)),
        _ => None,
    }
}

/// Get a safe install command (validates input, returns structured command)
pub fn get_safe_install_command(
    name: &str,
    source: &str,
    version: Option<&str>,
) -> Result<Option<SafeCommand>> {
    validate_package_name(name)?;
    if let Some(v) = version {
        validate_version(v)?;
    }

    let cmd = match (source, version) {
        ("cargo", Some(v)) => Some(SafeCommand {
            program: "cargo",
            args: vec!["install".into(), format!("{}@{}", name, v)],
            display: format!("cargo install {}@{}", name, v),
        }),
        ("cargo", None) => Some(SafeCommand {
            program: "cargo",
            args: vec!["install".into(), name.into()],
            display: format!("cargo install {}", name),
        }),
        ("pip", Some(v)) => Some(SafeCommand {
            program: "pip",
            args: vec!["install".into(), format!("{}=={}", name, v)],
            display: format!("pip install {}=={}", name, v),
        }),
        ("pip", None) => Some(SafeCommand {
            program: "pip",
            args: vec!["install".into(), "--upgrade".into(), name.into()],
            display: format!("pip install --upgrade {}", name),
        }),
        ("npm", Some(v)) => Some(SafeCommand {
            program: "npm",
            args: vec!["install".into(), "-g".into(), format!("{}@{}", name, v)],
            display: format!("npm install -g {}@{}", name, v),
        }),
        ("npm", None) => Some(SafeCommand {
            program: "npm",
            args: vec!["install".into(), "-g".into(), name.into()],
            display: format!("npm install -g {}", name),
        }),
        ("apt", _) => Some(SafeCommand {
            program: "sudo",
            args: vec!["apt".into(), "install".into(), "-y".into(), name.into()],
            display: format!("sudo apt install -y {}", name),
        }),
        ("brew", Some(v)) => Some(SafeCommand {
            program: "brew",
            args: vec!["install".into(), format!("{}@{}", name, v)],
            display: format!("brew install {}@{}", name, v),
        }),
        ("brew", None) => Some(SafeCommand {
            program: "brew",
            args: vec!["install".into(), name.into()],
            display: format!("brew install {}", name),
        }),
        ("snap", _) => Some(SafeCommand {
            program: "sudo",
            args: vec!["snap".into(), "install".into(), name.into()],
            display: format!("sudo snap install {}", name),
        }),
        ("flatpak", _) => Some(SafeCommand {
            program: "flatpak",
            args: vec!["install".into(), "-y".into(), name.into()],
            display: format!("flatpak install -y {}", name),
        }),
        _ => None,
    };
    Ok(cmd)
}

/// Get a safe uninstall command (validates input, returns structured command)
pub fn get_safe_uninstall_command(name: &str, source: &str) -> Result<Option<SafeCommand>> {
    validate_package_name(name)?;

    let cmd = match source {
        "cargo" => Some(SafeCommand {
            program: "cargo",
            args: vec!["uninstall".into(), name.into()],
            display: format!("cargo uninstall {}", name),
        }),
        "pip" => Some(SafeCommand {
            program: "pip",
            args: vec!["uninstall".into(), "-y".into(), name.into()],
            display: format!("pip uninstall -y {}", name),
        }),
        "npm" => Some(SafeCommand {
            program: "npm",
            args: vec!["uninstall".into(), "-g".into(), name.into()],
            display: format!("npm uninstall -g {}", name),
        }),
        "apt" => Some(SafeCommand {
            program: "sudo",
            args: vec!["apt".into(), "remove".into(), "-y".into(), name.into()],
            display: format!("sudo apt remove -y {}", name),
        }),
        "brew" => Some(SafeCommand {
            program: "brew",
            args: vec!["uninstall".into(), name.into()],
            display: format!("brew uninstall {}", name),
        }),
        "snap" => Some(SafeCommand {
            program: "sudo",
            args: vec!["snap".into(), "remove".into(), name.into()],
            display: format!("sudo snap remove {}", name),
        }),
        "flatpak" => Some(SafeCommand {
            program: "flatpak",
            args: vec!["uninstall".into(), "-y".into(), name.into()],
            display: format!("flatpak uninstall -y {}", name),
        }),
        _ => None,
    };
    Ok(cmd)
}

// ==================== Commands ====================

pub fn cmd_install(
    db: &Database,
    name: &str,
    source: Option<String>,
    version: Option<String>,
    force: bool,
) -> Result<()> {
    // Check if already installed
    if is_installed(name) {
        println!("{} '{}' is already installed", "!".yellow(), name);
        println!(
            "  Use {} to update it",
            format!("hoards upgrade {}", name).cyan()
        );
        return Ok(());
    }

    // Determine source - from database, argument, or ask
    let install_source = if let Some(src) = source {
        src
    } else if let Some(tool) = db.get_tool_by_name(name)? {
        // Tool exists in database, use its source
        tool.source.to_string()
    } else {
        // Tool not in database, need source argument
        println!("{} Tool '{}' not in database", "!".yellow(), name);
        println!(
            "  Specify a source with: hoards install {} --source <cargo|pip|npm|apt|brew|snap>",
            name
        );
        return Ok(());
    };

    // Get safe install command (validates package name)
    let install_cmd = match get_safe_install_command(name, &install_source, version.as_deref())? {
        Some(cmd) => cmd,
        None => {
            println!(
                "Don't know how to install '{}' from '{}'",
                name, install_source
            );
            return Ok(());
        }
    };

    // Show plan
    println!("{} Install plan for '{}':\n", ">".cyan(), name.bold());
    println!("  {}: {}", install_source.cyan(), install_cmd);

    // Confirm
    if !force {
        println!();
        print!("Proceed? [y/N] ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    println!();

    // Execute install (safe: no shell interpolation)
    println!("{} Installing from {}...", ">".cyan(), install_source);
    let status = install_cmd.execute()?;

    if !status.success() {
        println!("{} Install failed", "!".red());
        return Ok(());
    }

    let version_msg = version
        .as_ref()
        .map(|v| format!(" ({})", v))
        .unwrap_or_default();
    println!(
        "{} Installed '{}'{} successfully!",
        "+".green(),
        name,
        version_msg
    );

    // Invalidate cheatsheet cache (will be regenerated with new version)
    let _ = crate::commands::ai::invalidate_cheatsheet_cache(db, name);

    // Add to database if not already there
    if db.get_tool_by_name(name)?.is_none() {
        let tool = Tool::new(name)
            .with_source(InstallSource::from(install_source.as_str()))
            .installed();
        db.insert_tool(&tool)?;
        println!("{} Added '{}' to database", "i".cyan(), name);
    } else {
        // Update installed status
        db.set_tool_installed(name, true)?;
    }

    Ok(())
}

pub fn cmd_uninstall(db: &Database, name: &str, remove_from_db: bool, force: bool) -> Result<()> {
    // Find the tool in database
    let tool = match db.get_tool_by_name(name)? {
        Some(t) => t,
        None => {
            println!("Tool '{}' not found in database.", name);
            println!("  Add it first with: hoards add {} --source <source>", name);
            return Ok(());
        }
    };

    // Check if installed
    let binary = tool.binary_name.as_deref().unwrap_or(name);
    if !is_installed(binary) {
        println!("{} '{}' is not installed", "!".yellow(), name);
        if remove_from_db {
            db.delete_tool(name)?;
            println!("{} Removed '{}' from database", "-".red(), name);
        }
        return Ok(());
    }

    let source = tool.source.to_string();

    // Get safe uninstall command (validates package name)
    let uninstall_cmd = match get_safe_uninstall_command(name, &source)? {
        Some(cmd) => cmd,
        None => {
            println!("Don't know how to uninstall '{}' from '{}'", name, source);
            return Ok(());
        }
    };

    // Show plan
    println!("{} Uninstall plan for '{}':\n", ">".cyan(), name.bold());
    println!("  {}: {}", source.red(), uninstall_cmd);
    if remove_from_db {
        println!("  Also removing from database");
    }

    // Confirm
    if !force {
        println!();
        print!("Proceed? [y/N] ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    println!();

    // Execute uninstall (safe: no shell interpolation)
    println!("{} Uninstalling from {}...", ">".cyan(), source);
    let status = uninstall_cmd.execute()?;

    if !status.success() {
        println!("{} Uninstall failed", "!".red());
        return Ok(());
    }

    println!("{} Uninstalled '{}'", "-".red(), name);

    // Update database
    if remove_from_db {
        db.delete_tool(name)?;
        println!("{} Removed '{}' from database", "-".red(), name);
    } else {
        db.set_tool_installed(name, false)?;
        println!("{} Marked '{}' as not installed", "i".cyan(), name);
    }

    Ok(())
}

pub fn cmd_upgrade(
    db: &Database,
    name: &str,
    to_source: Option<String>,
    version: Option<String>,
    force: bool,
) -> Result<()> {
    // Find the tool in database
    let tool = match db.get_tool_by_name(name)? {
        Some(t) => t,
        None => {
            println!(
                "Tool '{}' not found in database. Run 'hoards scan' first.",
                name
            );
            return Ok(());
        }
    };

    let current_source = tool.source.to_string();

    // Determine target source
    let target_source = to_source.unwrap_or_else(|| current_source.clone());

    // Get safe install/uninstall commands (validates package names)
    let (uninstall_cmd, install_cmd) = if target_source == current_source {
        // Same source - just update (possibly to specific version)
        let install = get_safe_install_command(name, &target_source, version.as_deref())?;
        (None, install)
    } else {
        // Cross-source upgrade
        let uninstall = get_safe_uninstall_command(name, &current_source)?;
        let install = get_safe_install_command(name, &target_source, version.as_deref())?;
        (uninstall, install)
    };

    let install_cmd = match install_cmd {
        Some(cmd) => cmd,
        None => {
            println!(
                "Don't know how to install '{}' from '{}'",
                name, target_source
            );
            return Ok(());
        }
    };

    // Show plan
    println!("{} Upgrade plan for '{}':\n", ">".cyan(), name.bold());

    if let Some(ref uninstall) = uninstall_cmd {
        println!(
            "  1. Uninstall from {}: {}",
            current_source.red(),
            uninstall
        );
        println!(
            "  2. Install from {}:   {}",
            target_source.green(),
            install_cmd
        );
    } else {
        let action = if version.is_some() {
            "Install version"
        } else {
            "Update"
        };
        println!("  {} via {}: {}", action, target_source.cyan(), install_cmd);
    }

    // Confirm
    if !force {
        println!();
        print!("Proceed? [y/N] ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    println!();

    // Execute uninstall if cross-source (safe: no shell interpolation)
    if let Some(uninstall) = uninstall_cmd {
        println!("{} Uninstalling from {}...", ">".cyan(), current_source);
        let status = uninstall.execute()?;

        if !status.success() {
            println!("{} Uninstall failed, aborting", "!".red());
            return Ok(());
        }
        println!("{} Uninstalled from {}", "+".green(), current_source);
    }

    // Execute install (safe: no shell interpolation)
    println!("{} Installing from {}...", ">".cyan(), target_source);
    let status = install_cmd.execute()?;

    if !status.success() {
        println!("{} Install failed", "!".red());
        return Ok(());
    }

    let version_msg = version
        .as_ref()
        .map(|v| format!(" ({})", v))
        .unwrap_or_default();
    println!(
        "{} Upgraded '{}'{} successfully!",
        "+".green(),
        name,
        version_msg
    );

    // Invalidate cheatsheet cache (will be regenerated with new version)
    let _ = crate::commands::ai::invalidate_cheatsheet_cache(db, name);

    // Update database if source changed
    if target_source != current_source {
        let mut updated_tool = tool.clone();
        updated_tool.source = InstallSource::from(target_source.as_str());
        if let Some(cmd) = get_install_command(name, &target_source) {
            updated_tool.install_command = Some(cmd);
        }
        db.update_tool(&updated_tool)?;
        println!(
            "{} Updated database: {} -> {}",
            "i".cyan(),
            current_source,
            target_source
        );
    }

    Ok(())
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Package Name Validation Tests ====================

    #[test]
    fn test_validate_package_name_valid() {
        assert!(validate_package_name("ripgrep").is_ok());
        assert!(validate_package_name("fd-find").is_ok());
        assert!(validate_package_name("bat_tool").is_ok());
        assert!(validate_package_name("python3.11").is_ok());
        assert!(validate_package_name("@types/node").is_ok());
        assert!(validate_package_name("@babel/core").is_ok());
    }

    #[test]
    fn test_validate_package_name_empty() {
        assert!(validate_package_name("").is_err());
    }

    #[test]
    fn test_validate_package_name_too_long() {
        let long_name = "a".repeat(201);
        assert!(validate_package_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_package_name_shell_injection() {
        assert!(validate_package_name("foo; rm -rf /").is_err());
        assert!(validate_package_name("foo && cat /etc/passwd").is_err());
        assert!(validate_package_name("foo | grep secret").is_err());
        assert!(validate_package_name("$(whoami)").is_err());
        assert!(validate_package_name("`id`").is_err());
        assert!(validate_package_name("foo\nbar").is_err());
        assert!(validate_package_name("foo'bar").is_err());
        assert!(validate_package_name("foo\"bar").is_err());
        assert!(validate_package_name("foo>file").is_err());
        assert!(validate_package_name("foo<file").is_err());
    }

    #[test]
    fn test_validate_package_name_path_traversal() {
        assert!(validate_package_name("../../../etc/passwd").is_err());
        assert!(validate_package_name("foo/../bar").is_err());
    }

    // ==================== Version Validation Tests ====================

    #[test]
    fn test_validate_version_valid() {
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("2.3.4-beta.1").is_ok());
        assert!(validate_version("0.1.0+build.123").is_ok());
        assert!(validate_version("latest").is_ok());
    }

    #[test]
    fn test_validate_version_empty() {
        assert!(validate_version("").is_err());
    }

    #[test]
    fn test_validate_version_shell_injection() {
        assert!(validate_version("1.0.0; rm -rf /").is_err());
        assert!(validate_version("$(whoami)").is_err());
    }

    // ==================== Safe Command Generation Tests ====================

    #[test]
    fn test_get_safe_install_command_cargo() {
        let cmd = get_safe_install_command("ripgrep", "cargo", None)
            .unwrap()
            .unwrap();
        assert_eq!(cmd.program, "cargo");
        assert_eq!(cmd.args, vec!["install", "ripgrep"]);
    }

    #[test]
    fn test_get_safe_install_command_with_version() {
        let cmd = get_safe_install_command("ripgrep", "cargo", Some("14.0.0"))
            .unwrap()
            .unwrap();
        assert_eq!(cmd.program, "cargo");
        assert_eq!(cmd.args, vec!["install", "ripgrep@14.0.0"]);
    }

    #[test]
    fn test_get_safe_install_command_pip() {
        let cmd = get_safe_install_command("httpie", "pip", None)
            .unwrap()
            .unwrap();
        assert_eq!(cmd.program, "pip");
        assert_eq!(cmd.args, vec!["install", "--upgrade", "httpie"]);
    }

    #[test]
    fn test_get_safe_install_command_apt() {
        let cmd = get_safe_install_command("git", "apt", None)
            .unwrap()
            .unwrap();
        assert_eq!(cmd.program, "sudo");
        assert_eq!(cmd.args, vec!["apt", "install", "-y", "git"]);
    }

    #[test]
    fn test_get_safe_install_command_flatpak() {
        let cmd = get_safe_install_command("org.mozilla.firefox", "flatpak", None)
            .unwrap()
            .unwrap();
        assert_eq!(cmd.program, "flatpak");
        assert_eq!(cmd.args, vec!["install", "-y", "org.mozilla.firefox"]);
    }

    #[test]
    fn test_get_safe_uninstall_command_flatpak() {
        let cmd = get_safe_uninstall_command("org.mozilla.firefox", "flatpak")
            .unwrap()
            .unwrap();
        assert_eq!(cmd.program, "flatpak");
        assert_eq!(cmd.args, vec!["uninstall", "-y", "org.mozilla.firefox"]);
    }

    #[test]
    fn test_get_safe_install_command_rejects_injection() {
        assert!(get_safe_install_command("foo; rm -rf /", "cargo", None).is_err());
    }

    #[test]
    fn test_get_safe_uninstall_command_cargo() {
        let cmd = get_safe_uninstall_command("ripgrep", "cargo")
            .unwrap()
            .unwrap();
        assert_eq!(cmd.program, "cargo");
        assert_eq!(cmd.args, vec!["uninstall", "ripgrep"]);
    }

    #[test]
    fn test_get_safe_uninstall_command_rejects_injection() {
        assert!(get_safe_uninstall_command("foo && cat /etc/passwd", "cargo").is_err());
    }

    #[test]
    fn test_safe_command_unknown_source() {
        assert!(
            get_safe_install_command("tool", "unknown", None)
                .unwrap()
                .is_none()
        );
        assert!(
            get_safe_uninstall_command("tool", "unknown")
                .unwrap()
                .is_none()
        );
    }
}
