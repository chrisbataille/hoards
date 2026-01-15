use anyhow::Result;
use std::process::Command;

/// An available update
#[derive(Debug)]
pub struct Update {
    pub name: String,
    pub current: String,
    pub latest: String,
    pub source: String,
}

/// Check for cargo updates using `cargo install --list` and crates.io
pub fn check_cargo_updates() -> Result<Vec<Update>> {
    let output = Command::new("cargo").args(["install", "--list"]).output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut updates = Vec::new();
    let mut current_crate: Option<(String, String)> = None;

    for line in stdout.lines() {
        if !line.starts_with(' ') {
            // Parse "crate_name v1.2.3:" format
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0].to_string();
                let version = parts[1]
                    .trim_start_matches('v')
                    .trim_end_matches(':')
                    .to_string();
                current_crate = Some((name, version));
            }
        } else if current_crate.is_some() {
            // We have a crate, check for update
            let (name, current_version) = current_crate.take().unwrap();

            // Query crates.io for latest version
            if let Ok(latest) = get_crates_io_version(&name)
                && latest != current_version
                && version_is_newer(&latest, &current_version)
            {
                updates.push(Update {
                    name,
                    current: current_version,
                    latest,
                    source: "cargo".to_string(),
                });
            }
        }
    }

    Ok(updates)
}

/// Get latest version from crates.io
fn get_crates_io_version(crate_name: &str) -> Result<String> {
    let output = Command::new("curl")
        .args([
            "-s",
            &format!("https://crates.io/api/v1/crates/{}", crate_name),
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to query crates.io");
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let version = json["crate"]["max_stable_version"]
        .as_str()
        .or_else(|| json["crate"]["max_version"].as_str())
        .ok_or_else(|| anyhow::anyhow!("No version found"))?;

    Ok(version.to_string())
}

/// Check for pip updates using `pip list --outdated`
pub fn check_pip_updates() -> Result<Vec<Update>> {
    let output = Command::new("pip3")
        .args(["list", "--outdated", "--format=json"])
        .output()
        .or_else(|_| {
            Command::new("pip")
                .args(["list", "--outdated", "--format=json"])
                .output()
        })?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let json: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)?;
    let mut updates = Vec::new();

    for pkg in json {
        if let (Some(name), Some(current), Some(latest)) = (
            pkg["name"].as_str(),
            pkg["version"].as_str(),
            pkg["latest_version"].as_str(),
        ) {
            updates.push(Update {
                name: name.to_string(),
                current: current.to_string(),
                latest: latest.to_string(),
                source: "pip".to_string(),
            });
        }
    }

    Ok(updates)
}

/// Check for npm updates using `npm outdated -g`
pub fn check_npm_updates() -> Result<Vec<Update>> {
    let output = Command::new("npm")
        .args(["outdated", "-g", "--json"])
        .output()?;

    // npm outdated returns exit code 1 if there are outdated packages
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() || stdout.trim() == "{}" {
        return Ok(Vec::new());
    }

    let json: serde_json::Value = serde_json::from_str(&stdout)?;
    let mut updates = Vec::new();

    if let Some(obj) = json.as_object() {
        for (name, info) in obj {
            if let (Some(current), Some(latest)) =
                (info["current"].as_str(), info["latest"].as_str())
                && current != latest
            {
                updates.push(Update {
                    name: name.to_string(),
                    current: current.to_string(),
                    latest: latest.to_string(),
                    source: "npm".to_string(),
                });
            }
        }
    }

    Ok(updates)
}

/// Check for apt updates using `apt list --upgradable`
pub fn check_apt_updates() -> Result<Vec<Update>> {
    let output = Command::new("apt")
        .args(["list", "--upgradable"])
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut updates = Vec::new();

    for line in stdout.lines().skip(1) {
        // Format: "package/source version arch [upgradable from: old_version]"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 6 {
            let name = parts[0].split('/').next().unwrap_or("");
            let latest = parts[1];
            // Extract current version from "[upgradable from: x.x.x]"
            if let Some(from_idx) = parts.iter().position(|&p| p == "from:")
                && let Some(current) = parts.get(from_idx + 1)
            {
                let current = current.trim_end_matches(']');
                updates.push(Update {
                    name: name.to_string(),
                    current: current.to_string(),
                    latest: latest.to_string(),
                    source: "apt".to_string(),
                });
            }
        }
    }

    Ok(updates)
}

