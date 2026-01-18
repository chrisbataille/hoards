use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// AI provider options
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    #[default]
    None,
    Claude,
    Gemini,
    Codex,
    Opencode,
}

/// Claude model options (when using Claude as AI provider)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeModel {
    /// Claude Haiku - fastest and most cost-effective
    #[default]
    Haiku,
    /// Claude Sonnet - balanced intelligence and speed  
    Sonnet,
    /// Claude Opus - most capable model
    Opus,
}

impl ClaudeModel {
    /// Get the model alias for the Claude CLI
    pub fn as_cli_arg(&self) -> &'static str {
        match self {
            Self::Haiku => "haiku",
            Self::Sonnet => "sonnet",
            Self::Opus => "opus",
        }
    }
}

impl std::fmt::Display for ClaudeModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Haiku => write!(f, "haiku"),
            Self::Sonnet => write!(f, "sonnet"),
            Self::Opus => write!(f, "opus"),
        }
    }
}

impl From<&str> for ClaudeModel {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sonnet" => Self::Sonnet,
            "opus" => Self::Opus,
            _ => Self::Haiku, // Default to haiku
        }
    }
}

impl std::fmt::Display for AiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Claude => write!(f, "claude"),
            Self::Gemini => write!(f, "gemini"),
            Self::Codex => write!(f, "codex"),
            Self::Opencode => write!(f, "opencode"),
            Self::None => write!(f, "none"),
        }
    }
}

impl From<&str> for AiProvider {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "claude" => Self::Claude,
            "gemini" => Self::Gemini,
            "codex" => Self::Codex,
            "opencode" | "open-code" => Self::Opencode,
            _ => Self::None,
        }
    }
}

impl AiProvider {
    /// Get the CLI command for this provider
    pub fn command(&self) -> Option<&'static str> {
        match self {
            Self::Claude => Some("claude"),
            Self::Gemini => Some("gemini"),
            Self::Codex => Some("codex"),
            Self::Opencode => Some("opencode"),
            Self::None => None,
        }
    }

    /// Check if the CLI tool is installed
    pub fn is_installed(&self) -> bool {
        if let Some(cmd) = self.command() {
            which::which(cmd).is_ok()
        } else {
            false
        }
    }

    /// Get all available providers
    pub fn all() -> &'static [AiProvider] {
        &[
            AiProvider::None,
            AiProvider::Claude,
            AiProvider::Gemini,
            AiProvider::Codex,
            AiProvider::Opencode,
        ]
    }
}

/// Usage tracking mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum UsageMode {
    /// Manual history scanning with `usage scan`
    #[default]
    Scan,
    /// Real-time tracking via shell hooks
    Hook,
}

impl std::fmt::Display for UsageMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scan => write!(f, "scan"),
            Self::Hook => write!(f, "hook"),
        }
    }
}

/// Usage tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageConfig {
    /// Tracking mode: scan (manual) or hook (automatic)
    #[serde(default)]
    pub mode: UsageMode,
    /// Shell for hook mode (fish, bash, zsh)
    pub shell: Option<String>,
}

/// AI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AiConfig {
    #[serde(default)]
    pub provider: AiProvider,
    /// Claude model to use (haiku, sonnet, opus) - only applies when provider is Claude
    #[serde(default)]
    pub claude_model: ClaudeModel,
}

/// TUI theme options
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TuiTheme {
    #[default]
    CatppuccinMocha,
    CatppuccinLatte,
    Dracula,
    Nord,
    TokyoNight,
    Gruvbox,
    Custom,
}

impl std::fmt::Display for TuiTheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CatppuccinMocha => write!(f, "Catppuccin Mocha"),
            Self::CatppuccinLatte => write!(f, "Catppuccin Latte"),
            Self::Dracula => write!(f, "Dracula"),
            Self::Nord => write!(f, "Nord"),
            Self::TokyoNight => write!(f, "Tokyo Night"),
            Self::Gruvbox => write!(f, "Gruvbox"),
            Self::Custom => write!(f, "Custom"),
        }
    }
}

impl TuiTheme {
    /// Get all available themes (includes Custom only if custom theme file exists)
    pub fn all() -> Vec<TuiTheme> {
        let mut themes = vec![
            TuiTheme::CatppuccinMocha,
            TuiTheme::CatppuccinLatte,
            TuiTheme::Dracula,
            TuiTheme::Nord,
            TuiTheme::TokyoNight,
            TuiTheme::Gruvbox,
        ];
        // Add Custom if the custom theme file exists
        if crate::tui::theme::CustomTheme::exists() {
            themes.push(TuiTheme::Custom);
        }
        themes
    }

