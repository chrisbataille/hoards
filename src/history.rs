//! Shell history parsing for usage tracking
//!
//! Parses history files from Fish, Bash, and Zsh to count tool usage.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Parsed command from history
#[derive(Debug)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: Option<i64>,
}

/// Get the path to Fish history file
pub fn fish_history_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("fish").join("fish_history"))
}

/// Get the path to Bash history file
pub fn bash_history_path() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".bash_history"))
}

/// Get the path to Zsh history file
pub fn zsh_history_path() -> Option<PathBuf> {
    dirs::home_dir().map(|d| d.join(".zsh_history"))
}

/// Parse Fish history file
/// Format: `- cmd: <command>\n  when: <timestamp>\n`
pub fn parse_fish_history(path: &PathBuf) -> Result<Vec<HistoryEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read fish history: {}", path.display()))?;

    let mut entries = Vec::new();
    let mut current_cmd: Option<String> = None;
    let mut current_time: Option<i64> = None;

    for line in content.lines() {
        if let Some(cmd) = line.strip_prefix("- cmd: ") {
            // Save previous entry if exists
            if let Some(cmd) = current_cmd.take() {
                entries.push(HistoryEntry {
                    command: cmd,
                    timestamp: current_time.take(),
                });
            }
            current_cmd = Some(cmd.to_string());
        } else if let Some(when) = line.strip_prefix("  when: ") {
            current_time = when.parse().ok();
        }
    }

    // Don't forget the last entry
    if let Some(cmd) = current_cmd {
        entries.push(HistoryEntry {
            command: cmd,
            timestamp: current_time,
        });
    }

    Ok(entries)
}

/// Parse Bash history file (simple format, one command per line)
pub fn parse_bash_history(path: &PathBuf) -> Result<Vec<HistoryEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read bash history: {}", path.display()))?;

    let entries = content
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| HistoryEntry {
            command: line.to_string(),
            timestamp: None,
        })
        .collect();

    Ok(entries)
}

/// Parse Zsh history file
/// Format can be: `<command>` or `: <timestamp>:<duration>;<command>`
pub fn parse_zsh_history(path: &PathBuf) -> Result<Vec<HistoryEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read zsh history: {}", path.display()))?;

    let entries = content
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            // Try to parse extended format: `: <timestamp>:<duration>;<command>`
            if line.starts_with(": ")
                && let Some(semicolon_pos) = line.find(';') {
                    let metadata = &line[2..semicolon_pos];
                    let command = &line[semicolon_pos + 1..];
                    let timestamp = metadata.split(':').next().and_then(|s| s.parse().ok());
                    return HistoryEntry {
                        command: command.to_string(),
                        timestamp,
                    };
                }
            // Simple format
            HistoryEntry {
                command: line.to_string(),
                timestamp: None,
            }
        })
        .collect();

    Ok(entries)
}

/// Extract the base command from a command line (first word, without path)
pub fn extract_command(line: &str) -> Option<&str> {
    let line = line.trim();

    // Skip empty lines or common prefixes
    if line.is_empty() {
        return None;
    }

    // Handle sudo, env, time, etc.
    let line = line
        .strip_prefix("sudo ")
        .or_else(|| line.strip_prefix("env "))
        .or_else(|| line.strip_prefix("time "))
        .or_else(|| line.strip_prefix("command "))
        .unwrap_or(line);

    // Get first word
    let cmd = line.split_whitespace().next()?;

    // Remove path prefix (e.g., /usr/bin/git -> git)
    let cmd = cmd.rsplit('/').next().unwrap_or(cmd);

    // Skip shell builtins and common non-tools
    let skip = [
        "cd", "ls", "echo", "export", "set", "unset", "alias", "source",
        "if", "then", "else", "fi", "for", "do", "done", "while", "case",
        "esac", "function", "return", "exit", "true", "false", "test",
        "[", "[[", "pwd", "pushd", "popd", "dirs", "history", "clear",
    ];

    if skip.contains(&cmd) {
        return None;
    }

    Some(cmd)
}

/// Count command usage from history entries
pub fn count_commands(entries: &[HistoryEntry]) -> HashMap<String, i64> {
    let mut counts: HashMap<String, i64> = HashMap::new();

    for entry in entries {
        if let Some(cmd) = extract_command(&entry.command) {
            *counts.entry(cmd.to_string()).or_insert(0) += 1;
        }
    }

    counts
}

