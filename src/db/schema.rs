//! Database schema initialization and migrations

use anyhow::Result;
use rusqlite::Connection;

/// Initialize the database schema
pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
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

        -- Daily usage tracking for sparklines
        CREATE TABLE IF NOT EXISTS usage_daily (
            tool_id INTEGER NOT NULL REFERENCES tools(id) ON DELETE CASCADE,
            date TEXT NOT NULL,  -- YYYY-MM-DD format
            count INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (tool_id, date)
        );

        CREATE INDEX IF NOT EXISTS idx_usage_daily_date ON usage_daily(date);

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

        CREATE TABLE IF NOT EXISTS discover_search_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            query TEXT NOT NULL,
            ai_enabled INTEGER NOT NULL DEFAULT 0,
            source_filters TEXT NOT NULL,  -- JSON array of enabled sources
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_discover_search_history_created ON discover_search_history(created_at DESC);
        "#,
    )?;

    Ok(())
}
