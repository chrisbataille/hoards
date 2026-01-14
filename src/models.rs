use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Installation source for a tool
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstallSource {
    Cargo,
    Apt,
    Snap,
    Flatpak,
    Npm,
    Pip,
    Brew,
    Manual,
    Unknown,
}

impl std::fmt::Display for InstallSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cargo => write!(f, "cargo"),
            Self::Apt => write!(f, "apt"),
            Self::Snap => write!(f, "snap"),
            Self::Flatpak => write!(f, "flatpak"),
            Self::Npm => write!(f, "npm"),
            Self::Pip => write!(f, "pip"),
            Self::Brew => write!(f, "brew"),
            Self::Manual => write!(f, "manual"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl From<&str> for InstallSource {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "cargo" => Self::Cargo,
            "apt" => Self::Apt,
            "snap" => Self::Snap,
            "flatpak" => Self::Flatpak,
            "npm" => Self::Npm,
            "pip" => Self::Pip,
            "brew" => Self::Brew,
            "manual" => Self::Manual,
            _ => Self::Unknown,
        }
    }
}

/// A tool tracked by hoard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub source: InstallSource,
    pub install_command: Option<String>,
    pub binary_name: Option<String>,
    pub is_installed: bool,
    pub is_favorite: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Tool {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            name: name.into(),
            description: None,
            category: None,
            source: InstallSource::Unknown,
            install_command: None,
            binary_name: None,
            is_installed: false,
            is_favorite: false,
            notes: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_source(mut self, source: InstallSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_category(mut self, cat: impl Into<String>) -> Self {
        self.category = Some(cat.into());
        self
    }

    pub fn with_install_command(mut self, cmd: impl Into<String>) -> Self {
        self.install_command = Some(cmd.into());
        self
    }

    pub fn with_binary(mut self, bin: impl Into<String>) -> Self {
        self.binary_name = Some(bin.into());
        self
    }

    pub fn installed(mut self) -> Self {
        self.is_installed = true;
        self
    }
}

/// An interest category for AI-assisted discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interest {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
}

impl Interest {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            description: None,
            priority: 0,
            created_at: Utc::now(),
        }
    }
}

/// A config file tracked by hoard (links to dotfiles)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub id: Option<i64>,
    pub name: String,
    pub source_path: String,
    pub target_path: String,
    pub tool_id: Option<i64>,
    pub is_symlinked: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Config {
    pub fn new(name: impl Into<String>, source: impl Into<String>, target: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            name: name.into(),
            source_path: source.into(),
            target_path: target.into(),
            tool_id: None,
            is_symlinked: false,
            created_at: now,
            updated_at: now,
        }
    }
}

