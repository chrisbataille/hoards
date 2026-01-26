use anyhow::Result;
use std::process::Command;

use crate::http::HTTP_AGENT;
use crate::models::{InstallSource, Tool};

/// Fetch package description from PyPI API
/// Returns None if the request fails or description is not available
pub fn fetch_pypi_description(package: &str) -> Option<String> {
    let url = format!("https://pypi.org/pypi/{}/json", package);
    let mut response = HTTP_AGENT.get(&url).call().ok()?;
    let json: serde_json::Value = response.body_mut().read_json().ok()?;

    let summary = json.get("info")?.get("summary")?.as_str()?;

    if summary.is_empty() || summary == "UNKNOWN" {
        None
    } else {
        Some(summary.to_string())
    }
}

/// Fetch package description from npm registry
/// Returns None if the request fails or description is not available
pub fn fetch_npm_description(package: &str) -> Option<String> {
    let url = format!("https://registry.npmjs.org/{}", package);
    let mut response = HTTP_AGENT.get(&url).call().ok()?;
    let json: serde_json::Value = response.body_mut().read_json().ok()?;

    json.get("description")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Fetch crate description from crates.io API
/// Returns None if the request fails or description is not available
pub fn fetch_crates_io_description(crate_name: &str) -> Option<String> {
    let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
    let mut response = HTTP_AGENT.get(&url).call().ok()?;
    let json: serde_json::Value = response.body_mut().read_json().ok()?;

    json.get("crate")?
        .get("description")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Fetch formula description from Homebrew API
/// Returns None if the request fails or description is not available
pub fn fetch_brew_description(formula: &str) -> Option<String> {
    let url = format!("https://formulae.brew.sh/api/formula/{}.json", formula);
    let mut response = HTTP_AGENT.get(&url).call().ok()?;
    let json: serde_json::Value = response.body_mut().read_json().ok()?;

    json.get("desc")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Extract description from man page NAME section
/// Format is typically: "tool - short description"
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
/// Tries to find a description line in common help formats
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
            || line.contains("[--")  // Option patterns
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

/// Known tools to scan for, organized by category
pub struct KnownTool {
    pub name: &'static str,
    pub binary: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub source: InstallSource,
    pub install_cmd: &'static str,
}

/// List of known CLI tools to scan for
pub static KNOWN_TOOLS: &[KnownTool] = &[
    // Modern CLI replacements
    KnownTool {
        name: "eza",
        binary: "eza",
        description: "Modern ls replacement with git integration",
        category: "files",
        source: InstallSource::Cargo,
        install_cmd: "cargo install eza",
    },
    KnownTool {
        name: "bat",
        binary: "bat",
        description: "Cat clone with syntax highlighting",
        category: "files",
        source: InstallSource::Cargo,
        install_cmd: "cargo install bat",
    },
    KnownTool {
        name: "ripgrep",
        binary: "rg",
        description: "Fast recursive grep",
        category: "search",
        source: InstallSource::Cargo,
        install_cmd: "cargo install ripgrep",
    },
    KnownTool {
        name: "fd",
        binary: "fd",
        description: "Fast find alternative",
        category: "search",
        source: InstallSource::Cargo,
        install_cmd: "cargo install fd-find",
    },
    KnownTool {
        name: "dust",
        binary: "dust",
        description: "Intuitive disk usage viewer",
        category: "system",
        source: InstallSource::Cargo,
        install_cmd: "cargo install du-dust",
    },
    KnownTool {
        name: "duf",
        binary: "duf",
        description: "Better df alternative",
        category: "system",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install duf",
    },
    KnownTool {
        name: "btop",
        binary: "btop",
        description: "Resource monitor",
        category: "system",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install btop",
    },
    KnownTool {
        name: "htop",
        binary: "htop",
        description: "Interactive process viewer",
        category: "system",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install htop",
    },
    KnownTool {
        name: "procs",
        binary: "procs",
        description: "Modern ps replacement",
        category: "system",
        source: InstallSource::Cargo,
        install_cmd: "cargo install procs",
    },
    KnownTool {
        name: "bottom",
        binary: "btm",
        description: "Graphical process/system monitor",
        category: "system",
        source: InstallSource::Cargo,
        install_cmd: "cargo install bottom",
    },
    KnownTool {
        name: "zoxide",
        binary: "zoxide",
        description: "Smarter cd command",
        category: "navigation",
        source: InstallSource::Cargo,
        install_cmd: "cargo install zoxide",
    },
    KnownTool {
        name: "fzf",
        binary: "fzf",
        description: "Fuzzy finder",
        category: "search",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install fzf",
    },
    KnownTool {
        name: "delta",
        binary: "delta",
        description: "Better git diff viewer",
        category: "git",
        source: InstallSource::Cargo,
        install_cmd: "cargo install git-delta",
    },
    KnownTool {
        name: "lazygit",
        binary: "lazygit",
        description: "Terminal UI for git",
        category: "git",
        source: InstallSource::Manual,
        install_cmd: "go install github.com/jesseduffield/lazygit@latest",
    },
    KnownTool {
        name: "lazydocker",
        binary: "lazydocker",
        description: "Terminal UI for docker",
        category: "docker",
        source: InstallSource::Manual,
        install_cmd: "go install github.com/jesseduffield/lazydocker@latest",
    },
    KnownTool {
        name: "tokei",
        binary: "tokei",
        description: "Code statistics",
        category: "dev",
        source: InstallSource::Cargo,
        install_cmd: "cargo install tokei",
    },
    KnownTool {
        name: "hyperfine",
        binary: "hyperfine",
        description: "Command-line benchmarking",
        category: "dev",
        source: InstallSource::Cargo,
        install_cmd: "cargo install hyperfine",
    },
    KnownTool {
        name: "just",
        binary: "just",
        description: "Modern make alternative",
        category: "dev",
        source: InstallSource::Cargo,
        install_cmd: "cargo install just",
    },
    KnownTool {
        name: "starship",
        binary: "starship",
        description: "Cross-shell prompt",
        category: "shell",
        source: InstallSource::Cargo,
        install_cmd: "cargo install starship",
    },
    KnownTool {
        name: "jq",
        binary: "jq",
        description: "JSON processor",
        category: "data",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install jq",
    },
    KnownTool {
        name: "yq",
        binary: "yq",
        description: "YAML processor",
        category: "data",
        source: InstallSource::Manual,
        install_cmd: "pip install yq",
    },
    KnownTool {
        name: "httpie",
        binary: "http",
        description: "Human-friendly HTTP client",
        category: "network",
        source: InstallSource::Pip,
        install_cmd: "pip install httpie",
    },
    KnownTool {
        name: "curlie",
        binary: "curlie",
        description: "Curl with httpie interface",
        category: "network",
        source: InstallSource::Cargo,
        install_cmd: "cargo install curlie",
    },
    KnownTool {
        name: "xh",
        binary: "xh",
        description: "Fast HTTP client",
        category: "network",
        source: InstallSource::Cargo,
        install_cmd: "cargo install xh",
    },
    KnownTool {
        name: "bandwhich",
        binary: "bandwhich",
        description: "Network utilization viewer",
        category: "network",
        source: InstallSource::Cargo,
        install_cmd: "cargo install bandwhich",
    },
    KnownTool {
        name: "dog",
        binary: "dog",
        description: "DNS lookup client",
        category: "network",
        source: InstallSource::Cargo,
        install_cmd: "cargo install dog",
    },
    KnownTool {
        name: "tldr",
        binary: "tldr",
        description: "Simplified man pages",
        category: "docs",
        source: InstallSource::Cargo,
        install_cmd: "cargo install tealdeer",
    },
    KnownTool {
        name: "glow",
        binary: "glow",
        description: "Markdown renderer",
        category: "docs",
        source: InstallSource::Manual,
        install_cmd: "go install github.com/charmbracelet/glow@latest",
    },
    KnownTool {
        name: "sd",
        binary: "sd",
        description: "Intuitive sed alternative",
        category: "text",
        source: InstallSource::Cargo,
        install_cmd: "cargo install sd",
    },
    KnownTool {
        name: "choose",
        binary: "choose",
        description: "Human-friendly cut",
        category: "text",
        source: InstallSource::Cargo,
        install_cmd: "cargo install choose",
    },
    // Shells
    KnownTool {
        name: "fish",
        binary: "fish",
        description: "Friendly interactive shell",
        category: "shell",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install fish",
    },
    KnownTool {
        name: "zsh",
        binary: "zsh",
        description: "Z shell",
        category: "shell",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install zsh",
    },
    KnownTool {
        name: "nushell",
        binary: "nu",
        description: "Modern shell with structured data",
        category: "shell",
        source: InstallSource::Cargo,
        install_cmd: "cargo install nu",
    },
    // Terminal emulators/multiplexers
    KnownTool {
        name: "alacritty",
        binary: "alacritty",
        description: "GPU-accelerated terminal",
        category: "terminal",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install alacritty",
    },
    KnownTool {
        name: "zellij",
        binary: "zellij",
        description: "Terminal multiplexer",
        category: "terminal",
        source: InstallSource::Cargo,
        install_cmd: "cargo install zellij",
    },
    KnownTool {
        name: "tmux",
        binary: "tmux",
        description: "Terminal multiplexer",
        category: "terminal",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install tmux",
    },
    KnownTool {
        name: "wezterm",
        binary: "wezterm",
        description: "GPU-accelerated terminal",
        category: "terminal",
        source: InstallSource::Manual,
        install_cmd: "flatpak install wezterm",
    },
    KnownTool {
        name: "kitty",
        binary: "kitty",
        description: "GPU-accelerated terminal",
        category: "terminal",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install kitty",
    },
    // Editors
    KnownTool {
        name: "neovim",
        binary: "nvim",
        description: "Hyperextensible Vim-based editor",
        category: "editor",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install neovim",
    },
    KnownTool {
        name: "helix",
        binary: "hx",
        description: "Post-modern modal editor",
        category: "editor",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install helix",
    },
    KnownTool {
        name: "micro",
        binary: "micro",
        description: "Modern terminal-based editor",
        category: "editor",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install micro",
    },
    // Version managers
    KnownTool {
        name: "rustup",
        binary: "rustup",
        description: "Rust toolchain manager",
        category: "lang",
        source: InstallSource::Manual,
        install_cmd: "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
    },
    KnownTool {
        name: "pyenv",
        binary: "pyenv",
        description: "Python version manager",
        category: "lang",
        source: InstallSource::Manual,
        install_cmd: "curl https://pyenv.run | bash",
    },
    KnownTool {
        name: "nvm",
        binary: "nvm",
        description: "Node version manager",
        category: "lang",
        source: InstallSource::Manual,
        install_cmd: "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.0/install.sh | bash",
    },
    KnownTool {
        name: "fnm",
        binary: "fnm",
        description: "Fast Node manager",
        category: "lang",
        source: InstallSource::Cargo,
        install_cmd: "cargo install fnm",
    },
    // Container/K8s
    KnownTool {
        name: "docker",
        binary: "docker",
        description: "Container runtime",
        category: "container",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install docker.io",
    },
    KnownTool {
        name: "podman",
        binary: "podman",
        description: "Daemonless container engine",
        category: "container",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install podman",
    },
    KnownTool {
        name: "kubectl",
        binary: "kubectl",
        description: "Kubernetes CLI",
        category: "container",
        source: InstallSource::Manual,
        install_cmd: "sudo snap install kubectl --classic",
    },
    KnownTool {
        name: "k9s",
        binary: "k9s",
        description: "Kubernetes TUI",
        category: "container",
        source: InstallSource::Manual,
        install_cmd: "go install github.com/derailed/k9s@latest",
    },
    KnownTool {
        name: "helm",
        binary: "helm",
        description: "Kubernetes package manager",
        category: "container",
        source: InstallSource::Manual,
        install_cmd: "sudo snap install helm --classic",
    },
    // Git tools
    KnownTool {
        name: "gh",
        binary: "gh",
        description: "GitHub CLI",
        category: "git",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install gh",
    },
    KnownTool {
        name: "git-lfs",
        binary: "git-lfs",
        description: "Git large file storage",
        category: "git",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install git-lfs",
    },
    KnownTool {
        name: "gitui",
        binary: "gitui",
        description: "Blazing fast git TUI",
        category: "git",
        source: InstallSource::Cargo,
        install_cmd: "cargo install gitui",
    },
    // Security
    KnownTool {
        name: "age",
        binary: "age",
        description: "Simple encryption tool",
        category: "security",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install age",
    },
    KnownTool {
        name: "git-crypt",
        binary: "git-crypt",
        description: "Git file encryption",
        category: "security",
        source: InstallSource::Apt,
        install_cmd: "sudo apt install git-crypt",
    },
];

/// Check if a binary is installed
pub fn is_installed(binary: &str) -> bool {
    which::which(binary).is_ok()
}

/// Scan system for known tools and return found ones
pub fn scan_known_tools() -> Vec<Tool> {
    KNOWN_TOOLS
        .iter()
        .filter(|kt| is_installed(kt.binary))
        .map(|kt| {
            Tool::new(kt.name)
                .with_source(kt.source.clone())
                .with_description(kt.description)
                .with_category(kt.category)
                .with_install_command(kt.install_cmd)
                .with_binary(kt.binary)
                .installed()
        })
        .collect()
}

/// Scan system for known tools and return NOT installed ones (suggestions)
pub fn scan_missing_tools() -> Vec<Tool> {
    KNOWN_TOOLS
        .iter()
        .filter(|kt| !is_installed(kt.binary))
        .map(|kt| {
            Tool::new(kt.name)
                .with_source(kt.source.clone())
                .with_description(kt.description)
                .with_category(kt.category)
                .with_install_command(kt.install_cmd)
                .with_binary(kt.binary)
        })
        .collect()
}

/// Scan cargo installed crates and return as Tools
/// Cargo packages are almost always CLI tools
pub fn scan_cargo_tools() -> Result<Vec<Tool>> {
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
                        .with_install_command(format!("cargo install {}", crate_name))
                        .installed();

                    // Fetch description from crates.io
                    if let Some(description) = fetch_crates_io_description(crate_name) {
                        tool = tool.with_description(description);
                    }

                    tools.push(tool);
                }
            }
        }
    }

    Ok(tools)
}

