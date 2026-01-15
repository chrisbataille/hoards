//! Npm (Node.js) package source

use super::PackageSource;
use crate::http::HTTP_AGENT;
use crate::models::{InstallSource, Tool};
use crate::scanner::{KNOWN_TOOLS, is_installed};
use anyhow::Result;
use std::process::Command;

pub struct NpmSource;

impl PackageSource for NpmSource {
    fn name(&self) -> &'static str {
        "npm"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Npm
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        let output = Command::new("npm")
            .args(["list", "-g", "--depth=0", "--json"])
            .output()?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse JSON to get package names
        let json: serde_json::Value = match serde_json::from_str(&stdout) {
            Ok(v) => v,
            Err(_) => return Ok(Vec::new()),
        };

        let mut tools = Vec::new();

        if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
            for (package, _) in deps {
                // Skip npm itself
                if package == "npm" {
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
                    .with_source(InstallSource::Npm)
                    .with_binary(package)
                    .with_category("cli")
                    .with_install_command(self.install_command(package))
                    .installed();

                // Fetch description from npm registry
                if let Some(description) = self.fetch_description(package) {
                    tool = tool.with_description(description);
                }

                tools.push(tool);
            }
        }

        Ok(tools)
    }

    fn fetch_description(&self, package: &str) -> Option<String> {
        let url = format!("https://registry.npmjs.org/{}", package);
        let mut response = HTTP_AGENT.get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("description")?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
    }

    fn install_command(&self, package: &str) -> String {
        format!("npm install -g {}", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        format!("npm uninstall -g {}", package)
    }

    fn supports_updates(&self) -> bool {
        true
    }

    fn check_update(&self, package: &str, _current_version: &str) -> Option<String> {
        let url = format!("https://registry.npmjs.org/{}", package);
        let mut response = HTTP_AGENT.get(&url).call().ok()?;
        let json: serde_json::Value = response.body_mut().read_json().ok()?;

        json.get("dist-tags")?
            .get("latest")?
            .as_str()
            .map(|s| s.to_string())
    }
}
