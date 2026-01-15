use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::PathBuf;

use crate::models::{Bundle, Config, InstallSource, Interest, Tool};

/// Parse a datetime from a string column, returning current time on failure
fn parse_datetime(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Map a database row to a Tool struct
fn tool_from_row(row: &rusqlite::Row) -> rusqlite::Result<Tool> {
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

/// Database wrapper for hoards
pub struct Database {
    conn: Connection,
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
        db.init_schema()?;

        Ok(db)
    }

    /// Open an in-memory database (for testing)
    #[allow(dead_code)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Get the database file path
    pub fn db_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("dev", "hoards", "hoards")
            .context("Failed to determine project directories")?;

        Ok(proj_dirs.data_dir().join("hoards.db"))
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS tools (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                category TEXT,
                source TEXT NOT NULL DEFAULT 'unknown',
                install_command TEXT,
                binary_name TEXT,
                is_installed INTEGER NOT NULL DEFAULT 0,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                notes TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS interests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                priority INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                source_path TEXT NOT NULL,
                target_path TEXT NOT NULL,
                tool_id INTEGER REFERENCES tools(id),
                is_symlinked INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_tools_name ON tools(name);
            CREATE INDEX IF NOT EXISTS idx_tools_category ON tools(category);
            CREATE INDEX IF NOT EXISTS idx_tools_source ON tools(source);
            CREATE INDEX IF NOT EXISTS idx_tools_installed ON tools(is_installed);

            CREATE TABLE IF NOT EXISTS bundles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS bundle_tools (
                bundle_id INTEGER NOT NULL REFERENCES bundles(id) ON DELETE CASCADE,
                tool_name TEXT NOT NULL,
                PRIMARY KEY (bundle_id, tool_name)
            );

            CREATE TABLE IF NOT EXISTS tool_labels (
                tool_id INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
                label TEXT NOT NULL,
                PRIMARY KEY (tool_id, label)
            );

            CREATE TABLE IF NOT EXISTS tool_github (
                tool_id INTEGER PRIMARY KEY REFERENCES tools(id) ON DELETE CASCADE,
                repo_owner TEXT NOT NULL,
                repo_name TEXT NOT NULL,
                description TEXT,
                stars INTEGER DEFAULT 0,
                language TEXT,
                homepage TEXT,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tool_usage (
                tool_id INTEGER PRIMARY KEY REFERENCES tools(id) ON DELETE CASCADE,
                use_count INTEGER NOT NULL DEFAULT 0,
                last_used TEXT,
                first_seen TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS extraction_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                repo_owner TEXT NOT NULL,
                repo_name TEXT NOT NULL,
                version TEXT NOT NULL,
                name TEXT NOT NULL,
                binary TEXT,
                source TEXT NOT NULL,
                install_command TEXT,
                description TEXT NOT NULL,
                category TEXT NOT NULL,
                extracted_at TEXT NOT NULL,
                UNIQUE(repo_owner, repo_name)
            );

            CREATE INDEX IF NOT EXISTS idx_bundles_name ON bundles(name);
            CREATE INDEX IF NOT EXISTS idx_tool_labels_label ON tool_labels(label);
            CREATE INDEX IF NOT EXISTS idx_extraction_cache_repo ON extraction_cache(repo_owner, repo_name);

            CREATE TABLE IF NOT EXISTS ai_cache (
                cache_key TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            "#,
        )?;

        Ok(())
    }

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

    // ==================== Config Operations ====================

    /// Insert a new config
    pub fn insert_config(&self, config: &Config) -> Result<i64> {
        self.conn.execute(
            r#"
            INSERT INTO configs (name, source_path, target_path, tool_id, is_symlinked, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                config.name,
                config.source_path,
                config.target_path,
                config.tool_id,
                config.is_symlinked,
                config.created_at.to_rfc3339(),
                config.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// List all configs
    pub fn list_configs(&self) -> Result<Vec<Config>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, source_path, target_path, tool_id, is_symlinked, created_at, updated_at
             FROM configs ORDER BY name"
        )?;

        let configs = stmt
            .query_map([], |row| {
                Ok(Config {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    source_path: row.get(2)?,
                    target_path: row.get(3)?,
                    tool_id: row.get(4)?,
                    is_symlinked: row.get(5)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(configs)
    }

    /// Get a config by name
    pub fn get_config_by_name(&self, name: &str) -> Result<Option<Config>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, source_path, target_path, tool_id, is_symlinked, created_at, updated_at
             FROM configs WHERE name = ?1"
        )?;

        let config = stmt
            .query_row([name], |row| {
                Ok(Config {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    source_path: row.get(2)?,
                    target_path: row.get(3)?,
                    tool_id: row.get(4)?,
                    is_symlinked: row.get(5)?,
                    created_at: parse_datetime(row.get(6)?),
                    updated_at: parse_datetime(row.get(7)?),
                })
            })
            .optional()?;

        Ok(config)
    }

    /// Get configs associated with a tool
    pub fn get_configs_for_tool(&self, tool_id: i64) -> Result<Vec<Config>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, source_path, target_path, tool_id, is_symlinked, created_at, updated_at
             FROM configs WHERE tool_id = ?1 ORDER BY name"
        )?;

        let configs = stmt
            .query_map([tool_id], |row| {
                Ok(Config {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    source_path: row.get(2)?,
                    target_path: row.get(3)?,
                    tool_id: row.get(4)?,
                    is_symlinked: row.get(5)?,
                    created_at: parse_datetime(row.get(6)?),
                    updated_at: parse_datetime(row.get(7)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(configs)
    }

    /// Update a config's symlink status
    pub fn set_config_symlinked(&self, name: &str, is_symlinked: bool) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE configs SET is_symlinked = ?1, updated_at = ?2 WHERE name = ?3",
            params![is_symlinked, now, name],
        )?;
        Ok(())
    }

    /// Update a config's paths
    pub fn update_config_paths(
        &self,
        name: &str,
        source_path: &str,
        target_path: &str,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE configs SET source_path = ?1, target_path = ?2, updated_at = ?3 WHERE name = ?4",
            params![source_path, target_path, now, name],
        )?;
        Ok(())
    }

    /// Link a config to a tool
    pub fn link_config_to_tool(&self, config_name: &str, tool_name: &str) -> Result<()> {
        let tool = self
            .get_tool_by_name(tool_name)?
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not found", tool_name))?;

        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE configs SET tool_id = ?1, updated_at = ?2 WHERE name = ?3",
            params![tool.id, now, config_name],
        )?;
        Ok(())
    }

    /// Delete a config
    pub fn delete_config(&self, name: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM configs WHERE name = ?1", [name])?;
        Ok(rows > 0)
    }

    // ==================== Bundle Operations ====================

    /// Create a new bundle
    pub fn create_bundle(&self, bundle: &Bundle) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO bundles (name, description, created_at) VALUES (?1, ?2, ?3)",
            params![
                bundle.name,
                bundle.description,
                bundle.created_at.to_rfc3339()
            ],
        )?;

        let bundle_id = self.conn.last_insert_rowid();

        // Insert bundle tools
        for tool_name in &bundle.tools {
            self.conn.execute(
                "INSERT INTO bundle_tools (bundle_id, tool_name) VALUES (?1, ?2)",
                params![bundle_id, tool_name],
            )?;
        }

        Ok(bundle_id)
    }

    /// Get a bundle by name
    pub fn get_bundle(&self, name: &str) -> Result<Option<Bundle>> {
        let bundle_row = self.conn.query_row(
            "SELECT id, name, description, created_at FROM bundles WHERE name = ?1",
            [name],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        );

        match bundle_row {
            Ok((id, name, description, created_at)) => {
                // Get tools for this bundle
                let mut stmt = self.conn.prepare(
                    "SELECT tool_name FROM bundle_tools WHERE bundle_id = ?1 ORDER BY tool_name",
                )?;
                let tools: Vec<String> =
                    stmt.query_map([id], |row| row.get(0))?
                        .collect::<Result<Vec<_>, _>>()?;

                Ok(Some(Bundle {
                    id: Some(id),
                    name,
                    description,
                    tools,
                    created_at: DateTime::parse_from_rfc3339(&created_at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all bundles
    pub fn list_bundles(&self) -> Result<Vec<Bundle>> {
        // Single query with LEFT JOIN to get bundles and their tools
        let mut stmt = self.conn.prepare(
            "SELECT b.id, b.name, b.description, b.created_at, bt.tool_name
             FROM bundles b
             LEFT JOIN bundle_tools bt ON b.id = bt.bundle_id
             ORDER BY b.name, bt.tool_name",
        )?;

        // Group rows by bundle
        let mut bundles: Vec<Bundle> = Vec::new();
        let mut current_id: Option<i64> = None;

        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let created_at: String = row.get(3)?;
            let tool_name: Option<String> = row.get(4)?;
            if current_id != Some(id) {
                // New bundle
                bundles.push(Bundle {
                    id: Some(id),
                    name,
                    description,
                    tools: tool_name.into_iter().collect(),
                    created_at: parse_datetime(created_at),
                });
                current_id = Some(id);
            } else if let Some(tool) = tool_name {
                // Add tool to current bundle
                if let Some(bundle) = bundles.last_mut() {
                    bundle.tools.push(tool);
                }
            }
        }

        Ok(bundles)
    }

    /// Delete a bundle by name
    pub fn delete_bundle(&self, name: &str) -> Result<bool> {
        let rows = self
            .conn
            .execute("DELETE FROM bundles WHERE name = ?1", [name])?;
        Ok(rows > 0)
    }

    /// Add tools to an existing bundle
    pub fn add_to_bundle(&self, bundle_name: &str, tools: &[String]) -> Result<bool> {
        let bundle_id: i64 = match self.conn.query_row(
            "SELECT id FROM bundles WHERE name = ?1",
            [bundle_name],
            |row| row.get(0),
        ) {
            Ok(id) => id,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e.into()),
        };

        for tool_name in tools {
            // Use INSERT OR IGNORE to skip duplicates
            self.conn.execute(
                "INSERT OR IGNORE INTO bundle_tools (bundle_id, tool_name) VALUES (?1, ?2)",
                params![bundle_id, tool_name],
            )?;
        }

        Ok(true)
    }

    /// Remove tools from a bundle
    pub fn remove_from_bundle(&self, bundle_name: &str, tools: &[String]) -> Result<bool> {
        let bundle_id: i64 = match self.conn.query_row(
            "SELECT id FROM bundles WHERE name = ?1",
            [bundle_name],
            |row| row.get(0),
        ) {
            Ok(id) => id,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(e.into()),
        };

        for tool_name in tools {
            self.conn.execute(
                "DELETE FROM bundle_tools WHERE bundle_id = ?1 AND tool_name = ?2",
                params![bundle_id, tool_name],
            )?;
        }

        Ok(true)
    }

    /// Get all bundle names (for completions)
    pub fn get_bundle_names(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name FROM bundles ORDER BY name")?;
        let names = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(names)
    }

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

        for label in labels {
            self.conn.execute(
                "INSERT OR IGNORE INTO tool_labels (tool_id, label) VALUES (?1, ?2)",
                params![tool_id, label.to_lowercase()],
            )?;
        }

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
                    t.created_at, t.updated_at
             FROM tools t
             JOIN tool_labels tl ON t.id = tl.tool_id
             WHERE tl.label = ?1
             ORDER BY t.name",
        )?;

        let tool_iter = stmt.query_map([label.to_lowercase()], tool_from_row)?;

        tool_iter.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Clear labels for a tool
    pub fn clear_labels(&self, tool_name: &str) -> Result<bool> {
        let rows = self.conn.execute(
            "DELETE FROM tool_labels WHERE tool_id = (SELECT id FROM tools WHERE name = ?1)",
            [tool_name],
        )?;
        Ok(rows > 0)
    }

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

    // ==================== Usage Tracking ====================

    /// Record tool usage (increment count or insert new record)
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

        let now = Utc::now().to_rfc3339();

        // Try to update existing record, or insert new one
        let updated = self.conn.execute(
            "UPDATE tool_usage SET use_count = use_count + ?1, last_used = COALESCE(?2, last_used), updated_at = ?3 WHERE tool_id = ?4",
            params![count, last_used, now, tool_id],
        )?;

        if updated == 0 {
            self.conn.execute(
                "INSERT INTO tool_usage (tool_id, use_count, last_used, first_seen, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![tool_id, count, last_used, now, now],
            )?;
        }

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
             ORDER BY t.name"
        )?;

        let tools = stmt
            .query_map([], tool_from_row)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(tools)
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
}

/// Tool usage statistics
#[derive(Debug, Clone)]
pub struct ToolUsage {
    pub use_count: i64,
    pub last_used: Option<String>,
    pub first_seen: String,
}

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

// ==================== Extraction Cache ====================

/// Cached extraction from a GitHub README
#[derive(Debug, Clone)]
pub struct CachedExtraction {
    pub repo_owner: String,
    pub repo_name: String,
    pub version: String,
    pub name: String,
    pub binary: Option<String>,
    pub source: String,
    pub install_command: Option<String>,
    pub description: String,
    pub category: String,
    pub extracted_at: String,
}

impl Database {
    /// Get cached extraction for a repository if version matches
    pub fn get_cached_extraction(
        &self,
        owner: &str,
        repo: &str,
        version: &str,
    ) -> Result<Option<CachedExtraction>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT repo_owner, repo_name, version, name, binary, source,
                   install_command, description, category, extracted_at
            FROM extraction_cache
            WHERE repo_owner = ?1 AND repo_name = ?2 AND version = ?3
            "#,
        )?;

        let mut rows = stmt.query(params![owner, repo, version])?;

        if let Some(row) = rows.next()? {
            Ok(Some(CachedExtraction {
                repo_owner: row.get(0)?,
                repo_name: row.get(1)?,
                version: row.get(2)?,
                name: row.get(3)?,
                binary: row.get(4)?,
                source: row.get(5)?,
                install_command: row.get(6)?,
                description: row.get(7)?,
                category: row.get(8)?,
                extracted_at: row.get(9)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Cache an extraction (upserts if repo already exists)
    pub fn cache_extraction(&self, extraction: &CachedExtraction) -> Result<()> {
        self.conn.execute(
            r#"
            INSERT INTO extraction_cache
                (repo_owner, repo_name, version, name, binary, source,
                 install_command, description, category, extracted_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(repo_owner, repo_name) DO UPDATE SET
                version = excluded.version,
                name = excluded.name,
                binary = excluded.binary,
                source = excluded.source,
                install_command = excluded.install_command,
                description = excluded.description,
                category = excluded.category,
                extracted_at = excluded.extracted_at
            "#,
            params![
                extraction.repo_owner,
                extraction.repo_name,
                extraction.version,
                extraction.name,
                extraction.binary,
                extraction.source,
                extraction.install_command,
                extraction.description,
                extraction.category,
                extraction.extracted_at,
            ],
        )?;
        Ok(())
    }

    /// List all cached extractions
    pub fn list_cached_extractions(&self) -> Result<Vec<CachedExtraction>> {
        let mut stmt = self.conn.prepare(
            r#"
            SELECT repo_owner, repo_name, version, name, binary, source,
                   install_command, description, category, extracted_at
            FROM extraction_cache
            ORDER BY extracted_at DESC
            "#,
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CachedExtraction {
                repo_owner: row.get(0)?,
                repo_name: row.get(1)?,
                version: row.get(2)?,
                name: row.get(3)?,
                binary: row.get(4)?,
                source: row.get(5)?,
                install_command: row.get(6)?,
                description: row.get(7)?,
                category: row.get(8)?,
                extracted_at: row.get(9)?,
            })
        })?;

        let mut extractions = Vec::new();
        for row in rows {
            extractions.push(row?);
        }
        Ok(extractions)
    }

    /// Clear extraction cache
    pub fn clear_extraction_cache(&self) -> Result<usize> {
        let count = self.conn.execute("DELETE FROM extraction_cache", [])?;
        Ok(count)
    }

    // ==================== AI Cache Operations ====================

    /// Get a cached value by key
    pub fn get_ai_cache(&self, key: &str) -> Result<Option<String>> {
        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT content FROM ai_cache WHERE cache_key = ?",
                [key],
                |row| row.get(0),
            )
            .ok();
        Ok(result)
    }

    /// Set a cached value
    pub fn set_ai_cache(&self, key: &str, content: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO ai_cache (cache_key, content, created_at)
             VALUES (?, ?, datetime('now'))",
            rusqlite::params![key, content],
        )?;
        Ok(())
    }

    /// Delete a cached value
    pub fn delete_ai_cache(&self, key: &str) -> Result<bool> {
        let count = self
            .conn
            .execute("DELETE FROM ai_cache WHERE cache_key = ?", [key])?;
        Ok(count > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
