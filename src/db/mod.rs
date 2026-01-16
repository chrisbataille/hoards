//! Database module - SQLite operations for hoards
//!
//! This module is split into focused submodules:
//! - `schema`: Database initialization and migrations
//! - `tools`: Tool and Interest CRUD operations
//! - `bundles`: Bundle operations
//! - `configs`: Config file tracking
//! - `labels`: Tool labeling operations
//! - `github`: GitHub metadata storage
//! - `usage`: Usage tracking operations
//! - `extractions`: AI extraction cache

mod bundles;
mod configs;
mod extractions;
mod github;
mod labels;
mod schema;
mod tools;
mod usage;

// Re-export commonly used types
pub use extractions::CachedExtraction;
pub use github::{GitHubInfo, GitHubInfoInput};
pub use usage::ToolUsage;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use rusqlite::Connection;
use std::path::PathBuf;

/// Database wrapper for hoards
pub struct Database {
    pub(crate) conn: Connection,
}

impl Database {
    /// Open or create the database at the default location
    pub fn open() -> Result<Self> {
        let path = Self::db_path()?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let conn = Connection::open(&path).context("Failed to open database")?;

        let db = Self { conn };
        schema::init_schema(&db.conn)?;

        Ok(db)
    }

    /// Open an in-memory database (for testing)
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        schema::init_schema(&db.conn)?;
        Ok(db)
    }

    /// Get the database file path
    pub fn db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("dev", "hoards", "hoards")
            .context("Failed to determine project directories")?;

        Ok(proj_dirs.data_dir().join("hoards.db"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Bundle, InstallSource, Tool};

    // ==================== Tool CRUD Tests ====================

    #[test]
    fn test_tool_crud() -> Result<()> {
        let db = Database::open_in_memory()?;

        // Insert
        let tool = Tool::new("ripgrep")
            .with_source(InstallSource::Cargo)
            .with_description("Fast search tool")
            .with_category("search")
            .installed();

        let id = db.insert_tool(&tool)?;
        assert!(id > 0);

        // Get by name
        let fetched = db.get_tool_by_name("ripgrep")?.unwrap();
        assert_eq!(fetched.name, "ripgrep");
        assert!(fetched.is_installed);

        // Search
        let results = db.search_tools("rip")?;
        assert_eq!(results.len(), 1);

        // List
        let all = db.list_tools(false, None)?;
        assert_eq!(all.len(), 1);

        // Delete
        assert!(db.delete_tool("ripgrep")?);
        assert!(db.get_tool_by_name("ripgrep")?.is_none());

        Ok(())
    }

    #[test]
    fn test_tool_update() -> Result<()> {
        let db = Database::open_in_memory()?;

        let tool = Tool::new("test")
            .with_source(InstallSource::Cargo)
            .with_description("Original");
        db.insert_tool(&tool)?;

        // Update via get + modify + update
        let mut fetched = db.get_tool_by_name("test")?.unwrap();
        fetched.description = Some("Updated".to_string());
        fetched.is_installed = true;
        db.update_tool(&fetched)?;

        let updated = db.get_tool_by_name("test")?.unwrap();
        assert_eq!(updated.description, Some("Updated".to_string()));
        assert!(updated.is_installed);

        Ok(())
    }

    #[test]
    fn test_tool_update_description() -> Result<()> {
        let db = Database::open_in_memory()?;

        let tool = Tool::new("test");
        db.insert_tool(&tool)?;

        db.update_tool_description("test", "New description")?;

        let fetched = db.get_tool_by_name("test")?.unwrap();
        assert_eq!(fetched.description, Some("New description".to_string()));

        Ok(())
    }

    #[test]
    fn test_tool_update_category() -> Result<()> {
        let db = Database::open_in_memory()?;

        let tool = Tool::new("test");
        db.insert_tool(&tool)?;

        db.update_tool_category("test", "search")?;

        let fetched = db.get_tool_by_name("test")?.unwrap();
        assert_eq!(fetched.category, Some("search".to_string()));

        Ok(())
    }

    #[test]
    fn test_set_tool_installed() -> Result<()> {
        let db = Database::open_in_memory()?;

        let tool = Tool::new("test");
        db.insert_tool(&tool)?;

        // Initially not installed
        let fetched = db.get_tool_by_name("test")?.unwrap();
        assert!(!fetched.is_installed);

        // Set as installed
        db.set_tool_installed("test", true)?;
        let fetched = db.get_tool_by_name("test")?.unwrap();
        assert!(fetched.is_installed);

        // Set as not installed
        db.set_tool_installed("test", false)?;
        let fetched = db.get_tool_by_name("test")?.unwrap();
        assert!(!fetched.is_installed);

        Ok(())
    }

    #[test]
    fn test_list_tools_filter_installed() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("installed").installed())?;
        db.insert_tool(&Tool::new("not-installed"))?;

        let all = db.list_tools(false, None)?;
        assert_eq!(all.len(), 2);

        let installed_only = db.list_tools(true, None)?;
        assert_eq!(installed_only.len(), 1);
        assert_eq!(installed_only[0].name, "installed");

        Ok(())
    }

    #[test]
    fn test_list_tools_filter_category() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("rg").with_category("search"))?;
        db.insert_tool(&Tool::new("fd").with_category("files"))?;
        db.insert_tool(&Tool::new("bat").with_category("files"))?;

        let search_tools = db.list_tools(false, Some("search"))?;
        assert_eq!(search_tools.len(), 1);

        let file_tools = db.list_tools(false, Some("files"))?;
        assert_eq!(file_tools.len(), 2);

        Ok(())
    }

    #[test]
    fn test_get_categories() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("a").with_category("search"))?;
        db.insert_tool(&Tool::new("b").with_category("files"))?;
        db.insert_tool(&Tool::new("c").with_category("search"))?;

        let categories = db.get_categories()?;
        assert_eq!(categories.len(), 2);
        assert!(categories.contains(&"search".to_string()));
        assert!(categories.contains(&"files".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_all_tools() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("a"))?;
        db.insert_tool(&Tool::new("b"))?;
        db.insert_tool(&Tool::new("c"))?;

        let all = db.get_all_tools()?;
        assert_eq!(all.len(), 3);

        Ok(())
    }

    // ==================== Bundle Tests ====================

    #[test]
    fn test_bundle_crud() -> Result<()> {
        let db = Database::open_in_memory()?;

        // Insert tools first
        db.insert_tool(&Tool::new("ripgrep"))?;
        db.insert_tool(&Tool::new("fd"))?;

        // Create bundle
        let bundle = Bundle::new("search", vec!["ripgrep".to_string(), "fd".to_string()])
            .with_description("Search tools");
        let id = db.create_bundle(&bundle)?;
        assert!(id > 0);

        // Get bundle
        let fetched = db.get_bundle("search")?.unwrap();
        assert_eq!(fetched.name, "search");
        assert_eq!(fetched.tools.len(), 2);
        assert_eq!(fetched.description, Some("Search tools".to_string()));

        // List bundles
        let bundles = db.list_bundles()?;
        assert_eq!(bundles.len(), 1);

        // Delete bundle
        db.delete_bundle("search")?;
        assert!(db.get_bundle("search")?.is_none());

        Ok(())
    }

    #[test]
    fn test_bundle_add_remove_tools() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("a"))?;
        db.insert_tool(&Tool::new("b"))?;
        db.insert_tool(&Tool::new("c"))?;

        let bundle = Bundle::new("test", vec!["a".to_string()]);
        db.create_bundle(&bundle)?;

        // Add tool to bundle
        db.add_to_bundle("test", &["b".to_string()])?;
        let fetched = db.get_bundle("test")?.unwrap();
        assert_eq!(fetched.tools.len(), 2);

        // Remove tool from bundle
        db.remove_from_bundle("test", &["a".to_string()])?;
        let fetched = db.get_bundle("test")?.unwrap();
        assert_eq!(fetched.tools.len(), 1);
        assert_eq!(fetched.tools[0], "b");

        Ok(())
    }

    // ==================== Labels Tests ====================

    #[test]
    fn test_labels() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("test"))?;

        // Add labels
        db.add_labels("test", &["rust".to_string(), "cli".to_string()])?;

        // Get labels
        let labels = db.get_labels("test")?;
        assert_eq!(labels.len(), 2);
        assert!(labels.contains(&"rust".to_string()));
        assert!(labels.contains(&"cli".to_string()));

        // Get all labels
        let all_labels = db.get_all_labels()?;
        assert_eq!(all_labels.len(), 2);

        // Clear labels and re-add just one
        db.clear_labels("test")?;
        db.add_labels("test", &["cli".to_string()])?;
        let labels = db.get_labels("test")?;
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0], "cli");

        Ok(())
    }

    #[test]
    fn test_list_tools_by_label() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("rg"))?;
        db.insert_tool(&Tool::new("fd"))?;
        db.insert_tool(&Tool::new("bat"))?;

        db.add_labels("rg", &["rust".to_string(), "search".to_string()])?;
        db.add_labels("fd", &["rust".to_string()])?;
        db.add_labels("bat", &["rust".to_string()])?;

        let rust_tools = db.list_tools_by_label("rust")?;
        assert_eq!(rust_tools.len(), 3);

        let search_tools = db.list_tools_by_label("search")?;
        assert_eq!(search_tools.len(), 1);

        Ok(())
    }

    // ==================== Usage Tests ====================

    #[test]
    fn test_usage_tracking() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("test").installed())?;

        // Record usage
        db.record_usage("test", 10, Some("2024-01-01T00:00:00Z"))?;

        // Get usage
        let usage = db.get_usage("test")?.unwrap();
        assert_eq!(usage.use_count, 10);
        assert_eq!(usage.last_used, Some("2024-01-01T00:00:00Z".to_string()));

        // Update usage (should add to count)
        db.record_usage("test", 5, Some("2024-01-02T00:00:00Z"))?;
        let usage = db.get_usage("test")?.unwrap();
        assert_eq!(usage.use_count, 15);
        assert_eq!(usage.last_used, Some("2024-01-02T00:00:00Z".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_all_usage() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("a").installed())?;
        db.insert_tool(&Tool::new("b").installed())?;

        db.record_usage("a", 10, None)?;
        db.record_usage("b", 5, None)?;

        let all_usage = db.get_all_usage()?;
        assert_eq!(all_usage.len(), 2);

        Ok(())
    }

    #[test]
    fn test_get_unused_tools() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("used").installed())?;
        db.insert_tool(&Tool::new("unused").installed())?;
        db.insert_tool(&Tool::new("not-installed"))?;

        db.record_usage("used", 10, None)?;

        let unused = db.get_unused_tools()?;
        assert_eq!(unused.len(), 1);
        assert_eq!(unused[0].name, "unused");

        Ok(())
    }

    #[test]
    fn test_usage_cascade_on_tool_delete() -> Result<()> {
        let db = Database::open_in_memory()?;

        // Insert tool and record usage
        db.insert_tool(&Tool::new("test").installed())?;
        db.record_usage("test", 10, None)?;

        // Verify usage exists
        let usage = db.get_usage("test")?;
        assert!(usage.is_some());

        // Delete tool - usage should be cascade deleted
        db.delete_tool("test")?;

        // Verify no orphaned records (CASCADE handles cleanup)
        let count = db.count_orphaned_usage()?;
        assert_eq!(count, 0);

        Ok(())
    }

    #[test]
    fn test_clear_usage() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("a").installed())?;
        db.insert_tool(&Tool::new("b").installed())?;
        db.record_usage("a", 10, None)?;
        db.record_usage("b", 20, None)?;

        let usage = db.get_all_usage()?;
        assert_eq!(usage.len(), 2);

        db.clear_usage()?;

        let usage = db.get_all_usage()?;
        assert!(usage.is_empty());

        Ok(())
    }

    // ==================== Search Tests ====================

    #[test]
    fn test_search_tools() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("ripgrep").with_description("Fast grep"))?;
        db.insert_tool(&Tool::new("fd").with_description("Fast find"))?;
        db.insert_tool(&Tool::new("bat"))?;

        // Search by name
        let results = db.search_tools("rip")?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "ripgrep");

        // Search by description
        let results = db.search_tools("fast")?;
        assert_eq!(results.len(), 2);

        // Search no results
        let results = db.search_tools("nonexistent")?;
        assert!(results.is_empty());

        Ok(())
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_duplicate_tool_name() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("test"))?;
        // Trying to insert duplicate should fail
        let result = db.insert_tool(&Tool::new("test"));
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_get_nonexistent_tool() -> Result<()> {
        let db = Database::open_in_memory()?;
        let result = db.get_tool_by_name("nonexistent")?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn test_delete_nonexistent_tool() -> Result<()> {
        let db = Database::open_in_memory()?;
        let deleted = db.delete_tool("nonexistent")?;
        assert!(!deleted);
        Ok(())
    }

    // ==================== Daily Usage Tests ====================

    #[test]
    fn test_daily_usage_tracking() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("ripgrep").installed())?;

        // Record usage (creates daily entry for today)
        db.record_usage("ripgrep", 5, None)?;
        db.record_usage("ripgrep", 3, None)?;

        // Get daily usage for last 7 days
        let daily = db.get_daily_usage("ripgrep", 7)?;
        assert_eq!(daily.len(), 7);

        // Today should have 8 uses (5 + 3)
        assert_eq!(daily[6], 8);

        // Previous days should be 0
        for i in 0..6 {
            assert_eq!(daily[i], 0);
        }

        Ok(())
    }

    #[test]
    fn test_daily_usage_nonexistent_tool() -> Result<()> {
        let db = Database::open_in_memory()?;

        let daily = db.get_daily_usage("nonexistent", 7)?;
        assert_eq!(daily.len(), 7);
        assert!(daily.iter().all(|&x| x == 0));

        Ok(())
    }

    #[test]
    fn test_get_all_daily_usage() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.insert_tool(&Tool::new("ripgrep").installed())?;
        db.insert_tool(&Tool::new("fd").installed())?;

        db.record_usage("ripgrep", 10, None)?;
        db.record_usage("fd", 5, None)?;

        let all_daily = db.get_all_daily_usage(7)?;

        // Should have entries for both tools
        assert!(all_daily.contains_key("ripgrep"));
        assert!(all_daily.contains_key("fd"));

        // Today's values
        assert_eq!(all_daily["ripgrep"][6], 10);
        assert_eq!(all_daily["fd"][6], 5);

        Ok(())
    }
}