/// Scan pip installed packages that have CLI binaries
pub fn scan_pip_tools() -> Result<Vec<Tool>> {
    // Try pip3 first, then pip
    let output = Command::new("pip3")
        .args(["list", "--format=freeze"])
        .output()
        .or_else(|_| {
            Command::new("pip")
                .args(["list", "--format=freeze"])
                .output()
        })?;

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
            .with_install_command(format!("pip install {}", package))
            .installed();

        // Fetch description from PyPI
        if let Some(description) = fetch_pypi_description(&package) {
            tool = tool.with_description(description);
        }

        tools.push(tool);
    }

    Ok(tools)
}

/// Scan npm globally installed packages
pub fn scan_npm_tools() -> Result<Vec<Tool>> {
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
                .with_install_command(format!("npm install -g {}", package))
                .installed();

            // Fetch description from npm registry
            if let Some(description) = fetch_npm_description(package) {
                tool = tool.with_description(description);
            }

            tools.push(tool);
        }
    }

    Ok(tools)
}

/// Scan Homebrew/Linuxbrew installed packages
pub fn scan_brew_tools() -> Result<Vec<Tool>> {
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

        tools.push(
            Tool::new(package)
                .with_source(InstallSource::Brew)
                .with_binary(package)
                .with_category("cli")
                .with_install_command(format!("brew install {}", package))
                .installed(),
        );
    }

    Ok(tools)
}

