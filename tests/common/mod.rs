//! Common test utilities

use hoards::Database;

/// Test context that manages a temporary in-memory database
pub struct TestContext {
    pub db: Database,
}

impl TestContext {
    /// Create a new test context with a fresh in-memory database
    pub fn new() -> Self {
        let db = Database::open_in_memory().expect("Failed to create test database");
        TestContext { db }
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}
