//! Label database operations

use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;

use crate::models::Tool;

use super::Database;
use super::tools::tool_from_row;

impl Database {
    // ==================== Label Operations ====================

    /// Add labels to a tool
    pub fn add_labels(&self, tool_name: &str, labels: &[String]) -> Result<bool> {
        let tool_id: i64 =
            match self
                .conn
                .query_row("SELECT id FROM tools WHERE name = ?1", [tool_name], |row| {
                    row.get(0)
                }) {
                Ok(id) => id,
                Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
                Err(e) => return Err(e.into()),
            };

        let tx = self.conn.unchecked_transaction()?;
        for label in labels {
            tx.execute(
                "INSERT OR IGNORE INTO tool_labels (tool_id, label) VALUES (?1, ?2)",
                params![tool_id, label.to_lowercase()],
            )?;
        }
        tx.commit()?;

        Ok(true)
    }

    /// Get labels for a tool
    pub fn get_labels(&self, tool_name: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT tl.label FROM tool_labels tl
             JOIN tools t ON tl.tool_id = t.id
             WHERE t.name = ?1
             ORDER BY tl.label",
        )?;
        let labels = stmt
            .query_map([tool_name], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(labels)
    }

    /// Get all unique labels
    pub fn get_all_labels(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT label FROM tool_labels ORDER BY label")?;
        let labels = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(labels)
    }

    /// Get all labels with their tool counts in a single query
    pub fn get_label_counts(&self) -> Result<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT label, COUNT(*) as count FROM tool_labels GROUP BY label ORDER BY label",
        )?;
        let counts = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(counts)
    }

    /// List tools by label
    pub fn list_tools_by_label(&self, label: &str) -> Result<Vec<Tool>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.description, t.category, t.source, t.install_command,
                    t.binary_name, t.is_installed, t.is_favorite, t.notes,
                    t.created_at, t.updated_at, t.installed_version, t.available_version,
                    t.version_policy
             FROM tools t
             JOIN tool_labels tl ON t.id = tl.tool_id
             WHERE tl.label = ?1
             ORDER BY t.name",
        )?;

        let tool_iter = stmt.query_map([label.to_lowercase()], tool_from_row)?;

        tool_iter.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Remove a specific label from a tool
    pub fn remove_label(&self, tool_name: &str, label: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM tool_labels WHERE tool_id = (SELECT id FROM tools WHERE name = ?1) AND label = ?2",
            rusqlite::params![tool_name, label.to_lowercase()],
        )?;
        Ok(rows > 0)
    }

    /// Clear labels for a tool
    pub fn clear_labels(&self, tool_name: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM tool_labels WHERE tool_id = (SELECT id FROM tools WHERE name = ?1)",
            [tool_name],
        )?;
        Ok(rows > 0)
    }

    /// Get all labels for all tools (batch operation for TUI)
    /// Returns a map of tool_name -> Vec<label>
    pub fn get_all_tool_labels(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, tl.label
             FROM tool_labels tl
             JOIN tools t ON tl.tool_id = t.id
             ORDER BY t.name, tl.label",
        )?;

        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        for row in rows {
            let (tool_name, label) = row?;
            result.entry(tool_name).or_default().push(label);
        }

        Ok(result)
    }
}
