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

## Architecture

```
src/
├── main.rs           # CLI entry point
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