/// Check for brew updates using `brew outdated`
pub fn check_brew_updates() -> Result<Vec<Update>> {
    let output = Command::new("brew").args(["outdated", "--json"]).output()?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let mut updates = Vec::new();

    if let Some(formulae) = json["formulae"].as_array() {
        for formula in formulae {
            if let (Some(name), Some(current), Some(latest)) = (
                formula["name"].as_str(),
                formula["installed_versions"]
                    .as_array()
                    .and_then(|v| v.first())
                    .and_then(|v| v.as_str()),
                formula["current_version"].as_str(),
            ) {
                updates.push(Update {
                    name: name.to_string(),
                    current: current.to_string(),
                    latest: latest.to_string(),
                    source: "brew".to_string(),
                });
            }
        }
    }

    Ok(updates)
}

/// A potential upgrade by switching sources
#[derive(Debug)]
pub struct CrossSourceUpgrade {
    pub name: String,
    pub current_version: String,
    pub current_source: String,
    pub better_version: String,
    pub better_source: String,
}

/// Get installed version of an apt package
pub fn get_apt_version(package: &str) -> Option<String> {
    let output = Command::new("dpkg-query")
        .args(["-W", "-f", "${Version}", package])
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Some(version);
        }
    }
    None
}

/// Get installed version of a cargo crate
pub fn get_cargo_version(crate_name: &str) -> Option<String> {
    let output = Command::new("cargo")
        .args(["install", "--list"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if !line.starts_with(' ') {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 && parts[0] == crate_name {
                return Some(
                    parts[1]
                        .trim_start_matches('v')
                        .trim_end_matches(':')
                        .to_string(),
                );
            }
        }
    }
    None
}

/// Get installed version of a pip package
pub fn get_pip_version(package: &str) -> Option<String> {
    let output = Command::new("pip3")
        .args(["show", package])
        .output()
        .or_else(|_| Command::new("pip").args(["show", package]).output())
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.starts_with("Version:") {
            return Some(line.trim_start_matches("Version:").trim().to_string());
        }
    }
    None
}

/// Get installed version of an npm package
pub fn get_npm_version(package: &str) -> Option<String> {
    let output = Command::new("npm")
        .args(["list", "-g", package, "--depth=0", "--json"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    json["dependencies"][package]["version"]
        .as_str()
        .map(|s| s.to_string())
}

/// Get installed version based on source
pub fn get_installed_version(name: &str, source: &str) -> Option<String> {
    match source {
        "cargo" => get_cargo_version(name),
        "pip" => get_pip_version(name),
        "npm" => get_npm_version(name),
        "apt" => get_apt_version(name),
        _ => None,
    }
}

/// Get all available newer versions based on source
pub fn get_available_versions(name: &str, source: &str, current: &str) -> Vec<String> {
    match source {
        "cargo" => get_crates_io_versions(name, current),
        "pip" => get_pypi_versions(name, current),
        "npm" => get_npm_versions(name, current),
        _ => Vec::new(),
    }
}

/// Get latest version from crates.io
pub fn get_crates_io_latest(crate_name: &str) -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "5",
            &format!("https://crates.io/api/v1/crates/{}", crate_name),
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    json["crate"]["max_stable_version"]
        .as_str()
        .or_else(|| json["crate"]["max_version"].as_str())
        .map(|s| s.to_string())
}

/// Get all versions from crates.io newer than the current version
pub fn get_crates_io_versions(crate_name: &str, current: &str) -> Vec<String> {
    let output = match Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "5",
            &format!("https://crates.io/api/v1/crates/{}", crate_name),
        ])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(j) => j,
        _ => return Vec::new(),
    };

    let mut versions: Vec<String> = json["versions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v["num"].as_str())
                .filter(|v| is_stable_version(v))
                .filter(|v| version_is_newer(v, current))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    // Sort versions (oldest first, so newest is at the end)
    versions.sort_by(|a, b| {
        if version_is_newer(a, b) {
            std::cmp::Ordering::Greater
        } else if version_is_newer(b, a) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    });

    versions
}

