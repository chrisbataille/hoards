//! Homebrew package source

use super::{http_agent, PackageSource};
use crate::models::{InstallSource, Tool};
use crate::scanner::{is_installed, KNOWN_TOOLS};
use anyhow::Result;
use std::process::Command;

pub struct BrewSource;

impl PackageSource for BrewSource {
    fn name(&self) -> &'static str {
        "brew"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Brew
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        let output = Command::new("brew")
            .args(["list", "--formula", "-1"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tools = Vec::new();

        for package in stdout.lines() {
            let package = package.trim();
            if package.is_empty() {
                continue;
            }

            // Skip if already in KNOWN_TOOLS
            if KNOWN_TOOLS.iter().any(|kt| kt.name == package) {
                continue;
            }

            // Check if package has a binary in PATH
            if !is_installed(package) {
                continue;
            }

            let mut tool = Tool::new(package)
                .with_source(InstallSource::Brew)
                .with_binary(package)
                .with_category("cli")
                .with_install_command(self.install_command(package))
                .installed();

            // Fetch description from Homebrew API
            if let Some(description) = self.fetch_description(package) {
                tool = tool.with_description(description);
            }

            tools.push(tool);
        }

        Ok(tools)
    }

    fn fetch_description(&self, package: &str) -> Option<String> {
        let url = format!("https://formulae.brew.sh/api/formula/{}.json", package);
        let mut response = http_agent().get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("desc")?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    fn install_command(&self, package: &str) -> String {
        format!("brew install {}", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        format!("brew uninstall {}", package)
    }

    fn supports_updates(&self) -> bool {
        true
    }

    fn check_update(&self, package: &str, _current_version: &str) -> Option<String> {
        let url = format!("https://formulae.brew.sh/api/formula/{}.json", package);
        let mut response = http_agent().get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("versions")?
            .get("stable")?
            .as_str()
            .map(|s| s.to_string())
    }
}
