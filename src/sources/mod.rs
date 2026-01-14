//! Package source trait and implementations
//!
//! Each package manager (cargo, pip, npm, etc.) implements the `PackageSource` trait,
//! providing a unified interface for scanning, fetching descriptions, and managing tools.

mod apt;
mod brew;
mod cargo;
mod flatpak;
mod manual;
mod npm;
mod pip;

pub use apt::AptSource;
pub use brew::BrewSource;
pub use cargo::CargoSource;
pub use flatpak::FlatpakSource;
pub use manual::ManualSource;
pub use npm::NpmSource;
pub use pip::PipSource;

use crate::models::{InstallSource, Tool};
use anyhow::Result;
use std::time::Duration;

/// Trait for package managers/sources
///
/// Implement this trait to add support for a new package source.
/// Each source provides methods for:
/// - Scanning installed packages
/// - Fetching package descriptions from registries
/// - Generating install/uninstall commands
pub trait PackageSource: Send + Sync {
    /// Unique identifier for this source (e.g., "cargo", "pip")
    fn name(&self) -> &'static str;

    /// The InstallSource enum variant for this source
    fn install_source(&self) -> InstallSource;

    /// Scan system for installed packages from this source
    fn scan(&self) -> Result<Vec<Tool>>;

    /// Fetch description from package registry
    /// Returns None if not available or request fails
    fn fetch_description(&self, package: &str) -> Option<String>;

    /// Generate install command for a package
    fn install_command(&self, package: &str) -> String;

    /// Generate uninstall command for a package
    fn uninstall_command(&self, package: &str) -> String;

    /// Check if this source supports update checking
    fn supports_updates(&self) -> bool {
        false
    }

    /// Check for available updates (package_name -> latest_version)
    fn check_update(&self, _package: &str, _current_version: &str) -> Option<String> {
        None
    }
}

/// Create a shared HTTP agent with timeout for API requests
pub(crate) fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .new_agent()
}

/// Get all available package sources
pub fn all_sources() -> Vec<Box<dyn PackageSource>> {
    vec![
        Box::new(CargoSource),
        Box::new(PipSource),
        Box::new(NpmSource),
        Box::new(BrewSource),
        Box::new(AptSource),
        Box::new(FlatpakSource),
        Box::new(ManualSource),
    ]
}

/// Get a specific source by name
pub fn get_source(name: &str) -> Option<Box<dyn PackageSource>> {
    match name.to_lowercase().as_str() {
        "cargo" => Some(Box::new(CargoSource)),
        "pip" => Some(Box::new(PipSource)),
        "npm" => Some(Box::new(NpmSource)),
        "brew" => Some(Box::new(BrewSource)),
        "apt" => Some(Box::new(AptSource)),
        "flatpak" => Some(Box::new(FlatpakSource)),
        "manual" => Some(Box::new(ManualSource)),
        _ => None,
    }
}

