//! Manual/fallback package source
//!
//! Used for tools installed via other means (go install, curl scripts, etc.)
//! Provides description fetching via man pages and --help output.

use super::PackageSource;
use crate::models::{InstallSource, Tool};
use anyhow::Result;
use std::process::Command;

pub struct ManualSource;

impl ManualSource {
    /// Extract description from man page NAME section
    pub fn fetch_man_description(binary: &str) -> Option<String> {
        let output = Command::new("man")
            .args(["-f", binary]) // whatis format: "tool (1) - description"
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse "tool (section) - description" format
        for line in stdout.lines() {
            if let Some(pos) = line.find(" - ") {
                let desc = line[pos + 3..].trim();
                if !desc.is_empty() {
                    // Capitalize first letter
                    let mut chars = desc.chars();
                    return Some(match chars.next() {
                        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                        None => desc.to_string(),
                    });
                }
            }
        }
        None
    }

    /// Extract description from --help output
    pub fn fetch_help_description(binary: &str) -> Option<String> {
        // Try --help first, then -h
        let output = Command::new(binary)
            .arg("--help")
            .output()
            .or_else(|_| Command::new(binary).arg("-h").output())
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let text = if stdout.len() > stderr.len() {
            stdout
        } else {
            stderr
        };

        // Skip if too short
        if text.len() < 10 {
            return None;
        }

        for line in text.lines().take(25) {
            let line = line.trim();

            // Skip empty or too short lines
            if line.len() < 15 {
                continue;
            }

            // Skip lines that look like usage, options, or technical output
            if line.starts_with("Usage:")
                || line.starts_with("usage:")
                || line.starts_with('-')
                || line.starts_with('[')
                || line.starts_with("Options:")
                || line.starts_with("Commands:")
                || line.starts_with("Arguments:")
                || line.starts_with("USAGE:")
                || line.starts_with("FLAGS:")
                || line.starts_with("Error:")
                || line.contains("[--") // Option patterns
                || line.contains("<")    // Argument placeholders
                || line.contains("├")    // Tree output
                || line.contains("└")
                || line.contains("▄")    // ASCII art
                || line.contains("▀")
                || line.contains("[0m")  // ANSI codes
                || line.contains("[38;")
                || line.chars().filter(|c| *c == '-').count() > 3
            // Option-heavy lines
            {
                continue;
            }

            // Take first sentence or first 80 chars
            let desc = if let Some(pos) = line.find(". ") {
                &line[..pos]
            } else if line.chars().count() > 80 {
                // Find byte index at 80th character boundary (safe for UTF-8)
                line.char_indices()
                    .nth(80)
                    .map_or(line, |(idx, _)| &line[..idx])
            } else {
                line
            };

            // Skip if it looks like a command name, version, or error
            let lower = desc.to_lowercase();
            if lower.contains("version")
                || lower.contains("not found")
                || lower.contains("deprecated")
                || lower.starts_with("error")
                || desc.chars().filter(|c| *c == ' ').count() < 2
            {
                continue;
            }

            return Some(desc.to_string());
        }

        None
    }
}

impl PackageSource for ManualSource {
    fn name(&self) -> &'static str {
        "manual"
    }

    fn install_source(&self) -> InstallSource {
        InstallSource::Manual
    }

    fn scan(&self) -> Result<Vec<Tool>> {
        // Manual source doesn't scan - tools are added explicitly
        Ok(Vec::new())
    }

    fn fetch_description(&self, binary: &str) -> Option<String> {
        // Try man page first, then --help
        Self::fetch_man_description(binary).or_else(|| Self::fetch_help_description(binary))
    }

    fn install_command(&self, package: &str) -> String {
        format!("# Manual install required for {}", package)
    }

    fn uninstall_command(&self, package: &str) -> String {
        format!("# Manual uninstall required for {}", package)
    }
}