    /// Convert to index (for cycling)
    pub fn index(&self) -> usize {
        match self {
            Self::CatppuccinMocha => 0,
            Self::CatppuccinLatte => 1,
            Self::Dracula => 2,
            Self::Nord => 3,
            Self::TokyoNight => 4,
            Self::Gruvbox => 5,
            Self::Custom => 6,
        }
    }

    /// Create from index (for cycling)
    pub fn from_index(idx: usize) -> Self {
        // Always support all 7 themes (6 built-in + Custom)
        match idx % 7 {
            0 => Self::CatppuccinMocha,
            1 => Self::CatppuccinLatte,
            2 => Self::Dracula,
            3 => Self::Nord,
            4 => Self::TokyoNight,
            5 => Self::Gruvbox,
            _ => Self::Custom,
        }
    }
}

/// TUI configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TuiConfig {
    #[serde(default)]
    pub theme: TuiTheme,
}

/// Package source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    #[serde(default = "default_true")]
    pub cargo: bool,
    #[serde(default = "default_true")]
    pub apt: bool,
    #[serde(default)]
    pub pip: bool,
    #[serde(default)]
    pub npm: bool,
    #[serde(default)]
    pub brew: bool,
    #[serde(default = "default_true")]
    pub flatpak: bool,
    #[serde(default = "default_true")]
    pub manual: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            cargo: true,
            apt: true,
            pip: false,
            npm: false,
            brew: false,
            flatpak: true,
            manual: true,
        }
    }
}

impl SourcesConfig {
    /// Get list of enabled source names
    pub fn enabled_sources(&self) -> Vec<&'static str> {
        let mut sources = Vec::new();
        if self.cargo {
            sources.push("cargo");
        }
        if self.apt {
            sources.push("apt");
        }
        if self.pip {
            sources.push("pip");
        }
        if self.npm {
            sources.push("npm");
        }
        if self.brew {
            sources.push("brew");
        }
        if self.flatpak {
            sources.push("flatpak");
        }
        if self.manual {
            sources.push("manual");
        }
        sources
    }

    /// Check if a source is enabled by name
    pub fn is_enabled(&self, source: &str) -> bool {
        match source.to_lowercase().as_str() {
            "cargo" => self.cargo,
            "apt" => self.apt,
            "pip" => self.pip,
            "npm" => self.npm,
            "brew" => self.brew,
            "flatpak" => self.flatpak,
            "manual" => self.manual,
            _ => false,
        }
    }

    /// Toggle a source by name
    pub fn toggle(&mut self, source: &str) {
        match source.to_lowercase().as_str() {
            "cargo" => self.cargo = !self.cargo,
            "apt" => self.apt = !self.apt,
            "pip" => self.pip = !self.pip,
            "npm" => self.npm = !self.npm,
            "brew" => self.brew = !self.brew,
            "flatpak" => self.flatpak = !self.flatpak,
            "manual" => self.manual = !self.manual,
            _ => {}
        }
    }

    /// Get all source names
    pub fn all_sources() -> &'static [&'static str] {
        &["cargo", "apt", "pip", "npm", "brew", "flatpak", "manual"]
    }
}

/// Hoard configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HoardConfig {
    /// JSON Schema reference (optional, for editor support)
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    #[serde(default)]
    pub ai: AiConfig,

    #[serde(default)]
    pub usage: UsageConfig,

    #[serde(default)]
    pub tui: TuiConfig,

    #[serde(default)]
    pub sources: SourcesConfig,
}