/// Get source for an InstallSource enum
pub fn source_for(install_source: &InstallSource) -> Option<Box<dyn PackageSource>> {
    match install_source {
        InstallSource::Cargo => Some(Box::new(CargoSource)),
        InstallSource::Pip => Some(Box::new(PipSource)),
        InstallSource::Npm => Some(Box::new(NpmSource)),
        InstallSource::Brew => Some(Box::new(BrewSource)),
        InstallSource::Apt => Some(Box::new(AptSource)),
        InstallSource::Flatpak => Some(Box::new(FlatpakSource)),
        InstallSource::Manual => Some(Box::new(ManualSource)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== all_sources Tests ====================

    #[test]
    fn test_all_sources_returns_expected_count() {
        let sources = all_sources();
        assert_eq!(sources.len(), 7);
    }

    #[test]
    fn test_all_sources_have_unique_names() {
        let sources = all_sources();
        let names: Vec<_> = sources.iter().map(|s| s.name()).collect();

        // Check no duplicates
        let mut unique_names = names.clone();
        unique_names.sort();
        unique_names.dedup();
        assert_eq!(names.len(), unique_names.len());
    }

    #[test]
    fn test_all_sources_contain_expected_sources() {
        let sources = all_sources();
        let names: Vec<_> = sources.iter().map(|s| s.name()).collect();

        assert!(names.contains(&"cargo"));
        assert!(names.contains(&"pip"));
        assert!(names.contains(&"npm"));
        assert!(names.contains(&"brew"));
        assert!(names.contains(&"apt"));
        assert!(names.contains(&"flatpak"));
        assert!(names.contains(&"manual"));
    }

    // ==================== get_source Tests ====================

    #[test]
    fn test_get_source_valid() {
        assert!(get_source("cargo").is_some());
        assert!(get_source("pip").is_some());
        assert!(get_source("npm").is_some());
        assert!(get_source("brew").is_some());
        assert!(get_source("apt").is_some());
        assert!(get_source("flatpak").is_some());
        assert!(get_source("manual").is_some());
    }

    #[test]
    fn test_get_source_case_insensitive() {
        assert!(get_source("CARGO").is_some());
        assert!(get_source("Cargo").is_some());
        assert!(get_source("CaRgO").is_some());
    }

    #[test]
    fn test_get_source_invalid() {
        assert!(get_source("invalid").is_none());
        assert!(get_source("").is_none());
        assert!(get_source("snap").is_none()); // not implemented
    }

    // ==================== source_for Tests ====================

    #[test]
    fn test_source_for_valid() {
        assert!(source_for(&InstallSource::Cargo).is_some());
        assert!(source_for(&InstallSource::Pip).is_some());
        assert!(source_for(&InstallSource::Npm).is_some());
        assert!(source_for(&InstallSource::Brew).is_some());
        assert!(source_for(&InstallSource::Apt).is_some());
        assert!(source_for(&InstallSource::Flatpak).is_some());
        assert!(source_for(&InstallSource::Manual).is_some());
    }

    #[test]
    fn test_source_for_unknown() {
        assert!(source_for(&InstallSource::Unknown).is_none());
        assert!(source_for(&InstallSource::Snap).is_none());
    }

    #[test]
    fn test_source_for_matches_install_source() {
        // Verify that source_for returns a source with matching install_source
        let cargo = source_for(&InstallSource::Cargo).unwrap();
        assert_eq!(cargo.install_source(), InstallSource::Cargo);

        let pip = source_for(&InstallSource::Pip).unwrap();
        assert_eq!(pip.install_source(), InstallSource::Pip);
    }

    // ==================== PackageSource Trait Tests ====================

    #[test]
    fn test_cargo_source_properties() {
        let source = CargoSource;
        assert_eq!(source.name(), "cargo");
        assert_eq!(source.install_source(), InstallSource::Cargo);
        assert_eq!(source.install_command("ripgrep"), "cargo install ripgrep");
        assert_eq!(
            source.uninstall_command("ripgrep"),
            "cargo uninstall ripgrep"
        );
    }

    #[test]
    fn test_pip_source_properties() {
        let source = PipSource;
        assert_eq!(source.name(), "pip");
        assert_eq!(source.install_source(), InstallSource::Pip);
        assert_eq!(source.install_command("httpie"), "pip install httpie");
        assert_eq!(
            source.uninstall_command("httpie"),
            "pip uninstall -y httpie"
        );
    }

    #[test]
    fn test_npm_source_properties() {
        let source = NpmSource;
        assert_eq!(source.name(), "npm");
        assert_eq!(source.install_source(), InstallSource::Npm);
        assert_eq!(
            source.install_command("prettier"),
            "npm install -g prettier"
        );
        assert_eq!(
            source.uninstall_command("prettier"),
            "npm uninstall -g prettier"
        );
    }

    #[test]
    fn test_brew_source_properties() {
        let source = BrewSource;
        assert_eq!(source.name(), "brew");
        assert_eq!(source.install_source(), InstallSource::Brew);
        assert_eq!(source.install_command("jq"), "brew install jq");
        assert_eq!(source.uninstall_command("jq"), "brew uninstall jq");
    }

    #[test]
    fn test_apt_source_properties() {
        let source = AptSource;
        assert_eq!(source.name(), "apt");
        assert_eq!(source.install_source(), InstallSource::Apt);
        assert_eq!(source.install_command("git"), "sudo apt install git");
        assert_eq!(source.uninstall_command("git"), "sudo apt remove git");
    }

    #[test]
    fn test_flatpak_source_properties() {
        let source = FlatpakSource;
        assert_eq!(source.name(), "flatpak");
        assert_eq!(source.install_source(), InstallSource::Flatpak);
        assert_eq!(
            source.install_command("org.mozilla.firefox"),
            "flatpak install -y org.mozilla.firefox"
        );
        assert_eq!(
            source.uninstall_command("org.mozilla.firefox"),
            "flatpak uninstall -y org.mozilla.firefox"
        );
        assert!(source.supports_updates());
    }

    #[test]
    fn test_manual_source_properties() {
        let source = ManualSource;
        assert_eq!(source.name(), "manual");
        assert_eq!(source.install_source(), InstallSource::Manual);
        // Manual source returns comment-style instructions
        assert!(source.install_command("tool").contains("Manual"));
        assert!(source.uninstall_command("tool").contains("Manual"));
    }

    // ==================== Default Trait Method Tests ====================

    #[test]
    fn test_default_supports_updates() {
        // Most sources don't support updates by default
        let source = ManualSource;
        assert!(!source.supports_updates());
    }

    #[test]
    fn test_default_check_update() {
        let source = ManualSource;
        assert!(source.check_update("tool", "1.0.0").is_none());
    }
}