/// A bundle of tools that can be installed together
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub tools: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl Bundle {
    pub fn new(name: impl Into<String>, tools: Vec<String>) -> Self {
        Self {
            id: None,
            name: name.into(),
            description: None,
            tools,
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== InstallSource Tests ====================

    #[test]
    fn test_install_source_display() {
        assert_eq!(InstallSource::Cargo.to_string(), "cargo");
        assert_eq!(InstallSource::Apt.to_string(), "apt");
        assert_eq!(InstallSource::Snap.to_string(), "snap");
        assert_eq!(InstallSource::Flatpak.to_string(), "flatpak");
        assert_eq!(InstallSource::Npm.to_string(), "npm");
        assert_eq!(InstallSource::Pip.to_string(), "pip");
        assert_eq!(InstallSource::Brew.to_string(), "brew");
        assert_eq!(InstallSource::Manual.to_string(), "manual");
        assert_eq!(InstallSource::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_install_source_from_str() {
        assert_eq!(InstallSource::from("cargo"), InstallSource::Cargo);
        assert_eq!(InstallSource::from("CARGO"), InstallSource::Cargo); // case insensitive
        assert_eq!(InstallSource::from("Cargo"), InstallSource::Cargo);
        assert_eq!(InstallSource::from("apt"), InstallSource::Apt);
        assert_eq!(InstallSource::from("snap"), InstallSource::Snap);
        assert_eq!(InstallSource::from("flatpak"), InstallSource::Flatpak);
        assert_eq!(InstallSource::from("FLATPAK"), InstallSource::Flatpak);
        assert_eq!(InstallSource::from("npm"), InstallSource::Npm);
        assert_eq!(InstallSource::from("pip"), InstallSource::Pip);
        assert_eq!(InstallSource::from("brew"), InstallSource::Brew);
        assert_eq!(InstallSource::from("manual"), InstallSource::Manual);
        assert_eq!(InstallSource::from("unknown"), InstallSource::Unknown);
        assert_eq!(InstallSource::from("garbage"), InstallSource::Unknown);
        assert_eq!(InstallSource::from(""), InstallSource::Unknown);
    }

    #[test]
    fn test_install_source_roundtrip() {
        // Verify to_string -> from roundtrip
        let sources = [
            InstallSource::Cargo,
            InstallSource::Apt,
            InstallSource::Snap,
            InstallSource::Flatpak,
            InstallSource::Npm,
            InstallSource::Pip,
            InstallSource::Brew,
            InstallSource::Manual,
            InstallSource::Unknown,
        ];
        for source in sources {
            let s = source.to_string();
            assert_eq!(InstallSource::from(s.as_str()), source);
        }
    }

    #[test]
    fn test_install_source_equality() {
        assert_eq!(InstallSource::Cargo, InstallSource::Cargo);
        assert_ne!(InstallSource::Cargo, InstallSource::Apt);
    }

    // ==================== Tool Tests ====================

    #[test]
    fn test_tool_new() {
        let tool = Tool::new("ripgrep");
        assert_eq!(tool.name, "ripgrep");
        assert!(tool.id.is_none());
        assert!(tool.description.is_none());
        assert!(tool.category.is_none());
        assert_eq!(tool.source, InstallSource::Unknown);
        assert!(tool.install_command.is_none());
        assert!(tool.binary_name.is_none());
        assert!(!tool.is_installed);
        assert!(!tool.is_favorite);
        assert!(tool.notes.is_none());
    }

    #[test]
    fn test_tool_builder_pattern() {
        let tool = Tool::new("ripgrep")
            .with_source(InstallSource::Cargo)
            .with_description("Fast grep replacement")
            .with_category("search")
            .with_install_command("cargo install ripgrep")
            .with_binary("rg")
            .installed();

        assert_eq!(tool.name, "ripgrep");
        assert_eq!(tool.source, InstallSource::Cargo);
        assert_eq!(tool.description, Some("Fast grep replacement".to_string()));
        assert_eq!(tool.category, Some("search".to_string()));
        assert_eq!(tool.install_command, Some("cargo install ripgrep".to_string()));
        assert_eq!(tool.binary_name, Some("rg".to_string()));
        assert!(tool.is_installed);
    }

    #[test]
    fn test_tool_builder_chaining_order_independent() {
        // Builder methods can be called in any order
        let tool1 = Tool::new("test")
            .with_source(InstallSource::Pip)
            .with_description("desc")
            .installed();

        let tool2 = Tool::new("test")
            .installed()
            .with_description("desc")
            .with_source(InstallSource::Pip);

        assert_eq!(tool1.source, tool2.source);
        assert_eq!(tool1.description, tool2.description);
        assert_eq!(tool1.is_installed, tool2.is_installed);
    }

    // ==================== Interest Tests ====================

    #[test]
    fn test_interest_new() {
        let interest = Interest::new("rust-tools");
        assert_eq!(interest.name, "rust-tools");
        assert!(interest.id.is_none());
        assert!(interest.description.is_none());
        assert_eq!(interest.priority, 0);
    }

    // ==================== Config Tests ====================

    #[test]
    fn test_config_new() {
        let config = Config::new("nvim", "~/.config/nvim", "/repo/dev/nvim");
        assert_eq!(config.name, "nvim");
        assert_eq!(config.source_path, "~/.config/nvim");
        assert_eq!(config.target_path, "/repo/dev/nvim");
        assert!(config.id.is_none());
        assert!(config.tool_id.is_none());
        assert!(!config.is_symlinked);
    }

    // ==================== Bundle Tests ====================

    #[test]
    fn test_bundle_new() {
        let tools = vec!["ripgrep".to_string(), "fd".to_string(), "bat".to_string()];
        let bundle = Bundle::new("search-tools", tools.clone());

        assert_eq!(bundle.name, "search-tools");
        assert_eq!(bundle.tools, tools);
        assert!(bundle.id.is_none());
        assert!(bundle.description.is_none());
    }

    #[test]
    fn test_bundle_with_description() {
        let bundle = Bundle::new("dev", vec!["cargo".to_string()])
            .with_description("Development tools");

        assert_eq!(bundle.description, Some("Development tools".to_string()));
    }

    #[test]
    fn test_bundle_empty_tools() {
        let bundle = Bundle::new("empty", vec![]);
        assert!(bundle.tools.is_empty());
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_install_source_serialize_json() {
        let source = InstallSource::Cargo;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"Cargo\"");

        let deserialized: InstallSource = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, InstallSource::Cargo);
    }

    #[test]
    fn test_tool_serialize_json() {
        let tool = Tool::new("test")
            .with_source(InstallSource::Cargo)
            .with_description("Test tool");

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"source\":\"Cargo\""));
        assert!(json.contains("\"description\":\"Test tool\""));

        let deserialized: Tool = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.source, InstallSource::Cargo);
    }
}