/// Get latest version from PyPI
pub fn get_pypi_latest(package: &str) -> Option<String> {
    let output = Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "5",
            &format!("https://pypi.org/pypi/{}/json", package),
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    json["info"]["version"].as_str().map(|s| s.to_string())
}

/// Get all versions from PyPI newer than the current version
pub fn get_pypi_versions(package: &str, current: &str) -> Vec<String> {
    let output = match Command::new("curl")
        .args([
            "-s",
            "--max-time",
            "5",
            &format!("https://pypi.org/pypi/{}/json", package),
        ])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(j) => j,
        _ => return Vec::new(),
    };

    let mut versions: Vec<String> = json["releases"]
        .as_object()
        .map(|obj| {
            obj.keys()
                .filter(|v| is_stable_version(v))
                .filter(|v| version_is_newer(v, current))
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    versions.sort_by(|a, b| {
        if version_is_newer(a, b) {
            std::cmp::Ordering::Greater
        } else if version_is_newer(b, a) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    });

    versions
}

/// Get latest version from npm registry
pub fn get_npm_latest(package: &str) -> Option<String> {
    let output = Command::new("npm")
        .args(["view", package, "version"])
        .output()
        .ok()?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Some(version);
        }
    }
    None
}

/// Get all versions from npm newer than the current version
pub fn get_npm_versions(package: &str, current: &str) -> Vec<String> {
    let output = match Command::new("npm")
        .args(["view", package, "versions", "--json"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let json: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(j) => j,
        _ => return Vec::new(),
    };

    let mut versions: Vec<String> = json
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter(|v| is_stable_version(v))
                .filter(|v| version_is_newer(v, current))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default();

    versions.sort_by(|a, b| {
        if version_is_newer(a, b) {
            std::cmp::Ordering::Greater
        } else if version_is_newer(b, a) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    });

    versions
}

/// Known mappings from apt package names to cargo crate names
fn apt_to_cargo_name(apt_name: &str) -> Option<&'static str> {
    match apt_name {
        "bat" => Some("bat"),
        "fd-find" => Some("fd-find"),
        "ripgrep" => Some("ripgrep"),
        "exa" | "eza" => Some("eza"),
        "dust" => Some("du-dust"),
        "procs" => Some("procs"),
        "bottom" => Some("bottom"),
        "zoxide" => Some("zoxide"),
        "starship" => Some("starship"),
        "delta" | "git-delta" => Some("git-delta"),
        "tokei" => Some("tokei"),
        "hyperfine" => Some("hyperfine"),
        "just" => Some("just"),
        "sd" => Some("sd"),
        "tealdeer" | "tldr" => Some("tealdeer"),
        "gitui" => Some("gitui"),
        "zellij" => Some("zellij"),
        "helix" | "hx" => Some("helix"),
        "alacritty" => Some("alacritty"),
        _ => None,
    }
}

/// Known mappings from apt package names to pip package names
fn apt_to_pip_name(apt_name: &str) -> Option<&'static str> {
    match apt_name {
        "httpie" => Some("httpie"),
        "youtube-dl" => Some("youtube-dl"),
        "yt-dlp" => Some("yt-dlp"),
        "black" => Some("black"),
        "ruff" => Some("ruff"),
        "mypy" => Some("mypy"),
        "pylint" => Some("pylint"),
        "ansible" => Some("ansible"),
        _ => None,
    }
}

/// Known mappings from apt package names to npm package names
fn apt_to_npm_name(apt_name: &str) -> Option<&'static str> {
    match apt_name {
        "prettier" => Some("prettier"),
        "eslint" => Some("eslint"),
        "typescript" => Some("typescript"),
        _ => None,
    }
}

