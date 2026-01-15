//! Icon mappings for terminal display

/// Get icon for install source
pub fn source_icon(source: &str) -> &'static str {
    match source.to_lowercase().as_str() {
        "cargo" => "🦀",
        "pip" => "🐍",
        "npm" => "📦",
        "apt" => "🐧",
        "brew" => "🍺",
        "snap" => "📸",
        "flatpak" => "📦",
        "manual" => "🔧",
        _ => "📥",
    }
}

/// Get icon for tool status
pub fn status_icon(installed: bool) -> &'static str {
    if installed { "✓" } else { "✗" }
}

/// Get icon for category
pub fn category_icon(category: &str) -> &'static str {
    match category.to_lowercase().as_str() {
        "cli" | "shell" => "💻",
        "dev" | "development" => "🛠",
        "system" => "⚙",
        "network" | "net" => "🌐",
        "security" | "sec" => "🔒",
        "text" | "editor" => "📝",
        "search" => "🔍",
        "file" | "files" => "📁",
        "media" | "multimedia" => "🎬",
        "database" | "db" => "🗄",
        "container" | "docker" => "🐳",
        "cloud" => "☁",
        "terminal" => "🖥",
        "git" | "vcs" => "🔀",
        "test" | "testing" => "🧪",
        "build" => "🏗",
        "monitor" | "monitoring" => "📊",
        _ => "📌",
    }
}

/// Get icon for config status
pub fn config_status_icon(status: &str) -> &'static str {
    match status {
        "linked" => "🔗",
        "missing" => "❌",
        "conflict" => "⚠",
        "unlinked" => "◯",
        _ => "?",
    }
}

/// Print the icon legend
pub fn print_legend() {
    use colored::Colorize;

    println!();
    println!("{}", "Legend:".dimmed());
    println!(
        "  {} 🦀 cargo  🐍 pip  📦 npm  🐧 apt  🍺 brew  📸 snap  🔧 manual",
        "Sources:".dimmed()
    );
    println!(
        "  {} {} installed  {} missing",
        "Status:".dimmed(),
        "✓".green(),
        "✗".red()
    );
}

/// Print a compact legend (single line)
pub fn print_legend_compact() {
    use colored::Colorize;

    println!(
        "{} 🦀cargo 🐍pip 📦npm 🐧apt 🍺brew | {}installed {}missing",
        "".dimmed(),
        "✓".green(),
        "✗".red()
    );
}
