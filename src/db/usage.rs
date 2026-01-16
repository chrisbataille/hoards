//! Usage tracking database operations

use anyhow::Result;
use chrono::Utc;
use rusqlite::{OptionalExtension, params};

use crate::models::Tool;

use super::Database;
use super::tools::tool_from_row;

/// Tool usage statistics
#[derive(Debug, Clone)]
pub struct ToolUsage {
    pub use_count: i64,
    pub last_used: Option<String>,
    pub first_seen: String,
}

impl Database {
    // ==================== Usage Tracking ====================

    /// Record tool usage (increment count or insert new record)
    /// Also tracks daily usage for sparklines
    pub fn record_usage(
        &self,
        tool_name: &str,
        count: i64,
        last_used: Option<&str>,
    ) -> Result<bool> {
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

        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let today = now.format("%Y-%m-%d").to_string();

        // Try to update existing record, or insert new one
        let updated = self.conn.execute(
            "UPDATE tool_usage SET use_count = use_count + ?1, last_used = COALESCE(?2, last_used), updated_at = ?3 WHERE tool_id = ?4",
            params![count, last_used, now_str, tool_id],
        )?;

        if updated == 0 {
            self.conn.execute(
                "INSERT INTO tool_usage (tool_id, use_count, last_used, first_seen, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![tool_id, count, last_used, now_str, now_str],
            )?;
        }

        // Track daily usage for sparklines
        self.conn.execute(
            "INSERT INTO usage_daily (tool_id, date, count) VALUES (?1, ?2, ?3)
             ON CONFLICT(tool_id, date) DO UPDATE SET count = count + ?3",
            params![tool_id, today, count],
        )?;

        Ok(true)
    }

