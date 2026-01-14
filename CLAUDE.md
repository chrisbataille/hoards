# CLAUDE.md - Hoard Project

## Overview

Hoard is an AI-powered CLI tool manager with usage analytics, multi-source tracking, and config management.

**Tagline:** *"Know what you use. Discover what you need."*

## Development

### Build & Test
```bash
cargo build
cargo test
cargo clippy
cargo install --path .
```

### Pre-Commit Checklist
1. Run `cargo test` - all tests must pass
2. Run `cargo clippy` - 0 warnings
3. Update Fish completions if CLI changed: `shell/fish/completions/hoard.fish`

### Code Style
- **DRY**: Extract common logic into reusable functions
- **TDD**: Write tests first for new features
- Keep functions focused and single-purpose
- Use builder pattern for structs

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
