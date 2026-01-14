# Hoard

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**A smart tool management system with SQLite database and AI-assisted discovery.**

Hoard tracks your CLI tools across multiple package managers (cargo, apt, pip, npm, brew), provides usage analytics from shell history, and offers AI-powered categorization and recommendations.

## Features

- **Multi-source tracking** - Track tools from cargo, apt, pip, npm, brew, snap, and manual installs
- **Usage analytics** - Parse shell history (Fish, Bash, Zsh) to track which tools you actually use
- **AI integration** - Auto-categorize tools and generate descriptions using Claude, Gemini, or Codex
- **GitHub sync** - Fetch repository info, topics, and stars for your tools
- **Bundles** - Group related tools for batch installation
- **Cross-source updates** - Find newer versions of apt/snap tools available via cargo/pip
- **Export/Import** - Share your tool database in JSON or TOML format
- **Health checks** - Diagnose and fix database inconsistencies

## Quick Start

```bash
# Install hoard
cargo install --path .

# Scan your system for installed tools
hoard scan

# Sync installation status with the database
hoard sync

# See your tool statistics
hoard stats

# Track usage from shell history
hoard usage scan

# Find tools you never use
hoard unused
```

## Installation

### From Source

```bash
git clone https://github.com/youruser/hoard.git
cd hoard
cargo install --path .
```

### Prerequisites

- Rust 1.70+
- SQLite (included via rusqlite)
- `gh` CLI (optional, for GitHub integration)
- AI provider CLI (optional, for AI features)

## Commands

### Core Commands

| Command | Description |
|---------|-------------|
| `hoard add <name>` | Add a tool to the database |
| `hoard list` | List all tracked tools |
| `hoard search <query>` | Search tools by name or description |
| `hoard show <name>` | Show details for a specific tool |
| `hoard remove <name>` | Remove a tool from the database |
| `hoard scan` | Scan system for installed tools |
| `hoard sync` | Sync database with system state |

### Installation Management

| Command | Description |
|---------|-------------|
| `hoard install <name>` | Install a tool |
| `hoard uninstall <name>` | Uninstall a tool |
| `hoard upgrade <name>` | Upgrade a tool or switch sources |
| `hoard updates` | Check for available updates |

### Bundles

| Command | Description |
|---------|-------------|
| `hoard bundle create <name> <tools...>` | Create a new bundle |
| `hoard bundle list` | List all bundles |
| `hoard bundle show <name>` | Show bundle contents |
| `hoard bundle install <name>` | Install all tools in a bundle |
| `hoard bundle add <name> <tools...>` | Add tools to a bundle |
| `hoard bundle remove <name> <tools...>` | Remove tools from a bundle |
| `hoard bundle delete <name>` | Delete a bundle |

### Usage Analytics

| Command | Description |
|---------|-------------|
| `hoard usage scan` | Scan shell history for usage data |
| `hoard usage show` | Show usage statistics |
| `hoard usage tool <name>` | Show usage for a specific tool |
| `hoard unused` | Find installed tools you never use |
| `hoard recommend` | Get tool recommendations based on usage |

### AI Features

| Command | Description |
|---------|-------------|
| `hoard ai set <provider>` | Set AI provider (claude, gemini, codex) |
| `hoard ai show` | Show current AI configuration |
| `hoard ai test` | Test AI connection |
| `hoard ai categorize` | Auto-categorize uncategorized tools |
| `hoard ai describe` | Generate descriptions for tools |
| `hoard ai suggest-bundle` | Get AI-suggested bundles |

### GitHub Integration

| Command | Description |
|---------|-------------|
| `hoard gh sync` | Sync tools with GitHub data |
| `hoard gh fetch <name>` | Fetch GitHub info for a tool |
| `hoard gh search <query>` | Search GitHub for tools |
| `hoard gh info <name>` | Show GitHub info for a tool |
| `hoard gh rate-limit` | Show GitHub API rate limit status |
| `hoard gh backfill` | Fill descriptions from cached GitHub data |

### Maintenance

| Command | Description |
|---------|-------------|
| `hoard doctor` | Check database health |
| `hoard doctor --fix` | Auto-fix database issues |
| `hoard export` | Export tools to JSON/TOML |
| `hoard import <file>` | Import tools from file |
| `hoard edit <name>` | Interactively edit a tool |

## Configuration

### Database Location

The SQLite database is stored at:
- Linux: `~/.local/share/hoard/hoard.db`
- macOS: `~/Library/Application Support/hoard/hoard.db`

### AI Configuration

Configure AI providers in `~/.config/hoard/config.toml`:

```toml
[ai]
provider = "claude"  # claude, gemini, codex, opencode
```

Custom prompts can be placed in `~/.config/hoard/prompts/`.

### Topic Mapping

Customize GitHub topic to category mapping in `~/.config/hoard/topic-mapping.toml`:

```toml
[categories]
search = ["search", "grep", "regex", "find", "ripgrep"]
git = ["git", "github", "gitlab", "version-control"]
shell = ["shell", "terminal", "cli", "command-line"]
```

## Examples

### Track a New Tool

```bash
# Add with full metadata
hoard add ripgrep \
  --description "Fast grep replacement written in Rust" \
  --category search \
  --source cargo \
  --binary rg \
  --installed
```

### Create a Development Bundle

```bash
# Create a bundle of search tools
hoard bundle create search-tools ripgrep fd bat eza \
  --description "Modern CLI search and file tools"

# Install all tools in the bundle
hoard bundle install search-tools
```

### Check for Updates

```bash
# Check all sources
hoard updates

# Check only cargo tools
hoard updates --source cargo

# Find newer versions available on different sources
hoard updates --cross
```

### AI-Powered Features

```bash
# Set up Claude as AI provider
hoard ai set claude

# Auto-categorize all uncategorized tools
hoard ai categorize

# Generate descriptions for tools missing them
hoard ai describe --limit 10
```

## Architecture

```
hoard/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── lib.rs           # Library exports
│   ├── cli.rs           # Clap command definitions
│   ├── db.rs            # SQLite database operations
│   ├── models.rs        # Data structures (Tool, Bundle, etc.)
│   ├── scanner.rs       # System tool scanning
│   ├── github.rs        # GitHub API integration
│   ├── history.rs       # Shell history parsing
│   ├── ai.rs            # AI provider integration
│   ├── updates.rs       # Update checking logic
│   ├── config.rs        # Configuration management
│   ├── commands/        # Command implementations
│   │   ├── mod.rs
│   │   ├── install.rs   # Install/uninstall/upgrade
│   │   ├── bundle.rs    # Bundle management
│   │   ├── ai.rs        # AI commands
│   │   ├── github.rs    # GitHub commands
│   │   ├── usage.rs     # Usage tracking commands
│   │   └── misc.rs      # Export, import, doctor, edit
│   └── sources/         # Package source implementations
│       ├── mod.rs
│       ├── cargo.rs
│       ├── pip.rs
│       ├── npm.rs
│       ├── apt.rs
│       ├── brew.rs
│       └── manual.rs
└── docs/
    └── README.md
```

## Development

```bash
# Run tests
cargo test

# Run with clippy
cargo clippy

# Build release
cargo build --release
```

## License

MIT License - see [LICENSE](LICENSE) for details.