    /// Match a command to a tracked tool by binary or name
    /// Returns the tool name if found, None otherwise
    pub fn match_command_to_tool(&self, cmd: &str) -> Result<Option<String>> {
        // First try to match by binary name, then by tool name
        let result = self.conn.query_row(
            "SELECT name FROM tools WHERE binary_name = ?1 OR name = ?1 LIMIT 1",
            [cmd],
            |row| row.get(0),
        );

        match result {
            Ok(name) => Ok(Some(name)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get usage stats for a tool
    pub fn get_usage(&self, tool_name: &str) -> Result<Option<ToolUsage>> {
        let mut stmt = self.conn.prepare(
            "SELECT tu.use_count, tu.last_used, tu.first_seen
             FROM tool_usage tu
             INNER JOIN tools t ON tu.tool_id = t.id
             WHERE t.name = ?1",
        )?;

        let usage = stmt
            .query_row([tool_name], |row| {
                Ok(ToolUsage {
                    use_count: row.get(0)?,
                    last_used: row.get(1)?,
                    first_seen: row.get(2)?,
                })
            })
            .optional()?;

        Ok(usage)
    }

    /// Get all usage stats sorted by count (most used first)
    pub fn get_all_usage(&self) -> Result<Vec<(String, ToolUsage)>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, tu.use_count, tu.last_used, tu.first_seen
             FROM tool_usage tu
             INNER JOIN tools t ON tu.tool_id = t.id
             ORDER BY tu.use_count DESC",
        )?;

        let results = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    ToolUsage {
                        use_count: row.get(1)?,
                        last_used: row.get(2)?,
                        first_seen: row.get(3)?,
                    },
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get list of tool names and their binary names for matching against history
    pub fn get_tool_binaries(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, COALESCE(binary_name, name) as binary FROM tools")?;

        let results = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Clear all usage data
    pub fn clear_usage(&self) -> Result<()> {
        self.conn.execute("DELETE FROM tool_usage", [])?;
        Ok(())
    }

    /// Count orphaned usage records (tool_id doesn't exist in tools)
    pub fn count_orphaned_usage(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tool_usage WHERE tool_id NOT IN (SELECT id FROM tools)",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Delete orphaned usage records
    pub fn delete_orphaned_usage(&self) -> Result<usize> {
        let deleted = self.conn.execute(
            "DELETE FROM tool_usage WHERE tool_id NOT IN (SELECT id FROM tools)",
            [],
        )?;
        Ok(deleted)
    }

    /// Get installed tools with no usage data (never used)
    pub fn get_unused_tools(&self) -> Result<Vec<Tool>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.name, t.description, t.category, t.source, t.install_command,
                    t.binary_name, t.is_installed, t.is_favorite, t.notes, t.created_at, t.updated_at
             FROM tools t
             LEFT JOIN tool_usage tu ON t.id = tu.tool_id
             WHERE t.is_installed = 1 AND (tu.tool_id IS NULL OR tu.use_count = 0)
             ORDER BY t.name",
        )?;

        let tools = stmt
            .query_map([], tool_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tools)
    }

    /// Get daily usage counts for a tool over the last N days
    /// Returns a vector of counts, oldest first, with 0 for days with no usage
    pub fn get_daily_usage(&self, tool_name: &str, days: u32) -> Result<Vec<i64>> {
        let tool_id: Option<i64> = self
            .conn
            .query_row("SELECT id FROM tools WHERE name = ?1", [tool_name], |row| {
                row.get(0)
            })
            .ok();

        let Some(tool_id) = tool_id else {
            return Ok(vec![0; days as usize]);
        };

        // Generate dates for the last N days
        let today = Utc::now().date_naive();
        let mut dates: Vec<String> = Vec::with_capacity(days as usize);
        for i in (0..days).rev() {
            let date = today - chrono::Duration::days(i as i64);
            dates.push(date.format("%Y-%m-%d").to_string());
        }

        // Fetch usage for these dates
        let placeholders: String = dates.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT date, count FROM usage_daily WHERE tool_id = ?1 AND date IN ({}) ORDER BY date",
            placeholders
        );

        let mut stmt = self.conn.prepare(&query)?;

        // Build params: tool_id first, then all dates
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        params.push(Box::new(tool_id));
        for date in &dates {
            params.push(Box::new(date.clone()));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let usage_map: std::collections::HashMap<String, i64> = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Build result vector with 0 for missing days
        let result: Vec<i64> = dates
            .iter()
            .map(|d| *usage_map.get(d).unwrap_or(&0))
            .collect();

        Ok(result)
    }

    /// Get daily usage for all tools (batch operation for TUI)
    /// Returns a map of tool_name -> Vec<i64> (daily counts, oldest first)
    pub fn get_all_daily_usage(
        &self,
        days: u32,
    ) -> Result<std::collections::HashMap<String, Vec<i64>>> {
        let today = Utc::now().date_naive();
        let start_date = today - chrono::Duration::days(days as i64 - 1);

        let mut stmt = self.conn.prepare(
            "SELECT t.name, ud.date, ud.count
             FROM usage_daily ud
             JOIN tools t ON ud.tool_id = t.id
             WHERE ud.date >= ?1
             ORDER BY t.name, ud.date",
        )?;

        let start_str = start_date.format("%Y-%m-%d").to_string();

        // Generate all dates for the range
        let mut dates: Vec<String> = Vec::with_capacity(days as usize);
        for i in 0..days {
            let date = start_date + chrono::Duration::days(i as i64);
            dates.push(date.format("%Y-%m-%d").to_string());
        }

        // Collect raw data
        let rows: Vec<(String, String, i64)> = stmt
            .query_map([&start_str], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        // Build per-tool date->count maps
        let mut tool_data: std::collections::HashMap<
            String,
            std::collections::HashMap<String, i64>,
        > = std::collections::HashMap::new();
        for (name, date, count) in rows {
            tool_data.entry(name).or_default().insert(date, count);
        }

        // Convert to final format with all dates filled
        let result: std::collections::HashMap<String, Vec<i64>> = tool_data
            .into_iter()
            .map(|(name, date_map)| {
                let counts: Vec<i64> = dates
                    .iter()
                    .map(|d| *date_map.get(d).unwrap_or(&0))
                    .collect();
                (name, counts)
            })
            .collect();

        Ok(result)
    }
}
