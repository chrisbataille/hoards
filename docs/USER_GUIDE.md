# Hoard User Guide

A step-by-step guide to using hoard for managing your CLI tools.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Scanning Your System](#scanning-your-system)
3. [Managing Tools](#managing-tools)
4. [Using Bundles](#using-bundles)
5. [Usage Analytics](#usage-analytics)
6. [AI Features](#ai-features)
7. [GitHub Integration](#github-integration)
8. [Maintenance](#maintenance)
9. [Troubleshooting](#troubleshooting)

---

## Getting Started

### First-Time Setup

After installing hoard, run these commands to get started:

```bash
# 1. Scan your system for installed tools
hoard scan

# 2. Sync the database with current installation status
hoard sync

# 3. View your tool statistics
hoard stats
```

### Understanding the Output

When you run `hoard list`, you'll see output like:

```
NAME                 CATEGORY     SOURCE     STATUS   DESCRIPTION
--------------------------------------------------------------------------------
ripgrep              search       cargo      installed Fast grep replacement
fd                   files        cargo      installed Fast find replacement
bat                  files        cargo      installed Cat with syntax highlighting
htop                 system       apt        missing   Interactive process viewer
```

- **NAME**: The tool's name
- **CATEGORY**: Tool category (search, files, git, etc.)
- **SOURCE**: Where it was installed from (cargo, apt, pip, etc.)
- **STATUS**: `installed` or `missing`
- **DESCRIPTION**: Brief description

---

## Scanning Your System

### Full System Scan

```bash
# Scan and add all found tools to database
hoard scan

# Preview what would be added (dry run)
hoard scan --dry-run
```

The scan checks:
- **Known tools**: Curated list of popular CLI tools
- **Cargo**: Rust packages in `~/.cargo/bin`
- **Pip**: Python packages
- **Npm**: Node.js global packages
- **Apt**: Debian/Ubuntu packages
- **Brew**: Homebrew packages (macOS/Linux)
- **PATH**: Other binaries in your PATH

### Syncing Status

After installing or uninstalling tools outside of hoard:

```bash
# Update installed/missing status
hoard sync

# Preview changes
hoard sync --dry-run
```

---

## Managing Tools

### Adding Tools Manually

```bash
# Basic add
hoard add mytool

# With metadata
hoard add ripgrep \
  --description "Fast grep replacement" \
  --category search \
  --source cargo \
  --binary rg \
  --installed
```

### Viewing Tools

```bash
# List all tools
hoard list

# List only installed
hoard list --installed

# Filter by category
hoard list --category search

# Filter by label (GitHub topic)
hoard list --label rust

# Output as JSON
hoard list --format json

# Search by name or description
hoard search grep

# Show details for a specific tool
hoard show ripgrep
```

### Installing Tools

```bash
# Install using detected source
hoard install ripgrep

# Specify source
hoard install requests --source pip

# Install specific version
hoard install ripgrep --version 14.0.0

# Skip confirmation
hoard install ripgrep --force
```

### Uninstalling Tools

```bash
# Uninstall but keep in database
hoard uninstall ripgrep

# Uninstall and remove from database
hoard uninstall ripgrep --remove

# Skip confirmation
hoard uninstall ripgrep --force
```

### Upgrading Tools

```bash
# Upgrade to latest version
hoard upgrade ripgrep

# Upgrade to specific version
hoard upgrade ripgrep --version 14.1.0

# Switch to different source (e.g., apt to cargo)
hoard upgrade fd --to cargo
```

### Checking for Updates

```bash
# Check all sources
hoard updates

# Check specific source
hoard updates --source cargo

# Check only tracked tools
hoard updates --tracked

# Show all available versions
hoard updates --tracked --all-versions

# Find newer versions on other sources
hoard updates --cross
```

### Removing Tools

```bash
# Remove from database (keeps installed)
hoard remove mytool

# Skip confirmation
hoard remove mytool --force
```

---

## Using Bundles

Bundles let you group related tools for easy installation.

### Creating Bundles

```bash
# Create a bundle
hoard bundle create search-tools ripgrep fd bat \
  --description "Modern search and file tools"

# Create without description
hoard bundle create dev-tools cargo rustfmt clippy
```

### Managing Bundles

```bash
# List all bundles
hoard bundle list

# Show bundle contents
hoard bundle show search-tools

# Add tools to existing bundle
hoard bundle add search-tools eza zoxide

# Remove tools from bundle
hoard bundle remove search-tools eza

# Delete a bundle
hoard bundle delete search-tools
hoard bundle delete search-tools --force
```

### Installing Bundles

```bash
# Install all tools in a bundle
hoard bundle install search-tools

# Skip confirmation
hoard bundle install search-tools --force
```

### Updating Bundle Tools

```bash
# Interactive update (choose per tool)
hoard bundle update search-tools

# Auto-update all to latest
hoard bundle update search-tools --yes
```

---

## Usage Analytics

Track which tools you actually use based on shell history.

### Scanning History

```bash
# Scan shell history and record usage
hoard usage scan

# Preview what would be recorded
hoard usage scan --dry-run

# Reset counts before scanning
hoard usage scan --reset
```

Supported shells:
- **Fish**: `~/.local/share/fish/fish_history`
- **Bash**: `~/.bash_history`
- **Zsh**: `~/.zsh_history`

### Viewing Usage

```bash
# Show top 20 most used tools
hoard usage show

# Show top 50
hoard usage show --limit 50

# Show usage for specific tool
hoard usage tool ripgrep
```

### Finding Unused Tools

```bash
# List installed tools with no recorded usage
hoard unused
```

### Getting Recommendations

```bash
# Get 5 tool recommendations based on your usage
hoard recommend

# Get 10 recommendations
hoard recommend --count 10
```

---

## AI Features

Use AI to auto-categorize tools and generate descriptions.

### Setup

```bash
# Set AI provider
hoard ai set claude    # or: gemini, codex, opencode

# Show current config
hoard ai show

# Test connection
hoard ai test
```

### Auto-Categorization

```bash
# Categorize uncategorized tools
hoard ai categorize

# Preview changes
hoard ai categorize --dry-run
```

### Description Generation

```bash
# Generate descriptions for tools missing them
hoard ai describe

# Limit to 10 tools
hoard ai describe --limit 10

# Preview changes
hoard ai describe --dry-run
```

### Bundle Suggestions

```bash
# Get AI-suggested bundles based on your tools
hoard ai suggest-bundle

# Get 10 suggestions
hoard ai suggest-bundle --count 10
```

---

## GitHub Integration

Fetch repository information for your tools.

### Setup

Requires the `gh` CLI to be installed and authenticated:

```bash
# Install gh CLI
brew install gh  # or: apt install gh

# Authenticate
gh auth login
```

### Syncing with GitHub

```bash
# Sync all tools with GitHub data
hoard gh sync

# Preview changes
hoard gh sync --dry-run

# Limit API calls
hoard gh sync --limit 50

# Adjust delay between calls (ms)
hoard gh sync --delay 3000
```

### Rate Limits

GitHub has API rate limits:
- **Core API**: 5,000 requests/hour
- **Search API**: 30 requests/minute

```bash
# Check current rate limit status
hoard gh rate-limit
```

### Per-Tool Operations

```bash
# Fetch GitHub info for one tool
hoard gh fetch ripgrep

# Search GitHub
hoard gh search "rust cli tool"
hoard gh search grep --limit 20

# Show cached GitHub info
hoard gh info ripgrep
```

### Backfill Descriptions

Use cached GitHub data without API calls:

```bash
# Fill missing descriptions from cache
hoard gh backfill

# Preview changes
hoard gh backfill --dry-run
```

### Labels

GitHub topics become labels in hoard:

```bash
# List all labels
hoard labels

# List tools by label
hoard list --label rust
hoard list --label cli
```

---

## Maintenance

### Health Checks

```bash
# Check database health
hoard doctor

# Auto-fix issues
hoard doctor --fix
```

Checks performed:
- Tools marked installed but binary missing
- Tools without descriptions
- Tools without categories
- Tools without installation source
- Orphaned usage records
- Duplicate binaries

### Export/Import

```bash
# Export to JSON
hoard export --output tools.json

# Export to TOML
hoard export --output tools.toml --format toml

# Export only installed tools
hoard export --output installed.json --installed

# Print to stdout
hoard export

# Import from file
hoard import tools.json

# Skip existing tools
hoard import tools.json --skip-existing

# Preview import
hoard import tools.json --dry-run
```

### Editing Tools

```bash
# Interactive editor
hoard edit ripgrep
```

This opens an interactive prompt to edit:
- Description
- Category
- Source
- Binary name
- Install command
- Installed status

### Database Info

```bash
# Show database location and stats
hoard info
hoard stats
hoard categories
```

---

## Troubleshooting

### Common Issues

#### "Tool not found"

```bash
# Check if it exists
hoard show mytool

# Rescan system
hoard scan
hoard sync
```

#### "Rate limit exceeded" (GitHub)

```bash
# Check rate limit status
hoard gh rate-limit

# Wait for reset, or use --delay flag
hoard gh sync --delay 5000
```

#### "AI provider not configured"

```bash
# Set up AI provider
hoard ai set claude
hoard ai test
```

#### Database Issues

```bash
# Run health checks
hoard doctor --fix

# Database location
hoard info
```

### Getting Help

```bash
# General help
hoard --help

# Command-specific help
hoard install --help
hoard bundle --help
hoard ai --help
hoard gh --help
hoard usage --help
```

### Resetting

To start fresh:

```bash
# Find database location
hoard info

# Remove database (caution: deletes all data)
rm ~/.local/share/hoard/hoard.db

# Rescan
hoard scan
```

---

## Tips & Best Practices

1. **Regular syncing**: Run `hoard sync` after installing tools outside hoard
2. **Track usage**: Run `hoard usage scan` periodically to track usage
3. **Clean up**: Use `hoard unused` to find tools to remove
4. **Bundles**: Create bundles for your common tool sets
5. **Backups**: Export your database before major changes: `hoard export -o backup.json`
6. **Categories**: Keep tools categorized for easier browsing
7. **GitHub sync**: Run periodically to get latest descriptions and topics
