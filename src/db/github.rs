//! GitHub data database operations

use anyhow::Result;
use chrono::Utc;
use rusqlite::params;

use super::Database;

/// GitHub repository info
#[derive(Debug, Clone)]
pub struct GitHubInfo {
    pub repo_owner: String,
    pub repo_name: String,
    pub description: Option<String>,
    pub stars: i64,
    pub language: Option<String>,
    pub homepage: Option<String>,
}

/// Input data for storing GitHub repo info
#[derive(Debug)]
pub struct GitHubInfoInput<'a> {
    pub repo_owner: &'a str,
    pub repo_name: &'a str,
    pub description: Option<&'a str>,
    pub stars: i64,
    pub language: Option<&'a str>,
    pub homepage: Option<&'a str>,
}

impl Database {
    // ==================== GitHub Data Operations ====================

    /// Store GitHub repo info for a tool
    pub fn set_github_info(&self, tool_name: &str, info: GitHubInfoInput<'_>) -> Result<bool> {
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

        self.conn.execute(
            "INSERT OR REPLACE INTO tool_github
             (tool_id, repo_owner, repo_name, description, stars, language, homepage, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                tool_id,
                info.repo_owner,
                info.repo_name,
                info.description,
                info.stars,
                info.language,
                info.homepage,
                Utc::now().to_rfc3339()
            ],
        )?;

        Ok(true)
    }

    /// Get GitHub info for a tool
    pub fn get_github_info(&self, tool_name: &str) -> Result<Option<GitHubInfo>> {
        let result = self.conn.query_row(
            "SELECT tg.repo_owner, tg.repo_name, tg.description, tg.stars, tg.language, tg.homepage
             FROM tool_github tg
             JOIN tools t ON tg.tool_id = t.id
             WHERE t.name = ?1",
            [tool_name],
            |row| {
                Ok(GitHubInfo {
                    repo_owner: row.get(0)?,
                    repo_name: row.get(1)?,
                    description: row.get(2)?,
                    stars: row.get(3)?,
                    language: row.get(4)?,
                    homepage: row.get(5)?,
                })
            },
        );

        match result {
            Ok(info) => Ok(Some(info)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Check if tool has GitHub info cached
    pub fn has_github_info(&self, tool_name: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM tool_github tg
             JOIN tools t ON tg.tool_id = t.id
             WHERE t.name = ?1",
            [tool_name],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get tools without GitHub info
    pub fn get_tools_without_github(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name FROM tools t
             LEFT JOIN tool_github tg ON t.id = tg.tool_id
             WHERE tg.tool_id IS NULL
             ORDER BY t.name",
        )?;
        let names = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(names)
    }

    /// Get tools that have GitHub info but missing description in main table
    pub fn get_tools_needing_description_backfill(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, tg.description FROM tools t
             INNER JOIN tool_github tg ON t.id = tg.tool_id
             WHERE t.description IS NULL AND tg.description IS NOT NULL
             ORDER BY t.name",
        )?;
        let results = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }

    /// Get all GitHub info for all tools (for batch loading in TUI)
    pub fn get_all_github_info(&self) -> Result<Vec<(String, GitHubInfo)>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.name, tg.repo_owner, tg.repo_name, tg.description, tg.stars, tg.language, tg.homepage
             FROM tools t
             INNER JOIN tool_github tg ON t.id = tg.tool_id
             ORDER BY t.name",
        )?;
        let results = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    GitHubInfo {
                        repo_owner: row.get(1)?,
                        repo_name: row.get(2)?,
                        description: row.get(3)?,
                        stars: row.get(4)?,
                        language: row.get(5)?,
                        homepage: row.get(6)?,
                    },
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(results)
    }
}
