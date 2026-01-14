//! Pip (Python) package source

use super::{http_agent, PackageSource};
use crate::models::{InstallSource, Tool};
use crate::scanner::{is_installed, KNOWN_TOOLS};
use anyhow::Result;
use std::process::Command;

pub struct PipSource;

impl PackageSource for PipSource {
    fn name(&self) -> &'static str {
        "pip"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Pip
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        // Try pip3 first, then pip
        let output = Command::new("pip3")
            .args(["list", "--format=freeze"])
            .output()
            .or_else(|_| Command::new("pip").args(["list", "--format=freeze"]).output())?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tools = Vec::new();

        for line in stdout.lines() {
            // Format: "package==version"
            let package = match line.split("==").next() {
                Some(p) => p.to_lowercase().replace('_', "-"),
                None => continue,
            };

            // Skip if already in KNOWN_TOOLS
            if KNOWN_TOOLS.iter().any(|kt| kt.name == package) {
                continue;
            }

            // Check if package has a binary in PATH with same name
            if !is_installed(&package) {
                continue;
            }

            let mut tool = Tool::new(&package)
                .with_source(InstallSource::Pip)
                .with_binary(&package)
                .with_category("cli")
                .with_install_command(self.install_command(&package))
                .installed();

            // Fetch description from PyPI
            if let Some(description) = self.fetch_description(&package) {
                tool = tool.with_description(description);
            }

            tools.push(tool);
        }

        Ok(tools)
    }

    fn fetch_description(&self, package: &str) -> Option<String> {
        let url = format!("https://pypi.org/pypi/{}/json", package);
        let mut response = http_agent().get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        let summary = json.get("info")?.get("summary")?.as_str()?;

        if summary.is_empty() || summary == "UNKNOWN" {
            None
        } else {
            Some(summary.to_string())
        }
    }

    fn install_command(&self, package: &str) -> String {
        format!("pip install {}", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        format!("pip uninstall -y {}", package)
    }

    fn supports_updates(&self) -> bool {
        true
    }

    fn check_update(&self, package: &str, _current_version: &str) -> Option<String> {
        let url = format!("https://pypi.org/pypi/{}/json", package);
        let mut response = http_agent().get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("info")?
            .get("version")?
            .as_str()
            .map(|s| s.to_string())
    }
}
