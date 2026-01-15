//! Security tests for input validation and injection prevention

mod common;

use hoards::commands::install::validate_binary_name;
use hoards::validate_package_name;

// ==================== Package Name Validation ====================

#[test]
fn test_validate_package_name_valid() {
    assert!(validate_package_name("ripgrep").is_ok());
    assert!(validate_package_name("fd-find").is_ok());
    assert!(validate_package_name("tree_sitter").is_ok());
    assert!(validate_package_name("@types/node").is_ok());
    assert!(validate_package_name("org.mozilla.firefox").is_ok());
}

#[test]
fn test_validate_package_name_shell_injection() {
    // Command substitution
    assert!(validate_package_name("$(rm -rf /)").is_err());
    assert!(validate_package_name("`rm -rf /`").is_err());

    // Semicolon injection
    assert!(validate_package_name("foo; rm -rf /").is_err());

    // Pipe injection
    assert!(validate_package_name("foo | cat /etc/passwd").is_err());

    // Ampersand injection
    assert!(validate_package_name("foo && rm -rf /").is_err());
    assert!(validate_package_name("foo & rm -rf /").is_err());

    // Redirect injection
    assert!(validate_package_name("foo > /etc/passwd").is_err());
    assert!(validate_package_name("foo >> /etc/passwd").is_err());
    assert!(validate_package_name("foo < /etc/passwd").is_err());

    // Newline injection
    assert!(validate_package_name("foo\nrm -rf /").is_err());
    assert!(validate_package_name("foo\rrm -rf /").is_err());
}

#[test]
fn test_validate_package_name_edge_cases() {
    // Empty
    assert!(validate_package_name("").is_err());

    // Whitespace only
    assert!(validate_package_name("   ").is_err());

    // Leading/trailing whitespace
    assert!(validate_package_name(" ripgrep").is_err());
    assert!(validate_package_name("ripgrep ").is_err());

    // Path traversal
    assert!(validate_package_name("../../../etc/passwd").is_err());
    assert!(validate_package_name("foo/../bar").is_err());
}

// ==================== Binary Name Validation ====================

#[test]
fn test_validate_binary_name_valid() {
    assert!(validate_binary_name("ripgrep").is_ok());
    assert!(validate_binary_name("fd-find").is_ok());
    assert!(validate_binary_name("tree_sitter").is_ok());
    assert!(validate_binary_name("app.exe").is_ok());
    assert!(validate_binary_name("v1.2.3").is_ok());
}

#[test]
fn test_validate_binary_name_process_injection() {
    // Shell metacharacters that could affect pgrep/pkill
    assert!(validate_binary_name("foo;bar").is_err());
    assert!(validate_binary_name("foo|bar").is_err());
    assert!(validate_binary_name("foo&bar").is_err());
    assert!(validate_binary_name("foo`bar").is_err());
    assert!(validate_binary_name("foo$bar").is_err());
    assert!(validate_binary_name("foo(bar").is_err());
    assert!(validate_binary_name("foo)bar").is_err());

    // Spaces (invalid for process names)
    assert!(validate_binary_name("foo bar").is_err());

    // Slashes (path injection)
    assert!(validate_binary_name("foo/bar").is_err());
    assert!(validate_binary_name("/bin/sh").is_err());

    // Path traversal
    assert!(validate_binary_name("..").is_err());
}

#[test]
fn test_validate_binary_name_edge_cases() {
    // Empty
    assert!(validate_binary_name("").is_err());

    // Too long
    let long_name = "a".repeat(101);
    assert!(validate_binary_name(&long_name).is_err());

    // Max length OK
    let max_name = "a".repeat(100);
    assert!(validate_binary_name(&max_name).is_ok());
}

// ==================== Database Security ====================

#[test]
fn test_db_search_sql_injection() {
    let ctx = common::TestContext::new();

    // These should not cause SQL errors or unexpected behavior
    // The search should escape wildcards properly
    let results = ctx.db.search_tools("'; DROP TABLE tools; --");
    assert!(results.is_ok());

    let results = ctx.db.search_tools("% OR 1=1");
    assert!(results.is_ok());

    let results = ctx.db.search_tools("_%_");
    assert!(results.is_ok());
}
