//! Tool and Interest database operations

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;

use crate::models::{InstallSource, Interest, Tool};

use super::Database;

/// Parse a datetime from a string column, returning current time on failure
pub(crate) fn parse_datetime(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Map a database row to a Tool struct
pub(crate) fn tool_from_row(row: &rusqlite::Row) -> rusqlite::Result<Tool> {
    Ok(Tool {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        description: row.get(2)?,
        category: row.get(3)?,
        source: InstallSource::from(row.get::<_, String>(4)?.as_str()),
        install_command: row.get(5)?,
        binary_name: row.get(6)?,
        is_installed: row.get(7)?,
        is_favorite: row.get(8)?,
        notes: row.get(9)?,
        created_at: parse_datetime(row.get(10)?),
        updated_at: parse_datetime(row.get(11)?),
    })
}

impl Database {
    // ==================== Tool Operations ====================

    /// Insert a new tool
    pub fn insert_tool(&self, tool: &Tool) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO tools (name, description, category, source, install_command,
                             binary_name, is_installed, is_favorite, notes, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                tool.name,
                tool.description,
                tool.category,
                tool.source.to_string(),
                tool.install_command,
                tool.binary_name,
                tool.is_installed,
                tool.is_favorite,
                tool.notes,
                tool.created_at.to_rfc3339(),
                tool.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Insert or update a tool (upsert) - avoids race conditions on concurrent installs
    pub fn upsert_tool(&self, tool: &Tool) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO tools (name, description, category, source, install_command,
                             binary_name, is_installed, is_favorite, notes, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(name) DO UPDATE SET
                source = excluded.source,
                install_command = excluded.install_command,
                is_installed = excluded.is_installed,
                updated_at = excluded.updated_at
            "#,
            params![
                tool.name,
                tool.description,
                tool.category,
                tool.source.to_string(),
                tool.install_command,
                tool.binary_name,
                tool.is_installed,
                tool.is_favorite,
                tool.notes,
                tool.created_at.to_rfc3339(),
                tool.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Update an existing tool
    pub fn update_tool(&self, tool: &Tool) -> Result<()> {
        let id = tool.id.context("Tool must have an ID to update")?;

        self.conn.execute(
            r#"
            UPDATE tools SET
                name = ?1, description = ?2, category = ?3, source = ?4,
                install_command = ?5, binary_name = ?6, is_installed = ?7,
                is_favorite = ?8, notes = ?9, updated_at = ?10
            WHERE id = ?11
            "#,
            params![
                tool.name,
                tool.description,
                tool.category,
                tool.source.to_string(),
                tool.install_command,
                tool.binary_name,
                tool.is_installed,
                tool.is_favorite,
                tool.notes,
                Utc::now().to_rfc3339(),
                id,
            ],
        )?;

        Ok(())
    }

    /// Update only the description of a tool
    pub fn update_tool_description(&self, name: &str, description: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE tools SET description = ?1, updated_at = ?2 WHERE name = ?3",
            params![description, Utc::now().to_rfc3339(), name],
        )?;
        Ok(rows > 0)
    }

    /// Update only the category of a tool
    pub fn update_tool_category(&self, name: &str, category: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE tools SET category = ?1, updated_at = ?2 WHERE name = ?3",
            params![category, Utc::now().to_rfc3339(), name],
        )?;
        Ok(rows > 0)
    }

    /// Update only the source of a tool (for migration between package sources)
    pub fn update_tool_source(&self, name: &str, source: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE tools SET source = ?1, updated_at = ?2 WHERE name = ?3",
            params![source, Utc::now().to_rfc3339(), name],
        )?;
        Ok(rows > 0)
    }

    /// Get a tool by name
    pub fn get_tool_by_name(&self, name: &str) -> Result<Option<Tool>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, category, source, install_command,
                    binary_name, is_installed, is_favorite, notes, created_at, updated_at
             FROM tools WHERE name = ?1",
        )?;

        let tool = stmt.query_row([name], tool_from_row);

        match tool {
            Ok(t) => Ok(Some(t)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all tools with optional filters
    pub fn list_tools(&self, installed_only: bool, category: Option<&str>) -> Result<Vec<Tool>> {
        let mut query = String::from(
            "SELECT id, name, description, category, source, install_command,
                    binary_name, is_installed, is_favorite, notes, created_at, updated_at
             FROM tools WHERE 1=1",
        );

        if installed_only {
            query.push_str(" AND is_installed = 1");
        }
        if category.is_some() {
            query.push_str(" AND category = ?1");
        }
        query.push_str(" ORDER BY name");

        let mut stmt = self.conn.prepare(&query)?;

        let rows = if let Some(cat) = category {
            stmt.query([cat])?
        } else {
            stmt.query([])?
        };

        let tools = rows.mapped(tool_from_row).collect::<Result<Vec<_>, _>>()?;

        Ok(tools)
    }

    /// Search tools by name or description
    pub fn search_tools(&self, query: &str) -> Result<Vec<Tool>> {
        let pattern = format!("%{}%", query);

        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, category, source, install_command,
                    binary_name, is_installed, is_favorite, notes, created_at, updated_at
             FROM tools
             WHERE name LIKE ?1 OR description LIKE ?1 OR category LIKE ?1
             ORDER BY name",
        )?;

        let tools = stmt
            .query_map([&pattern], tool_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tools)
    }

    /// Update install status for a tool
    pub fn set_tool_installed(&self, name: &str, installed: bool) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE tools SET is_installed = ?1, updated_at = ?2 WHERE name = ?3",
            params![installed, Utc::now().to_rfc3339(), name],
        )?;

        Ok(rows > 0)
    }

    /// Update favorite status for a tool
    pub fn set_tool_favorite(&self, name: &str, favorite: bool) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE tools SET is_favorite = ?1, updated_at = ?2 WHERE name = ?3",
            params![favorite, Utc::now().to_rfc3339(), name],
        )?;

        Ok(rows > 0)
    }

    /// Delete a tool by name
    pub fn delete_tool(&self, name: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM tools WHERE name = ?1", [name])?;

        Ok(rows > 0)
    }

    /// Get all unique categories
    pub fn get_categories(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT category FROM tools WHERE category IS NOT NULL ORDER BY category",
        )?;

        let categories = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(categories)
    }

    /// Get all categories with their tool counts in a single query
    pub fn get_category_counts(&self) -> Result<Vec<(String, usize)>> {
        let mut stmt = self.conn.prepare(
            "SELECT category, COUNT(*) as count FROM tools WHERE category IS NOT NULL GROUP BY category ORDER BY category"
        )?;
        let counts = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, usize>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(counts)
    }

    /// Get tool count statistics
    pub fn get_stats(&self) -> Result<(i64, i64, i64)> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tools", [], |row| row.get(0))?;

        let installed: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tools WHERE is_installed = 1",
            [],
            |row| row.get(0),
        )?;

        let favorites: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tools WHERE is_favorite = 1",
            [],
            |row| row.get(0),
        )?;

        Ok((total, installed, favorites))
    }

    /// Get all tools for export
    pub fn get_all_tools(&self) -> Result<Vec<Tool>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, category, source, install_command,
                    binary_name, is_installed, is_favorite, notes, created_at, updated_at
             FROM tools ORDER BY name",
        )?;

        let tools = stmt
            .query_map([], tool_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tools)
    }

    /// Get the most recent update timestamp (proxy for last sync)
    pub fn get_last_sync_time(&self) -> Result<Option<DateTime<Utc>>> {
        let result: Option<String> =
            self.conn
                .query_row("SELECT MAX(updated_at) FROM tools", [], |row| row.get(0))?;

        Ok(result.map(parse_datetime))
    }

    // ==================== Interest Operations ====================

    /// Insert a new interest
    pub fn insert_interest(&self, interest: &Interest) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO interests (name, description, priority, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![
                interest.name,
                interest.description,
                interest.priority,
                interest.created_at.to_rfc3339(),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// List all interests
    pub fn list_interests(&self) -> Result<Vec<Interest>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, priority, created_at FROM interests ORDER BY priority DESC, name"
        )?;

        let interests = stmt
            .query_map([], |row| {
                Ok(Interest {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    priority: row.get(3)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(interests)
    }
}
