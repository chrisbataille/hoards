# Hoards User Guide

A comprehensive guide to using hoards for managing your CLI tools.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Syncing Your System](#syncing-your-system)
3. [Discovering Tools](#discovering-tools)
4. [Managing Tools](#managing-tools)
5. [Using Bundles](#using-bundles)
6. [Usage Insights](#usage-insights)
7. [AI Features](#ai-features)
8. [Config Management](#config-management)
9. [Maintenance](#maintenance)
10. [Troubleshooting](#troubleshooting)

---

## Getting Started

### First-Time Setup

The easiest way to get started is the interactive setup wizard:

```bash
hoards init
```

This will guide you through:
1. Scanning your system for installed tools
2. Syncing installation status
3. Optionally fetching GitHub data
4. Optionally setting up AI categorization

### Manual Setup

If you prefer manual control:

```bash
# Scan system and sync everything
hoards sync --all

# Or step by step:
hoards sync --scan      # Discover tools
hoards sync             # Update status
hoards sync --github    # Fetch GitHub data
hoards sync --usage     # Parse shell history
```

### Daily Maintenance

Run periodically to keep your database current:

```bash
hoards maintain
```

This performs a quick health check and sync.

---

## Syncing Your System

The `sync` command is your primary tool for keeping the database updated.

### Basic Sync

```bash
# Update installation status only
hoards sync

# Preview changes without applying
hoards sync --dry-run
```

### Full Sync Options

```bash
# Include tool discovery (scans PATH, package managers)
hoards sync --scan

# Include GitHub data (stars, descriptions, topics)
hoards sync --github

# Include usage tracking (parses shell history)
hoards sync --usage

# Include description fetching
hoards sync --descriptions

# Do everything
hoards sync --all
```

### What Gets Synced

| Flag | Action |
|------|--------|
| (none) | Update installed/missing status |
| `--scan` | Discover new tools from system |
| `--github` | Fetch repo info, stars, topics |
| `--usage` | Parse shell history for usage counts |
| `--descriptions` | Fetch descriptions from registries |
| `--all` | All of the above |

---

## Discovering Tools

The `discover` command group helps you explore and find tools.

### List Your Tools

```bash
# List all tracked tools
hoards discover list

# Filter by status
hoards discover list --installed
hoards discover list --missing

# Filter by category or label
hoards discover list --category search
hoards discover list --label rust

# Output formats
hoards discover list --format json
hoards discover list --format table
```

### Search

```bash
# Search local database
hoards discover search grep

# Search GitHub too
hoards discover search "rust cli" --github

# Limit results
hoards discover search grep --limit 20
```

### Browse by Category

```bash
# Show all categories with counts
hoards discover categories

# List tools in a category
hoards discover list --category search
```

### Browse by Label

```bash
# Show all labels (GitHub topics)
hoards discover labels

# List tools with a label
hoards discover list --label rust
```

### Find Similar Tools

```bash
# Find tools similar to one you like
hoards discover similar ripgrep
```

### Trending Tools

```bash
# Show popular tools by GitHub stars
hoards discover trending

# Limit results
hoards discover trending --limit 20
```

### Recommendations

```bash
# Get recommendations based on your usage
hoards discover recommended

# Get more recommendations
hoards discover recommended --count 10
```

### Find Missing Tools

```bash
# Tools you might want to install
hoards discover missing
```

---

## Managing Tools

### Adding Tools

```bash
# Basic add
hoards add mytool

# With full metadata
hoards add ripgrep \
  --description "Fast grep replacement" \
  --category search \
  --source cargo \
  --binary rg
```

### Viewing Tool Details

```bash
# Show details (includes GitHub info if synced)
hoards show ripgrep
```

### Installing Tools

```bash
# Install using detected source
hoards install ripgrep

# Specify source
hoards install requests --source pip

# Install specific version
hoards install ripgrep --version 14.0.0

# Skip confirmation
hoards install ripgrep --force
```

### Uninstalling Tools

```bash
# Uninstall but keep in database
hoards uninstall ripgrep

# Uninstall and remove from database
hoards uninstall ripgrep --remove

# Skip confirmation
hoards uninstall ripgrep --force
```

### Upgrading Tools

```bash
# Upgrade to latest version
hoards upgrade ripgrep

# Switch to different source
hoards upgrade fd --to cargo
```

### Removing from Database

```bash
# Remove from database (keeps installed)
hoards remove mytool
```

---

## Using Bundles

Bundles group related tools for easy management.

### Creating Bundles

```bash
hoards bundle create modern-unix ripgrep fd bat eza zoxide \
  --description "Modern replacements for classic Unix tools"
```

### Managing Bundles

```bash
# List all bundles
hoards bundle list

# Show bundle contents
hoards bundle show modern-unix

# Add tools to bundle
hoards bundle add modern-unix dust procs

# Remove tools from bundle
hoards bundle remove modern-unix dust

# Delete a bundle
hoards bundle delete modern-unix
```

### Installing Bundles

```bash
# Install all tools in a bundle
hoards bundle install modern-unix

# Skip confirmation
hoards bundle install modern-unix --force
```

---

## Usage Insights

The `insights` command group provides analytics about your tool usage.

### Overview Dashboard

```bash
# Combined stats overview
hoards insights overview
```

### Usage Statistics

```bash
# Show top used tools
hoards insights usage

# Show usage for specific tool
hoards insights usage ripgrep

# Limit results
hoards insights usage --limit 50
```

### Find Unused Tools

```bash
# Tools you have but never use
hoards insights unused
```

### Health Check

```bash
# Database health and diagnostics
hoards insights health

# Auto-fix issues
hoards insights health --fix
```

Health checks include:
- Tools marked installed but binary missing
- Tools without descriptions
- Tools without categories
- Orphaned usage records
- GitHub API rate limit status

### Statistics

```bash
# Database statistics
hoards insights stats
```

---

## AI Features

AI helps with categorization, descriptions, and discovery.

### Configuration

```bash
# Set AI provider
hoards ai config set claude    # or: gemini, codex

# Show current config
hoards ai config show

# Test connection
hoards ai config test
```

### Enrichment

```bash
# Interactive enrichment menu
hoards ai enrich

# Auto-categorize uncategorized tools
hoards ai enrich --categorize

# Generate descriptions for tools missing them
hoards ai enrich --describe

# Both operations
hoards ai enrich --all

# Preview without changes
hoards ai enrich --dry-run
```

### Extract from GitHub

Extract tool information directly from a GitHub repository's README:

```bash
# Extract from a single repository
hoards ai extract https://github.com/BurntSushi/ripgrep

# Extract from multiple repositories (batch mode)
hoards ai extract url1 url2 url3

# Rate limit API calls (milliseconds between requests)
hoards ai extract url1 url2 --delay 2000

# Skip confirmation prompt
hoards ai extract url --yes

# Preview without adding to database
hoards ai extract url --dry-run
```

Supported URL formats:
- `https://github.com/owner/repo`
- `git@github.com:owner/repo.git`
- `owner/repo` (shorthand)

Results are cached per repository version to avoid repeat API calls.

---

## Config Management

Track and manage tool configurations.

### Link Configs

```bash
hoards config link nvim \
  --source ~/.config/nvim \
  --target ~/dotfiles/nvim
```

### Manage Configs

```bash
# List managed configs
hoards config list

# Create symlinks
hoards config sync

# Check symlink status
hoards config status
```

---

## Maintenance

### Workflow Commands

```bash
# First-time setup wizard
hoards init

# Daily maintenance
hoards maintain

# Cleanup unused tools
hoards cleanup
```

### Export/Import

```bash
# Export to JSON
hoards export --output tools.json

# Export to TOML
hoards export --output tools.toml --format toml

# Export only installed
hoards export --output installed.json --installed

# Import from file
hoards import tools.json

# Preview import
hoards import tools.json --dry-run
```

### Editing Tools

```bash
# Interactive editor
hoards edit ripgrep
```

---

## Troubleshooting

### Common Issues

#### "Tool not found"

```bash
# Rescan system
hoards sync --scan
```

#### "Rate limit exceeded" (GitHub)

```bash
# Check rate limit
hoards insights health

# Use delay between API calls
hoards sync --github --delay 5000
```

#### "AI provider not configured"

```bash
hoards ai config set claude
hoards ai config test
```

#### Database Issues

```bash
# Run health check with auto-fix
hoards insights health --fix
```

### Getting Help

```bash
# General help
hoards --help

# Command-specific help
hoards sync --help
hoards discover --help
hoards insights --help
hoards ai --help
hoards bundle --help
```

### Database Location

- Linux: `~/.local/share/hoards/hoards.db`
- macOS: `~/Library/Application Support/hoards/hoards.db`

### Resetting

```bash
# Find database
hoards insights stats

# Remove database (caution!)
rm ~/.local/share/hoards/hoards.db

# Start fresh
hoards init
```

---

## Tips & Best Practices

1. **Use `hoards maintain`** regularly to keep data current
2. **Track usage** with `hoards sync --usage` to find unused tools
3. **Create bundles** for tools you install on new machines
4. **Backup** before major changes: `hoards export -o backup.json`
5. **Use AI enrichment** to auto-categorize and describe tools
6. **Sync GitHub data** for better descriptions and topics
