# CLAUDE.md - Hoards Project

## Overview

Hoards is an AI-powered CLI tool manager with usage analytics, multi-source tracking, and config management.

**Tagline:** *"Know what you use. Discover what you need."*

## Development

### Build & Test
```bash
cargo build
cargo test
cargo clippy
cargo install --path .
```

### Pre-Commit Hooks
Pre-commit hooks automatically run on each commit:
```bash
git config core.hooksPath .githooks  # Enable hooks after clone
```
Hooks run: `cargo fmt --check`, `cargo clippy`, `cargo test`

### CI/CD
- **CI**: GitHub Actions runs test, clippy, format on PRs
- **Releases**: release-plz automates versioning and crates.io publishing
- Use conventional commits: `feat:` (minor), `fix:` (patch), `feat!:` (major)

### Git Workflow
See [CONTRIBUTING.md](./CONTRIBUTING.md) for branching strategy and commit conventions.

**Quick reference:**
- `main` = stable releases (tagged)
- `develop` = integration branch
- `feature/*`, `fix/*` = branch from `develop`
- `hotfix/*` = branch from `main` for emergencies
- Use `/git` skill for guided workflow

### Code Style
- **DRY**: Extract common logic into reusable functions
- **TDD**: Write tests first for new features
- Keep functions focused and single-purpose
- Use builder pattern for structs

### Documentation Requirements
**IMPORTANT**: After implementing any new feature, update ALL relevant documentation:
- `README.md` - Quick reference and examples
- `docs/USER_GUIDE.md` - Detailed usage instructions
- `docs/API.md` - Library exports and programmatic usage
- `docs/MIGRATION.md` - If commands changed
- `docs/IMPLEMENTATION_PLAN.md` - Mark tasks complete

---

## Security Rules (MANDATORY)

### Command Execution
**NEVER use `sh -c` or shell interpolation for external commands.**

```rust
// WRONG - Shell injection vulnerability
Command::new("sh").arg("-c").arg(&user_input).output();
Command::new("sh").arg("-c").arg(format!("cargo install {}", pkg)).output();

// CORRECT - Use SafeCommand pattern from src/commands/install.rs
let cmd = get_safe_install_command(package, source, version)?;
cmd.execute()?;

// CORRECT - Direct command with args array
Command::new("cargo").args(["install", &package]).output();
```

### Input Validation
- **Always validate** user input, CLI args, config values, and AI responses
- Use `validate_package_name()` for any package/binary names
- Escape SQL LIKE wildcards: `%` → `\%`, `_` → `\_`
- Validate paths with `canonicalize()` before file operations

### Dependencies
- Run `cargo audit` before adding new dependencies
- Prefer std library over external crates (e.g., `std::io::IsTerminal` over `atty`)
- Check RUSTSEC advisories for existing dependencies
- Document why each dependency is needed in Cargo.toml comments

---

## Architecture Rules

### File Size Limits
- **main.rs**: CLI dispatch only, max 200 lines
- **Command modules**: One command group per file, max 500 lines
- **Any file > 500 lines**: Must be split or refactored

### Module Responsibilities
```
src/
├── main.rs           # CLI dispatch ONLY (no business logic)
├── lib.rs            # Public API exports
├── cli.rs            # Clap definitions
├── db.rs             # Database operations (consider splitting if >50 methods)
├── models.rs         # Data structures
├── commands/         # ALL command implementations go here
│   ├── mod.rs        # Shared utilities (confirm(), etc.)
│   ├── core.rs       # add, list, search, show, remove
│   ├── sync.rs       # scan, fetch-descriptions
│   ├── ai.rs         # AI-powered commands
│   └── ...
└── sources/          # Package source implementations
```

### Anti-Patterns to Avoid
- **God Objects**: Split structs with >20 methods into repositories/services
- **God Modules**: No file should handle multiple unrelated concerns
- **Code Duplication**: Extract to shared module (e.g., `utils.rs`, `commands/mod.rs`)
- **Clippy Suppressions**: Fix the issue instead of `#[allow(...)]`