impl HoardConfig {
    /// Get the config directory path
    pub fn config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .context("Could not determine config directory")
            .map(|d| d.join("hoards"))
    }

    /// Get the JSON config file path
    pub fn config_path() -> Result<PathBuf> {
        Self::config_dir().map(|d| d.join("config.json"))
    }

    /// Get the legacy TOML config file path (for migration)
    fn legacy_config_path() -> Result<PathBuf> {
        Self::config_dir().map(|d| d.join("config.toml"))
    }

    /// Check if config file exists
    pub fn exists() -> bool {
        Self::config_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Load config from file, or return default if not exists
    /// Handles migration from TOML to JSON automatically
    pub fn load() -> Result<Self> {
        let json_path = Self::config_path()?;
        let toml_path = Self::legacy_config_path()?;

        // If JSON config exists, load it
        if json_path.exists() {
            let content =
                std::fs::read_to_string(&json_path).context("Failed to read config file")?;
            let config: HoardConfig =
                serde_json::from_str(&content).context("Failed to parse config file")?;
            return Ok(config);
        }

        // If legacy TOML exists, migrate it
        if toml_path.exists() {
            let content =
                std::fs::read_to_string(&toml_path).context("Failed to read legacy config file")?;
            let legacy: LegacyHoardConfig =
                toml::from_str(&content).context("Failed to parse legacy config file")?;

            // Convert to new format
            let config = HoardConfig {
                schema: Some(
                    "https://raw.githubusercontent.com/chrisbataille/hoards/main/schema/config.schema.json"
                        .to_string(),
                ),
                ai: legacy.ai,
                usage: UsageConfig {
                    mode: legacy.usage.mode.unwrap_or_default(),
                    shell: legacy.usage.shell,
                },
                tui: TuiConfig::default(),
                sources: SourcesConfig::default(),
            };

            // Save as JSON
            config.save()?;

            // Optionally backup and remove TOML (keep backup for safety)
            let backup_path = toml_path.with_extension("toml.bak");
            std::fs::rename(&toml_path, &backup_path).ok();

            return Ok(config);
        }

        // No config exists, return default
        Ok(Self::default())
    }

    /// Save config to JSON file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        // Add schema reference if not set
        let mut config = self.clone();
        if config.schema.is_none() {
            config.schema = Some(
                "https://raw.githubusercontent.com/chrisbataille/hoards/main/schema/config.schema.json"
                    .to_string(),
            );
        }

        let content =
            serde_json::to_string_pretty(&config).context("Failed to serialize config")?;

        std::fs::write(&path, content).context("Failed to write config file")?;

        Ok(())
    }

    /// Set AI provider
    pub fn set_ai_provider(&mut self, provider: AiProvider) {
        self.ai.provider = provider;
    }

    /// Set TUI theme
    pub fn set_theme(&mut self, theme: TuiTheme) {
        self.tui.theme = theme;
    }

    /// Set usage mode
    pub fn set_usage_mode(&mut self, mode: UsageMode) {
        self.usage.mode = mode;
    }
}

/// Legacy TOML config structure (for migration)
#[derive(Debug, Clone, Deserialize)]
struct LegacyHoardConfig {
    #[serde(default)]
    ai: AiConfig,
    #[serde(default)]
    usage: LegacyUsageConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct LegacyUsageConfig {
    mode: Option<UsageMode>,
    shell: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_provider_display() {
        assert_eq!(AiProvider::Claude.to_string(), "claude");
        assert_eq!(AiProvider::None.to_string(), "none");
    }

    #[test]
    fn test_ai_provider_from_str() {
        assert_eq!(AiProvider::from("claude"), AiProvider::Claude);
        assert_eq!(AiProvider::from("GEMINI"), AiProvider::Gemini);
        assert_eq!(AiProvider::from("unknown"), AiProvider::None);
    }

    #[test]
    fn test_theme_cycling() {
        let theme = TuiTheme::CatppuccinMocha;
        assert_eq!(theme.index(), 0);
        assert_eq!(TuiTheme::from_index(0), TuiTheme::CatppuccinMocha);
        assert_eq!(TuiTheme::from_index(6), TuiTheme::Custom); // Custom at index 6
        assert_eq!(TuiTheme::from_index(7), TuiTheme::CatppuccinMocha); // Wraps at 7
    }

    #[test]
    fn test_sources_config_enabled() {
        let config = SourcesConfig::default();
        assert!(config.is_enabled("cargo"));
        assert!(config.is_enabled("apt"));
        assert!(!config.is_enabled("pip"));
        assert!(!config.is_enabled("npm"));
    }

    #[test]
    fn test_sources_config_toggle() {
        let mut config = SourcesConfig::default();
        assert!(config.cargo);
        config.toggle("cargo");
        assert!(!config.cargo);
        config.toggle("cargo");
        assert!(config.cargo);
    }

    #[test]
    fn test_sources_enabled_list() {
        let config = SourcesConfig::default();
        let enabled = config.enabled_sources();
        assert!(enabled.contains(&"cargo"));
        assert!(enabled.contains(&"apt"));
        assert!(!enabled.contains(&"pip"));
    }

    #[test]
    fn test_json_serialization() {
        let config = HoardConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(json.contains("\"provider\":"));
        assert!(json.contains("\"theme\":"));
    }

    #[test]
    fn test_json_deserialization() {
        let json = r#"{
            "ai": { "provider": "claude" },
            "tui": { "theme": "dracula" },
            "sources": { "cargo": true, "pip": true }
        }"#;
        let config: HoardConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.ai.provider, AiProvider::Claude);
        assert_eq!(config.tui.theme, TuiTheme::Dracula);
        assert!(config.sources.cargo);
        assert!(config.sources.pip);
    }
}