/// Directories to scan in PATH for unknown binaries
const PATH_SCAN_DIRS: &[&str] = &[
    "/usr/local/bin",
    "/home/*/go/bin",
    "/home/*/.local/bin",
    "/home/*/.cargo/bin",
    "/opt/*/bin",
];

/// Binaries to skip (system utilities, not interesting to track)
const PATH_SKIP_BINARIES: &[&str] = &[
    ".",
    "..",
    "activate",
    "deactivate",
    "python",
    "python3",
    "pip",
    "pip3",
    "node",
    "npm",
    "npx",
    "cargo",
    "rustc",
    "rustup",
    "go",
    "gofmt",
];

/// Scan PATH directories for binaries not tracked by other package managers
pub fn scan_path_tools(tracked_binaries: &std::collections::HashSet<String>) -> Result<Vec<Tool>> {
    use std::os::unix::fs::PermissionsExt;

    let home = std::env::var("HOME").unwrap_or_default();
    let mut tools = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for pattern in PATH_SCAN_DIRS {
        let expanded = pattern.replace('*', home.trim_start_matches("/home/"));
        let expanded = if pattern.contains("/home/*") {
            pattern.replace("/home/*", &home)
        } else {
            expanded
        };

        let dir = std::path::Path::new(&expanded);
        if !dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Must be a file
            if !path.is_file() {
                continue;
            }

            // Must be executable
            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            if metadata.permissions().mode() & 0o111 == 0 {
                continue;
            }

            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };

            // Skip if already seen, tracked, or in skip list
            if seen.contains(&name) || tracked_binaries.contains(&name) {
                continue;
            }
            if PATH_SKIP_BINARIES.contains(&name.as_str()) {
                continue;
            }
            if KNOWN_TOOLS
                .iter()
                .any(|kt| kt.binary == name || kt.name == name)
            {
                continue;
            }

            // Determine source hint from path
            let source = if expanded.contains("/go/bin") {
                InstallSource::Manual // Go binary
            } else if expanded.contains("/.cargo/bin") {
                InstallSource::Cargo
            } else {
                InstallSource::Manual
            };

            let category = if expanded.contains("/go/bin") {
                "go"
            } else {
                "cli"
            };

            seen.insert(name.clone());
            tools.push(
                Tool::new(&name)
                    .with_source(source)
                    .with_binary(&name)
                    .with_category(category)
                    .installed(),
            );
        }
    }

    Ok(tools)
}