/// Parse all available shell histories and combine counts
pub fn parse_all_histories() -> Result<HashMap<String, i64>> {
    let mut total_counts: HashMap<String, i64> = HashMap::new();

    // Try Fish history
    if let Some(path) = fish_history_path()
        && path.exists() {
            match parse_fish_history(&path) {
                Ok(entries) => {
                    let counts = count_commands(&entries);
                    for (cmd, count) in counts {
                        *total_counts.entry(cmd).or_insert(0) += count;
                    }
                }
                Err(e) => eprintln!("Warning: Failed to parse fish history: {}", e),
            }
        }

    // Try Bash history
    if let Some(path) = bash_history_path()
        && path.exists() {
            match parse_bash_history(&path) {
                Ok(entries) => {
                    let counts = count_commands(&entries);
                    for (cmd, count) in counts {
                        *total_counts.entry(cmd).or_insert(0) += count;
                    }
                }
                Err(e) => eprintln!("Warning: Failed to parse bash history: {}", e),
            }
        }

    // Try Zsh history
    if let Some(path) = zsh_history_path()
        && path.exists() {
            match parse_zsh_history(&path) {
                Ok(entries) => {
                    let counts = count_commands(&entries);
                    for (cmd, count) in counts {
                        *total_counts.entry(cmd).or_insert(0) += count;
                    }
                }
                Err(e) => eprintln!("Warning: Failed to parse zsh history: {}", e),
            }
        }

    Ok(total_counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // ==================== extract_command Tests ====================

    #[test]
    fn test_extract_command_basic() {
        assert_eq!(extract_command("git status"), Some("git"));
        assert_eq!(extract_command("cargo build"), Some("cargo"));
        assert_eq!(extract_command("rg pattern file.rs"), Some("rg"));
    }

    #[test]
    fn test_extract_command_with_sudo() {
        assert_eq!(extract_command("sudo apt update"), Some("apt"));
        assert_eq!(extract_command("sudo docker ps"), Some("docker"));
    }

    #[test]
    fn test_extract_command_with_env() {
        // Note: Current implementation strips "env " but doesn't skip VAR=value args
        // This tests actual behavior - could be enhanced in future
        assert_eq!(extract_command("env cargo test"), Some("cargo"));
    }

    #[test]
    fn test_extract_command_with_time() {
        assert_eq!(extract_command("time cargo build --release"), Some("cargo"));
    }

    #[test]
    fn test_extract_command_with_path() {
        assert_eq!(extract_command("/usr/bin/rg pattern"), Some("rg"));
        assert_eq!(extract_command("/home/user/.cargo/bin/cargo build"), Some("cargo"));
        assert_eq!(extract_command("./local-script.sh"), Some("local-script.sh"));
    }

    #[test]
    fn test_extract_command_skips_builtins() {
        assert_eq!(extract_command("cd /tmp"), None);
        assert_eq!(extract_command("echo hello"), None);
        assert_eq!(extract_command("export PATH=$PATH:/foo"), None);
        assert_eq!(extract_command("if true"), None);
        assert_eq!(extract_command("for i in 1 2 3"), None);
    }

    #[test]
    fn test_extract_command_empty() {
        assert_eq!(extract_command(""), None);
        assert_eq!(extract_command("   "), None);
    }

    #[test]
    fn test_extract_command_whitespace() {
        assert_eq!(extract_command("  git status  "), Some("git"));
        assert_eq!(extract_command("\tfd pattern"), Some("fd"));
    }

    // ==================== count_commands Tests ====================

    #[test]
    fn test_count_commands() {
        let entries = vec![
            HistoryEntry { command: "git status".to_string(), timestamp: None },
            HistoryEntry { command: "git commit".to_string(), timestamp: None },
            HistoryEntry { command: "rg pattern".to_string(), timestamp: None },
            HistoryEntry { command: "git push".to_string(), timestamp: None },
        ];

        let counts = count_commands(&entries);
        assert_eq!(counts.get("git"), Some(&3));
        assert_eq!(counts.get("rg"), Some(&1));
    }

    #[test]
    fn test_count_commands_empty() {
        let entries: Vec<HistoryEntry> = vec![];
        let counts = count_commands(&entries);
        assert!(counts.is_empty());
    }

    #[test]
    fn test_count_commands_only_builtins() {
        let entries = vec![
            HistoryEntry { command: "cd /tmp".to_string(), timestamp: None },
            HistoryEntry { command: "echo hello".to_string(), timestamp: None },
        ];
        let counts = count_commands(&entries);
        assert!(counts.is_empty());
    }

    // ==================== Fish History Parsing Tests ====================

    #[test]
    fn test_parse_fish_history() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "- cmd: git status")?;
        writeln!(file, "  when: 1704067200")?;
        writeln!(file, "- cmd: cargo build")?;
        writeln!(file, "  when: 1704067300")?;
        writeln!(file, "- cmd: rg pattern")?;
        file.flush()?;

        let path = file.path().to_path_buf();
        let entries = parse_fish_history(&path)?;

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].command, "git status");
        assert_eq!(entries[0].timestamp, Some(1704067200));
        assert_eq!(entries[1].command, "cargo build");
        assert_eq!(entries[2].command, "rg pattern");
        assert!(entries[2].timestamp.is_none());

        Ok(())
    }

    #[test]
    fn test_parse_fish_history_empty() -> Result<()> {
        let file = NamedTempFile::new()?;
        let path = file.path().to_path_buf();
        let entries = parse_fish_history(&path)?;
        assert!(entries.is_empty());
        Ok(())
    }

    // ==================== Bash History Parsing Tests ====================

    #[test]
    fn test_parse_bash_history() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "git status")?;
        writeln!(file, "cargo build")?;
        writeln!(file, "# comment line")?;
        writeln!(file, "rg pattern")?;
        writeln!(file)?; // empty line
        file.flush()?;

        let path = file.path().to_path_buf();
        let entries = parse_bash_history(&path)?;

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].command, "git status");
        assert_eq!(entries[1].command, "cargo build");
        assert_eq!(entries[2].command, "rg pattern");
        // Bash history doesn't have timestamps
        assert!(entries[0].timestamp.is_none());

        Ok(())
    }

    // ==================== Zsh History Parsing Tests ====================

    #[test]
    fn test_parse_zsh_history_simple() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "git status")?;
        writeln!(file, "cargo build")?;
        file.flush()?;

        let path = file.path().to_path_buf();
        let entries = parse_zsh_history(&path)?;

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "git status");
        assert!(entries[0].timestamp.is_none());

        Ok(())
    }

    #[test]
    fn test_parse_zsh_history_extended_format() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, ": 1704067200:0;git status")?;
        writeln!(file, ": 1704067300:5;cargo build --release")?;
        file.flush()?;

        let path = file.path().to_path_buf();
        let entries = parse_zsh_history(&path)?;

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "git status");
        assert_eq!(entries[0].timestamp, Some(1704067200));
        assert_eq!(entries[1].command, "cargo build --release");
        assert_eq!(entries[1].timestamp, Some(1704067300));

        Ok(())
    }

    #[test]
    fn test_parse_zsh_history_mixed_format() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, ": 1704067200:0;git status")?;
        writeln!(file, "simple command")?;
        writeln!(file, ": 1704067300:0;cargo build")?;
        file.flush()?;

        let path = file.path().to_path_buf();
        let entries = parse_zsh_history(&path)?;

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].timestamp, Some(1704067200));
        assert!(entries[1].timestamp.is_none());
        assert_eq!(entries[2].timestamp, Some(1704067300));

        Ok(())
    }

    // ==================== Path Functions Tests ====================

    #[test]
    fn test_history_paths_return_some() {
        // These should return Some on most systems
        // We can't test exact paths as they're system-dependent
        let fish = fish_history_path();
        let bash = bash_history_path();
        let zsh = zsh_history_path();

        // At minimum, if dirs crate works, these should return Some
        // (even if the files don't exist)
        assert!(fish.is_some() || bash.is_some() || zsh.is_some());
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_full_pipeline() -> Result<()> {
        // Create a fish-style history file
        let mut file = NamedTempFile::new()?;
        writeln!(file, "- cmd: git status")?;
        writeln!(file, "  when: 1704067200")?;
        writeln!(file, "- cmd: git commit -m 'test'")?;
        writeln!(file, "  when: 1704067300")?;
        writeln!(file, "- cmd: cargo build")?;
        writeln!(file, "  when: 1704067400")?;
        writeln!(file, "- cmd: git push")?;
        writeln!(file, "  when: 1704067500")?;
        writeln!(file, "- cmd: cd /tmp")?; // should be filtered out
        writeln!(file, "  when: 1704067600")?;
        file.flush()?;

        let path = file.path().to_path_buf();
        let entries = parse_fish_history(&path)?;
        let counts = count_commands(&entries);

        assert_eq!(counts.get("git"), Some(&3));
        assert_eq!(counts.get("cargo"), Some(&1));
        assert!(!counts.contains_key("cd")); // filtered out

        Ok(())
    }
}
