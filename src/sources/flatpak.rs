//! Flatpak package source

use super::PackageSource;
use crate::models::{InstallSource, Tool};
use crate::scanner::is_installed;
use anyhow::Result;
use std::process::Command;

pub struct FlatpakSource;

impl FlatpakSource {
    /// Extract the application name from a Flatpak application ID
    /// e.g., "org.mozilla.firefox" -> "firefox"
    fn app_name_from_id(app_id: &str) -> String {
        app_id
            .rsplit('.')
            .next()
            .unwrap_or(app_id)
            .to_lowercase()
    }

    /// Map Flatpak remote to category
    fn remote_to_category(remote: &str) -> &'static str {
        match remote {
            "flathub" => "app",
            "flathub-beta" => "app",
            "fedora" => "system",
            "gnome-nightly" => "dev",
            _ => "app",
        }
    }
}

impl PackageSource for FlatpakSource {
    fn name(&self) -> &'static str {
        "flatpak"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Flatpak
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        // List installed Flatpak applications
        // Format: Application ID, Version, Branch, Origin, Installation
        let output = Command::new("flatpak")
            .args(["list", "--app", "--columns=application,version,origin"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tools = Vec::new();

        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.is_empty() {
                continue;
            }

            let app_id = parts[0].trim();
            let version = parts.get(1).map(|s| s.trim()).filter(|s| !s.is_empty());
            let origin = parts.get(2).map(|s| s.trim()).unwrap_or("flathub");

            // Extract human-readable name from app ID
            let name = Self::app_name_from_id(app_id);

            // Skip if the app has a CLI binary with the same name
            // (Flatpak apps are typically GUI, so we check if there's a CLI version)
            if !is_installed(&name) {
                // No CLI binary found - this is a GUI-only Flatpak app
                // Still track it but mark appropriately
            }

            let category = Self::remote_to_category(origin);

            let mut tool = Tool::new(&name)
                .with_source(InstallSource::Flatpak)
                .with_binary(app_id) // Store full app ID as binary name for install/uninstall
                .with_category(category)
                .with_install_command(self.install_command(app_id))
                .installed();

            // Add version to notes if available
            if let Some(ver) = version {
                tool.notes = Some(format!("Version: {}", ver));
            }

            // Fetch description
            if let Some(desc) = self.fetch_description(app_id) {
                tool = tool.with_description(desc);
            }

            tools.push(tool);
        }

        Ok(tools)
    }

    fn fetch_description(&self, package: &str) -> Option<String> {
        // Use flatpak info to get application metadata
        let output = Command::new("flatpak")
            .args(["info", package])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Look for the "Subject:" or "Description:" line
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("Subject:") {
                return Some(line.trim_start_matches("Subject:").trim().to_string());
            }
        }

        // Fallback: try to extract from Name field
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("Name:") {
                return Some(line.trim_start_matches("Name:").trim().to_string());
            }
        }

        None
    }

    fn install_command(&self, package: &str) -> String {
        format!("flatpak install -y {}", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        format!("flatpak uninstall -y {}", package)
    }

    fn supports_updates(&self) -> bool {
        true
    }

    fn check_update(&self, package: &str, _current_version: &str) -> Option<String> {
        // Check if there's an update available for this package
        let output = Command::new("flatpak")
            .args(["remote-info", "--cached", "flathub", package])
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Look for Version line
        for line in stdout.lines() {
            let line = line.trim();
            if line.starts_with("Version:") {
                let remote_version = line.trim_start_matches("Version:").trim();
                if remote_version != _current_version {
                    return Some(remote_version.to_string());
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatpak_source_properties() {
        let source = FlatpakSource;
        assert_eq!(source.name(), "flatpak");
        assert_eq!(source.install_source(), InstallSource::Flatpak);
        assert!(source.supports_updates());
    }

    #[test]
    fn test_flatpak_install_command() {
        let source = FlatpakSource;
        assert_eq!(
            source.install_command("org.mozilla.firefox"),
            "flatpak install -y org.mozilla.firefox"
        );
    }

    #[test]
    fn test_flatpak_uninstall_command() {
        let source = FlatpakSource;
        assert_eq!(
            source.uninstall_command("org.mozilla.firefox"),
            "flatpak uninstall -y org.mozilla.firefox"
        );
    }

    #[test]
    fn test_app_name_from_id() {
        assert_eq!(FlatpakSource::app_name_from_id("org.mozilla.firefox"), "firefox");
        assert_eq!(FlatpakSource::app_name_from_id("com.visualstudio.code"), "code");
        assert_eq!(FlatpakSource::app_name_from_id("org.gnome.Calculator"), "calculator");
        assert_eq!(FlatpakSource::app_name_from_id("simple"), "simple");
    }

    #[test]
    fn test_remote_to_category() {
        assert_eq!(FlatpakSource::remote_to_category("flathub"), "app");
        assert_eq!(FlatpakSource::remote_to_category("flathub-beta"), "app");
        assert_eq!(FlatpakSource::remote_to_category("fedora"), "system");
        assert_eq!(FlatpakSource::remote_to_category("gnome-nightly"), "dev");
        assert_eq!(FlatpakSource::remote_to_category("unknown"), "app");
    }
}
