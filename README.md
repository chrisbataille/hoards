# Hoards

[![CI](https://github.com/chrisbataille/hoards/actions/workflows/ci.yml/badge.svg)](https://github.com/chrisbataille/hoards/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/hoards.svg)](https://crates.io/crates/hoards)
[![Downloads](https://img.shields.io/crates/d/hoards.svg)](https://crates.io/crates/hoards)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/chrisbataille/hoards)](https://github.com/chrisbataille/hoards/stargazers)

**AI-powered CLI tool manager with usage analytics and multi-source tracking.**

*"Know what you use. Discover what you need."*

Hoards tracks your CLI tools across multiple package managers, provides usage analytics from shell history, and offers AI-powered categorization and discovery.

## Features

- **Multi-source tracking** - Track tools from cargo, apt, pip, npm, brew, snap, flatpak
- **Usage analytics** - Parse shell history (Fish, Bash, Zsh) to see which tools you actually use
- **AI integration** - Auto-categorize tools and generate descriptions using Claude, Gemini, or Codex
- **GitHub sync** - Fetch repository info, topics, and stars
- **Bundles** - Group related tools for batch installation
- **Config management** - Track dotfiles and tool configurations
- **Health checks** - Diagnose and fix database inconsistencies

## Installation

```bash
# From crates.io
cargo install hoards

# From source
git clone https://github.com/chrisbataille/hoards.git
cd hoards
cargo install --path .
```

## Quick Start

```bash
# First-time setup (interactive wizard)
hoards init

# Or manually:
hoards sync --scan          # Scan system for tools
hoards sync --github        # Fetch GitHub data
hoards sync --usage         # Parse shell history
hoards sync --all           # Do everything

# Daily maintenance
hoards maintain             # Quick health check + sync

# See what you have
hoards discover list        # List all tools
hoards insights overview    # Usage stats dashboard
```

## Commands

### Workflow Commands

| Command | Description |
|---------|-------------|
| `hoards init` | First-time setup wizard |
| `hoards maintain` | Daily maintenance (sync + health check) |
| `hoards cleanup` | Find and remove unused tools |

### Sync

```bash
hoards sync                 # Sync installation status
hoards sync --scan          # Include tool discovery
hoards sync --github        # Include GitHub data
hoards sync --usage         # Include usage tracking
hoards sync --descriptions  # Fetch descriptions
hoards sync --all           # Everything
```

### Discover

```bash
hoards discover list                # List all tools
hoards discover search <query>      # Search tools
hoards discover categories          # Browse by category
hoards discover labels              # Browse by label
hoards discover missing             # Tools you might want
hoards discover recommended         # Based on your usage
hoards discover similar <tool>      # Find related tools
hoards discover trending            # Popular tools (GitHub stars)
```

### Insights

```bash
hoards insights overview            # Dashboard
hoards insights usage [tool]        # Usage statistics
hoards insights unused              # Tools you never use
hoards insights health              # Database health check
hoards insights stats               # Database statistics
```

### Tool Management

| Command | Description |
|---------|-------------|
| `hoards add <name>` | Add a tool to the database |
| `hoards show <name>` | Show tool details |
| `hoards remove <name>` | Remove from database |
| `hoards install <name>` | Install a tool |
| `hoards uninstall <name>` | Uninstall a tool |
| `hoards upgrade <name>` | Upgrade or switch sources |

### Bundles

```bash
hoards bundle create <name> <tools...>   # Create bundle
hoards bundle list                        # List bundles
hoards bundle show <name>                 # Show contents
hoards bundle install <name>              # Install all tools
hoards bundle add <name> <tools...>       # Add tools
hoards bundle remove <name> <tools...>    # Remove tools
hoards bundle delete <name>               # Delete bundle
```

### AI Features

```bash
# Configuration
hoards ai config set <provider>     # Set provider (claude, gemini, codex)
hoards ai config show               # Show current config
hoards ai config test               # Test connection

# Enrichment
hoards ai enrich                    # Interactive menu
hoards ai enrich --categorize       # Auto-categorize tools
hoards ai enrich --describe         # Generate descriptions
hoards ai enrich --all              # Both operations

# Extract from GitHub README
hoards ai extract <github-url>      # Extract tool info from README
hoards ai extract url1 url2 url3    # Batch mode
hoards ai extract url --yes         # Skip confirmation
```

### Config Management

```bash
hoards config link <name> --source <path> --target <path>
hoards config list                  # List managed configs
hoards config sync                  # Create symlinks
hoards config status                # Check symlink status
```

## Configuration

### Database Location

- Linux: `~/.local/share/hoards/hoards.db`
- macOS: `~/Library/Application Support/hoards/hoards.db`

### Config Files

```
~/.config/hoards/
├── config.toml           # Main configuration
├── prompts/              # Custom AI prompts
└── topic-mapping.toml    # GitHub topic → category mapping
```

### AI Setup

```bash
# Set your preferred AI provider
hoards ai config set claude

# Test the connection
hoards ai config test
```

## Examples

### Track a New Tool

```bash
hoards add ripgrep \
  --description "Fast grep replacement written in Rust" \
  --category search \
  --source cargo \
  --binary rg
```

### Create a Bundle

```bash
# Create a bundle of modern Unix tools
hoards bundle create modern-unix ripgrep fd bat eza zoxide \
  --description "Modern replacements for classic Unix tools"

# Install everything
hoards bundle install modern-unix
```

### Find Unused Tools

```bash
# Scan usage first
hoards sync --usage

# Find tools you never use
hoards insights unused

# Clean up
hoards cleanup
```

## Development

```bash
# Clone
git clone https://github.com/chrisbataille/hoards.git
cd hoards

# Enable pre-commit hooks
git config core.hooksPath .githooks

# Build & test
cargo build
cargo test
cargo clippy
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow.

## License

MIT License - see [LICENSE](LICENSE) for details.
