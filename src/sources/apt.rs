//! Apt (Debian/Ubuntu) package source

use super::PackageSource;
use crate::models::{InstallSource, Tool};
use crate::scanner::{is_installed, KNOWN_TOOLS};
use anyhow::Result;
use std::process::Command;

pub struct AptSource;

/// GUI-related apt sections to skip
const GUI_SECTIONS: &[&str] = &[
    "x11", "gnome", "kde", "xfce", "lxde", "lxqt", "mate", "cinnamon", "graphics", "video",
    "sound", "games", "fonts", "libdevel",
];

/// GUI-related dependencies to skip
const GUI_DEPS: &[&str] = &[
    "libgtk", "libqt", "libx11", "libwayland", "libgl", "libvulkan", "libsdl", "libegl", "libgdk",
    "libwx", "libfltk", "libcairo", "libpango", "libglib", "libgio",
];

/// Known GUI package names/patterns to skip
const GUI_PACKAGES: &[&str] = &[
    "firefox",
    "thunderbird",
    "chrome",
    "chromium",
    "code",
    "slack",
    "discord",
    "telegram",
    "signal",
    "spotify",
    "vlc",
    "gimp",
    "inkscape",
    "blender",
    "libreoffice",
    "solaar",
    "claude-desktop",
];

/// Package name patterns that indicate GUI apps
const GUI_PATTERNS: &[&str] = &[
    "-gtk", "-gnome", "-kde", "-qt", "-gui", "-desktop", "-applet",
];

impl AptSource {
    /// Map apt section to functional category
    fn section_to_category(section: &str) -> &'static str {
        // Extract base section (e.g., "universe/utils" -> "utils")
        let base = section.rsplit('/').next().unwrap_or(section);

        match base {
            "admin" | "utils" => "system",
            "devel" => "dev",
            "net" | "web" | "mail" => "network",
            "text" => "text",
            "editors" => "editor",
            "kernel" => "system",
            "shells" => "shell",
            "vcs" => "git",
            "database" => "data",
            "interpreters" => "lang",
            "perl" | "python" | "javascript" | "ruby" | "rust" | "golang" => "lang",
            "doc" | "documentation" => "docs",
            "debug" => "dev",
            "embedded" => "system",
            "electronics" => "dev",
            "science" | "math" => "data",
            "comm" => "network",
            _ => "cli",
        }
    }

    /// Check if an apt package depends on GUI libraries
    fn has_gui_dependencies(package: &str) -> bool {
        let output = Command::new("apt-cache")
            .args(["depends", package])
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let deps = String::from_utf8_lossy(&out.stdout).to_lowercase();
                GUI_DEPS.iter().any(|dep| deps.contains(dep))
            }
            _ => false,
        }
    }
}

impl PackageSource for AptSource {
    fn name(&self) -> &'static str {
        "apt"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Apt
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        // Get list of installed packages with their sections
        let output = Command::new("dpkg-query")
            .args(["-W", "-f", "${Package}\t${Section}\t${binary:Summary}\n"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tools = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(3, '\t').collect();
            if parts.len() < 2 {
                continue;
            }

            let package = parts[0];
            let section = parts.get(1).unwrap_or(&"");
            let description = parts.get(2).map(|s| s.to_string());

            // Skip GUI sections
            if GUI_SECTIONS.iter().any(|s| section.contains(s)) {
                continue;
            }

            // Skip libraries and dev packages
            if package.starts_with("lib") || package.ends_with("-dev") || package.ends_with("-doc")
            {
                continue;
            }

            // Skip known GUI packages
            if GUI_PACKAGES.iter().any(|p| package.contains(p)) {
                continue;
            }

            // Skip packages with GUI-indicating patterns
            if GUI_PATTERNS.iter().any(|p| package.contains(p)) {
                continue;
            }

            // Skip if already in KNOWN_TOOLS
            if KNOWN_TOOLS.iter().any(|kt| kt.name == package) {
                continue;
            }

            // Check if package has a binary in PATH with same name
            if !is_installed(package) {
                continue;
            }

            // Check if it depends on GUI libraries
            if Self::has_gui_dependencies(package) {
                continue;
            }

            let category = Self::section_to_category(section);

            let mut tool = Tool::new(package)
                .with_source(InstallSource::Apt)
                .with_binary(package)
                .with_category(category)
                .with_install_command(self.install_command(package))
                .installed();

            if let Some(desc) = description
                && !desc.is_empty() {
                    tool = tool.with_description(desc);
                }

            tools.push(tool);
        }

        Ok(tools)
    }

    fn fetch_description(&self, package: &str) -> Option<String> {
        // Use dpkg-query for local description (no remote API)
        let output = Command::new("dpkg-query")
            .args(["-W", "-f", "${binary:Summary}", package])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let desc = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if desc.is_empty() {
            None
        } else {
            Some(desc)
        }
    }

    fn install_command(&self, package: &str) -> String {
        format!("sudo apt install {}", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        format!("sudo apt remove {}", package)
    }
}
