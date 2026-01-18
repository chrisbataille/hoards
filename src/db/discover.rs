//! Discover search history operations

use anyhow::Result;
use chrono::Utc;

use super::Database;

/// A saved discover search entry
#[derive(Debug, Clone)]
pub struct DiscoverSearchEntry {
    pub id: i64,
    pub query: String,
    pub ai_enabled: bool,
    pub source_filters: Vec<String>, // e.g., ["cargo", "npm", "github"]
    pub created_at: String,
}

impl Database {
    /// Save a discover search to history
    pub fn save_discover_search(
        &self,
        query: &str,
        ai_enabled: bool,
        source_filters: &[String],
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        let filters_json = serde_json::to_string(source_filters)?;

        self.conn.execute(
            "INSERT INTO discover_search_history (query, ai_enabled, source_filters, created_at) 
             VALUES (?1, ?2, ?3, ?4)",
            (query, ai_enabled as i32, &filters_json, &now),
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get discover search history, most recent first
    pub fn get_discover_history(&self, limit: usize) -> Result<Vec<DiscoverSearchEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, query, ai_enabled, source_filters, created_at 
             FROM discover_search_history 
             ORDER BY created_at DESC 
             LIMIT ?1",
        )?;

        let entries = stmt
            .query_map([limit as i64], |row| {
                let filters_json: String = row.get(3)?;
                let source_filters: Vec<String> =
                    serde_json::from_str(&filters_json).unwrap_or_default();

                Ok(DiscoverSearchEntry {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    ai_enabled: row.get::<_, i32>(2)? != 0,
                    source_filters,
                    created_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    /// Clear discover search history
    pub fn clear_discover_history(&self) -> Result<()> {
        self.conn
            .execute("DELETE FROM discover_search_history", [])?;
        Ok(())
    }

    /// Delete old discover search history (keep only recent N entries)
    pub fn prune_discover_history(&self, keep_count: usize) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM discover_search_history 
             WHERE id NOT IN (
                 SELECT id FROM discover_search_history 
                 ORDER BY created_at DESC 
                 LIMIT ?1
             )",
            [keep_count as i64],
        )?;
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_get_discover_history() -> Result<()> {
        let db = Database::open_in_memory()?;

        // Save a search
        let id = db.save_discover_search(
            "ripgrep",
            false,
            &["cargo".to_string(), "github".to_string()],
        )?;
        assert!(id > 0);

        // Get history
        let history = db.get_discover_history(10)?;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].query, "ripgrep");
        assert!(!history[0].ai_enabled);
        assert_eq!(history[0].source_filters, vec!["cargo", "github"]);

        Ok(())
    }

    #[test]
    fn test_discover_history_order() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.save_discover_search("first", false, &[])?;
        db.save_discover_search("second", true, &["npm".to_string()])?;
        db.save_discover_search("third", false, &["cargo".to_string()])?;

        let history = db.get_discover_history(10)?;
        assert_eq!(history.len(), 3);
        // Most recent first
        assert_eq!(history[0].query, "third");
        assert_eq!(history[1].query, "second");
        assert_eq!(history[2].query, "first");

        Ok(())
    }

    #[test]
    fn test_prune_discover_history() -> Result<()> {
        let db = Database::open_in_memory()?;

        for i in 0..10 {
            db.save_discover_search(&format!("query{}", i), false, &[])?;
        }

        let history = db.get_discover_history(100)?;
        assert_eq!(history.len(), 10);

        // Keep only 5
        let deleted = db.prune_discover_history(5)?;
        assert_eq!(deleted, 5);

        let history = db.get_discover_history(100)?;
        assert_eq!(history.len(), 5);

        Ok(())
    }

    #[test]
    fn test_clear_discover_history() -> Result<()> {
        let db = Database::open_in_memory()?;

        db.save_discover_search("test1", false, &[])?;
        db.save_discover_search("test2", true, &[])?;

        db.clear_discover_history()?;

        let history = db.get_discover_history(10)?;
        assert!(history.is_empty());

        Ok(())
    }
}
