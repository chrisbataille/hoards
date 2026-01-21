//! Basic CLI integration tests

mod common;

use chrono::Utc;
use hoards::{Bundle, InstallSource, Tool};

// ==================== Database Workflow Tests ====================

#[test]
fn test_add_and_retrieve_tool() {
    let ctx = common::TestContext::new();

    // Add a tool
    let tool = Tool {
        id: None,
        name: "test-tool".to_string(),
        source: InstallSource::Manual,
        description: Some("A test tool".to_string()),
        category: None,
        install_command: None,
        binary_name: Some("test-tool".to_string()),
        is_installed: true,
        is_favorite: false,
        notes: None,
        installed_version: None,
        available_version: None,
        version_policy: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let id = ctx.db.insert_tool(&tool).expect("Failed to add tool");
    assert!(id > 0);

    // Retrieve it
    let retrieved = ctx
        .db
        .get_tool_by_name("test-tool")
        .expect("Failed to get tool")
        .expect("Tool not found");

    assert_eq!(retrieved.name, "test-tool");
    assert_eq!(retrieved.description, Some("A test tool".to_string()));
}

#[test]
fn test_search_tools() {
    let ctx = common::TestContext::new();

    // Add multiple tools
    for name in ["ripgrep", "fd-find", "bat", "exa"] {
        let tool = Tool {
            id: None,
            name: name.to_string(),
            source: InstallSource::Cargo,
            description: Some(format!("{} description", name)),
            category: None,
            install_command: None,
            binary_name: Some(name.to_string()),
            is_installed: true,
            is_favorite: false,
            notes: None,
            installed_version: None,
            available_version: None,
            version_policy: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        ctx.db.insert_tool(&tool).expect("Failed to add tool");
    }

    // Search by name
    let results = ctx.db.search_tools("ripgrep").expect("Search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "ripgrep");

    // Partial match should also work
    let results = ctx.db.search_tools("fd").expect("Search failed");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "fd-find");

    // Empty search returns all
    let results = ctx.db.search_tools("").expect("Search failed");
    assert_eq!(results.len(), 4);
}

#[test]
fn test_bundle_operations() {
    let ctx = common::TestContext::new();

    // Add some tools first
    for name in ["tool1", "tool2", "tool3"] {
        let tool = Tool {
            id: None,
            name: name.to_string(),
            source: InstallSource::Manual,
            description: None,
            category: None,
            install_command: None,
            binary_name: None,
            is_installed: false,
            is_favorite: false,
            notes: None,
            installed_version: None,
            available_version: None,
            version_policy: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        ctx.db.insert_tool(&tool).expect("Failed to add tool");
    }

    // Create a bundle
    let bundle = Bundle {
        id: None,
        name: "test-bundle".to_string(),
        description: Some("A test bundle".to_string()),
        tools: vec!["tool1".to_string(), "tool2".to_string()],
        version_policy: None,
        created_at: Utc::now(),
    };

    let bundle_id = ctx
        .db
        .create_bundle(&bundle)
        .expect("Failed to create bundle");
    assert!(bundle_id > 0);

    // Retrieve bundle
    let retrieved = ctx
        .db
        .get_bundle("test-bundle")
        .expect("Failed to get bundle")
        .expect("Bundle not found");

    assert_eq!(retrieved.name, "test-bundle");
    assert_eq!(retrieved.tools.len(), 2);

    // Add a tool to bundle
    ctx.db
        .add_to_bundle("test-bundle", &["tool3".to_string()])
        .expect("Failed to add to bundle");

    let updated = ctx
        .db
        .get_bundle("test-bundle")
        .expect("Failed to get bundle")
        .expect("Bundle not found");
    assert_eq!(updated.tools.len(), 3);

    // Remove a tool from bundle
    ctx.db
        .remove_from_bundle("test-bundle", &["tool1".to_string()])
        .expect("Failed to remove from bundle");

    let updated = ctx
        .db
        .get_bundle("test-bundle")
        .expect("Failed to get bundle")
        .expect("Bundle not found");
    assert_eq!(updated.tools.len(), 2);
}

#[test]
fn test_labels() {
    let ctx = common::TestContext::new();

    // Add a tool
    let tool = Tool {
        id: None,
        name: "labeled-tool".to_string(),
        source: InstallSource::Manual,
        description: None,
        category: None,
        install_command: None,
        binary_name: None,
        is_installed: false,
        is_favorite: false,
        notes: None,
        installed_version: None,
        available_version: None,
        version_policy: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    ctx.db.insert_tool(&tool).expect("Failed to add tool");

    // Add labels
    ctx.db
        .add_labels(
            "labeled-tool",
            &[
                "rust".to_string(),
                "cli".to_string(),
                "productivity".to_string(),
            ],
        )
        .expect("Failed to add labels");

    // Get labels
    let labels = ctx
        .db
        .get_labels("labeled-tool")
        .expect("Failed to get labels");

    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&"rust".to_string()));
    assert!(labels.contains(&"cli".to_string()));
    assert!(labels.contains(&"productivity".to_string()));
}

#[test]
fn test_label_remove() {
    let ctx = common::TestContext::new();

    // Add a tool
    let tool = Tool {
        id: None,
        name: "remove-label-tool".to_string(),
        source: InstallSource::Manual,
        description: None,
        category: None,
        install_command: None,
        binary_name: None,
        is_installed: false,
        is_favorite: false,
        notes: None,
        installed_version: None,
        available_version: None,
        version_policy: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    ctx.db.insert_tool(&tool).expect("Failed to add tool");

    // Add labels
    ctx.db
        .add_labels(
            "remove-label-tool",
            &["rust".to_string(), "cli".to_string()],
        )
        .expect("Failed to add labels");

    // Verify both labels exist
    let labels = ctx
        .db
        .get_labels("remove-label-tool")
        .expect("Failed to get labels");
    assert_eq!(labels.len(), 2);

    // Remove one label
    ctx.db
        .remove_label("remove-label-tool", "cli")
        .expect("Failed to remove label");

    // Verify only rust remains
    let labels = ctx
        .db
        .get_labels("remove-label-tool")
        .expect("Failed to get labels");
    assert_eq!(labels.len(), 1);
    assert!(labels.contains(&"rust".to_string()));
    assert!(!labels.contains(&"cli".to_string()));
}

#[test]
fn test_label_counts() {
    let ctx = common::TestContext::new();

    // Add multiple tools with labels
    for name in ["tool1", "tool2", "tool3"] {
        let tool = Tool {
            id: None,
            name: name.to_string(),
            source: InstallSource::Manual,
            description: None,
            category: None,
            install_command: None,
            binary_name: None,
            is_installed: false,
            is_favorite: false,
            notes: None,
            installed_version: None,
            available_version: None,
            version_policy: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        ctx.db.insert_tool(&tool).expect("Failed to add tool");
    }

    // Add shared labels
    ctx.db
        .add_labels("tool1", &["rust".to_string(), "cli".to_string()])
        .unwrap();
    ctx.db
        .add_labels("tool2", &["rust".to_string(), "search".to_string()])
        .unwrap();
    ctx.db.add_labels("tool3", &["rust".to_string()]).unwrap();

    // Check counts
    let counts = ctx
        .db
        .get_label_counts()
        .expect("Failed to get label counts");

    let rust_count = counts.iter().find(|(l, _)| l == "rust").map(|(_, c)| *c);
    let cli_count = counts.iter().find(|(l, _)| l == "cli").map(|(_, c)| *c);
    let search_count = counts.iter().find(|(l, _)| l == "search").map(|(_, c)| *c);

    assert_eq!(rust_count, Some(3)); // All three tools have rust
    assert_eq!(cli_count, Some(1)); // Only tool1 has cli
    assert_eq!(search_count, Some(1)); // Only tool2 has search
}

#[test]
fn test_list_tools_by_label() {
    let ctx = common::TestContext::new();

    // Add multiple tools
    for name in ["cargo-tool", "pip-tool", "npm-tool"] {
        let tool = Tool {
            id: None,
            name: name.to_string(),
            source: InstallSource::Manual,
            description: None,
            category: None,
            install_command: None,
            binary_name: None,
            is_installed: false,
            is_favorite: false,
            notes: None,
            installed_version: None,
            available_version: None,
            version_policy: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        ctx.db.insert_tool(&tool).expect("Failed to add tool");
    }

    // Add labels
    ctx.db
        .add_labels("cargo-tool", &["rust".to_string()])
        .unwrap();
    ctx.db
        .add_labels("pip-tool", &["python".to_string()])
        .unwrap();
    ctx.db
        .add_labels("npm-tool", &["javascript".to_string()])
        .unwrap();

    // List tools by label
    let rust_tools = ctx
        .db
        .list_tools_by_label("rust")
        .expect("Failed to list by label");
    assert_eq!(rust_tools.len(), 1);
    assert_eq!(rust_tools[0].name, "cargo-tool");

    let python_tools = ctx
        .db
        .list_tools_by_label("python")
        .expect("Failed to list by label");
    assert_eq!(python_tools.len(), 1);
    assert_eq!(python_tools[0].name, "pip-tool");

    // Non-existent label returns empty
    let unknown_tools = ctx
        .db
        .list_tools_by_label("unknown")
        .expect("Failed to list by label");
    assert!(unknown_tools.is_empty());
}

// ==================== Transaction Atomicity Tests ====================

#[test]
fn test_bundle_creation_atomic() {
    let ctx = common::TestContext::new();

    // Create a bundle with tools (should be atomic)
    let bundle = Bundle {
        id: None,
        name: "atomic-bundle".to_string(),
        description: None,
        tools: vec![
            "tool-a".to_string(),
            "tool-b".to_string(),
            "tool-c".to_string(),
        ],
        version_policy: None,
        created_at: Utc::now(),
    };

    ctx.db
        .create_bundle(&bundle)
        .expect("Failed to create bundle");

    // Verify all tools are present (atomic commit)
    let retrieved = ctx
        .db
        .get_bundle("atomic-bundle")
        .expect("Failed to get bundle")
        .expect("Bundle not found");

    assert_eq!(retrieved.tools.len(), 3);
}
