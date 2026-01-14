//! Cargo (Rust) package source

use super::{PackageSource, http_agent};
use crate::models::{InstallSource, Tool};
use crate::scanner::{KNOWN_TOOLS, is_installed};
use anyhow::Result;
use std::process::Command;

pub struct CargoSource;

impl PackageSource for CargoSource {
    fn name(&self) -> &'static str {
        "cargo"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Cargo
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        let output = Command::new("cargo").args(["install", "--list"]).output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tools = Vec::new();
        let mut current_crate: Option<String> = None;

        for line in stdout.lines() {
            if !line.starts_with(' ') {
                // Crate name line: "ripgrep v14.1.0:"
                current_crate = line.split_whitespace().next().map(|s| s.to_string());
            } else if let Some(ref crate_name) = current_crate {
                // Binary line: "    rg"
                let binary = line.trim();
                if !binary.is_empty() && is_installed(binary) {
                    // Skip if already in KNOWN_TOOLS (we have better metadata there)
                    let dominated = KNOWN_TOOLS
                        .iter()
                        .any(|kt| kt.name == crate_name || kt.binary == binary);
                    if !dominated {
                        let mut tool = Tool::new(crate_name)
                            .with_source(InstallSource::Cargo)
                            .with_binary(binary)
                            .with_category("cli")
                            .with_install_command(self.install_command(crate_name))
                            .installed();

                        // Fetch description from crates.io
                        if let Some(description) = self.fetch_description(crate_name) {
                            tool = tool.with_description(description);
                        }

                        tools.push(tool);
                    }
                }
            }
        }

        Ok(tools)
    }

    fn fetch_description(&self, package: &str) -> Option<String> {
        let url = format!("https://crates.io/api/v1/crates/{}", package);
        let mut response = http_agent().get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("crate")?
            .get("description")?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    fn install_command(&self, package: &str) -> String {
        format!("cargo install {}", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        format!("cargo uninstall {}", package)
    }

    fn supports_updates(&self) -> bool {
        true
    }

    fn check_update(&self, package: &str, _current_version: &str) -> Option<String> {
        let url = format!("https://crates.io/api/v1/crates/{}", package);
        let mut response = http_agent().get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("crate")?
            .get("max_stable_version")?
            .as_str()
            .map(|s| s.to_string())
    }
}
