use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// AI provider options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    Claude,
    Gemini,
    Codex,
    Opencode,
    None,
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
}

/// Hoard configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HoardConfig {
    #[serde(default)]
    pub ai: AiConfig,
}

/// AI-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: AiProvider,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::None,
        }
    }
}

impl HoardConfig {
    /// Get the config file path
    pub fn config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .context("Could not determine config directory")?
            .join("hoard");

        Ok(config_dir.join("config.toml"))
    }

    /// Load config from file, or return default if not exists
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .context("Failed to read config file")?;

        let config: HoardConfig = toml::from_str(&content)
            .context("Failed to parse config file")?;

        Ok(config)
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self)
            .context("Failed to serialize config")?;

        std::fs::write(&path, content)
            .context("Failed to write config file")?;

        Ok(())
    }

    /// Set AI provider
    pub fn set_ai_provider(&mut self, provider: AiProvider) {
        self.ai.provider = provider;
    }
}