/// Check if apt/snap tools have newer versions on other sources
pub fn check_cross_source_upgrades(tools: &[(String, String, String)]) -> Vec<CrossSourceUpgrade> {
    let mut upgrades = Vec::new();

    for (name, current_version, current_source) in tools {
        // Only check apt and snap tools
        if current_source != "apt" && current_source != "snap" {
            continue;
        }

        // Check cargo
        if let Some(cargo_name) = apt_to_cargo_name(name)
            && let Some(cargo_version) = get_crates_io_latest(cargo_name)
            && version_is_newer(&cargo_version, current_version)
        {
            upgrades.push(CrossSourceUpgrade {
                name: name.clone(),
                current_version: current_version.clone(),
                current_source: current_source.clone(),
                better_version: cargo_version,
                better_source: "cargo".to_string(),
            });
            continue; // Found an upgrade, skip other sources
        }

        // Check pip
        if let Some(pip_name) = apt_to_pip_name(name)
            && let Some(pip_version) = get_pypi_latest(pip_name)
            && version_is_newer(&pip_version, current_version)
        {
            upgrades.push(CrossSourceUpgrade {
                name: name.clone(),
                current_version: current_version.clone(),
                current_source: current_source.clone(),
                better_version: pip_version,
                better_source: "pip".to_string(),
            });
            continue;
        }

        // Check npm
        if let Some(npm_name) = apt_to_npm_name(name)
            && let Some(npm_version) = get_npm_latest(npm_name)
            && version_is_newer(&npm_version, current_version)
        {
            upgrades.push(CrossSourceUpgrade {
                name: name.clone(),
                current_version: current_version.clone(),
                current_source: current_source.clone(),
                better_version: npm_version,
                better_source: "npm".to_string(),
            });
        }
    }

    upgrades
}

/// Get migration candidates with optional source filtering
///
/// Wraps `check_cross_source_upgrades` with source filtering capability.
pub fn get_migration_candidates(
    tools: &[(String, String, String)],
    from_source: Option<&str>,
    to_source: Option<&str>,
) -> Vec<CrossSourceUpgrade> {
    let mut upgrades = check_cross_source_upgrades(tools);

    // Filter by from_source if specified
    if let Some(from) = from_source {
        upgrades.retain(|u| u.current_source == from);
    }

    // Filter by to_source if specified
    if let Some(to) = to_source {
        upgrades.retain(|u| u.better_source == to);
    }

    upgrades
}

/// Check if a version string is a stable release (not alpha, beta, rc, dev, etc.)
fn is_stable_version(v: &str) -> bool {
    // A stable version only contains digits, dots, and sometimes underscores
    // Pre-release versions contain letters like: 1.0a1, 1.0b2, 1.0rc1, 1.0.dev1, 1.0-alpha
    let lower = v.to_lowercase();

    // Check for common pre-release indicators
    if lower.contains("alpha")
        || lower.contains("beta")
        || lower.contains("dev")
        || lower.contains("pre")
    {
        return false;
    }

    // Check for "rc" but not in contexts like "src" (though unlikely in versions)
    if lower.contains("rc") && !lower.contains("src") {
        return false;
    }

    // Check for patterns like "1.0a1" or "1.0b2" (letter followed by digit)
    let chars: Vec<char> = lower.chars().collect();
    for i in 0..chars.len().saturating_sub(1) {
        if (chars[i] == 'a' || chars[i] == 'b') && chars[i + 1].is_ascii_digit() {
            // Check it's not part of a hex-like pattern (unlikely in versions)
            if i == 0 || !chars[i - 1].is_ascii_digit() {
                continue; // Probably okay
            }
            return false;
        }
    }

    true
}

/// Simple version comparison (assumes semver-like format)
pub fn version_is_newer(latest: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u32> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter_map(|p| p.parse().ok())
            .collect()
    };

    let latest_parts = parse(latest);
    let current_parts = parse(current);

    for (l, c) in latest_parts.iter().zip(current_parts.iter()) {
        if l > c {
            return true;
        }
        if l < c {
            return false;
        }
    }

    latest_parts.len() > current_parts.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_is_newer() {
        assert!(version_is_newer("1.2.0", "1.1.0"));
        assert!(version_is_newer("2.0.0", "1.9.9"));
        assert!(version_is_newer("1.0.1", "1.0.0"));
        assert!(!version_is_newer("1.0.0", "1.0.0"));
        assert!(!version_is_newer("1.0.0", "1.0.1"));
    }
}