/// GUI-related apt sections to skip
const GUI_SECTIONS: &[&str] = &[
    "x11", "gnome", "kde", "xfce", "lxde", "lxqt", "mate", "cinnamon", "graphics", "video",
    "sound", "games", "fonts", "libdevel",
];

/// GUI-related dependencies to skip
const GUI_DEPS: &[&str] = &[
    "libgtk",
    "libqt",
    "libx11",
    "libwayland",
    "libgl",
    "libvulkan",
    "libsdl",
    "libegl",
    "libgdk",
    "libwx",
    "libfltk",
    "libcairo",
    "libpango",
    "libglib",
    "libgio",
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

/// Scan apt installed packages and return CLI tools only
pub fn scan_apt_tools() -> Result<Vec<Tool>> {
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
        if package.starts_with("lib") || package.ends_with("-dev") || package.ends_with("-doc") {
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
        if has_gui_dependencies(package) {
            continue;
        }

        let category = section_to_category(section);

        let mut tool = Tool::new(package)
            .with_source(InstallSource::Apt)
            .with_binary(package)
            .with_category(category)
            .with_install_command(format!("sudo apt install {}", package))
            .installed();

        if let Some(desc) = description
            && !desc.is_empty()
        {
            tool = tool.with_description(desc);
        }

        tools.push(tool);
    }

    Ok(tools)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_installed() {
        // These should exist on any system
        assert!(is_installed("ls"));
        assert!(is_installed("cat"));

        // This should not exist
        assert!(!is_installed("definitely_not_a_real_binary_12345"));
    }

    #[test]
    fn test_scan_known_tools() {
        let tools = scan_known_tools();
        // Should find at least some tools on any dev system
        println!("Found {} installed tools", tools.len());
        for tool in &tools {
            println!("  - {}", tool.name);
        }
    }
}