### Function Guidelines
- Max 7 parameters (use struct if more needed)
- Max 50 lines per function
- Single responsibility per function
- Return `Result<T>` with context: `.context("what failed")?`

---

## Performance Rules

### HTTP Requests
```rust
// WRONG - Creating agent per request
fn fetch(url: &str) -> Option<String> {
    let agent = ureq::Agent::new();  // New agent each call
    agent.get(url).call().ok()
}

// CORRECT - Shared static agent
use std::sync::LazyLock;
static HTTP_AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .build()
        .new_agent()
});
```

### Database Operations
```rust
// WRONG - Individual inserts without transaction
for item in items {
    db.insert(item)?;  // Each triggers fsync
}

// CORRECT - Batch in transaction
let tx = self.conn.transaction()?;
for item in items {
    tx.execute(...)?;
}
tx.commit()?;
```

### Concurrency
- Use `thread::scope` or `rayon` for parallel HTTP fetches
- Never make sequential HTTP calls in a loop
- Stream large files with `BufReader` instead of `read_to_string()`

---

## Testing Requirements

### Minimum Coverage
- **Security-critical code**: 90% coverage (install.rs, input validation)
- **Core business logic**: 70% coverage (db.rs, models.rs, history.rs)
- **New features**: Must include tests before merging

### Required Test Types
1. **Unit tests**: For all public functions
2. **Edge cases**: Empty inputs, max values, special characters
3. **Error paths**: Network failures, permission errors, malformed data
4. **Security tests**: Injection attempts, boundary conditions

### Test Patterns
```rust
// Test naming: test_<function>_<scenario>
#[test]
fn test_validate_package_name_shell_injection() { ... }

// Always test both success and failure
#[test]
fn test_search_tools_returns_matches() { ... }
#[test]
fn test_search_tools_empty_query() { ... }

// Mock external dependencies
#[test]
fn test_fetch_description_http_error() {
    // Use mockito or similar
}
```

### Before Merging Checklist
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings
- [ ] New code has tests
- [ ] Security-sensitive code has injection tests

---

## Architecture

```
src/
├── main.rs           # CLI entry point (dispatch only)
├── lib.rs            # Library exports
├── cli.rs            # Clap command definitions
├── db.rs             # SQLite database operations
├── models.rs         # Data structures (Tool, Bundle, Config)
├── scanner.rs        # System tool scanning
├── github.rs         # GitHub API integration
├── history.rs        # Shell history parsing
├── ai.rs             # AI provider integration
├── commands/         # Command implementations
└── sources/          # Package source implementations
```

## Key Features

- **Multi-source tracking**: cargo, apt, pip, npm, brew, flatpak
- **Usage analytics**: Shell history parsing (Fish, Bash, Zsh)
- **AI integration**: Claude, Gemini, Codex for categorization/discovery
- **GitHub sync**: Stars, descriptions, topics
- **Bundles**: Group related tools
- **Config management**: Track tool configurations

---

## Known Technical Debt

Track issues here until resolved:

- [x] **main.rs bloat**: Reduced from ~1692 to 386 lines - dispatch only
- [ ] **db.rs God Object**: Consider repository pattern split
- [ ] **Sequential HTTP in sources**: Need parallel fetching
- [x] **Missing integration tests**: Added `tests/` directory with security and CLI tests
- [x] **Shell injection vulnerabilities**: Fixed in ai.rs (4 locations)
- [x] **atty dependency**: Replaced with std::io::IsTerminal
- [x] **Shared HTTP agent**: Created `src/http.rs` with connection pooling
- [x] **Database transactions**: Added to batch operations in db.rs
- [x] **Binary name validation**: Added `validate_binary_name()` for process operations

## Review Reports

- `SECURITY_AUDIT_REPORT.md` - Security vulnerabilities and remediation
- `COMPREHENSIVE_REVIEW_REPORT.md` - Full code review findings
