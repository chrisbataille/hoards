# Hoards User Guide

A comprehensive guide to using hoards for managing your CLI tools.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Syncing Your System](#syncing-your-system)
3. [Discovering Tools](#discovering-tools)
4. [Managing Tools](#managing-tools)
5. [Using Bundles](#using-bundles)
6. [Version Policies](#version-policies)
7. [Usage Tracking](#usage-tracking)
8. [Usage Insights](#usage-insights)
9. [Package Managers](#package-managers)
10. [AI Features](#ai-features)
11. [Config Management](#config-management)
12. [Terminal UI](#terminal-ui)
13. [Maintenance](#maintenance)
14. [Troubleshooting](#troubleshooting)

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
3. Fetching descriptions from registries
4. Installing shell completions for detected shells (Fish, Bash, Zsh)
5. Optionally fetching GitHub data
6. Optionally setting up AI categorization

### Shell Completions

Shell completions are automatically installed during `hoards init`. To manage them manually:

```bash
# Check completion status
hoards completions status

# Install completions for detected shells
hoards completions install

# Install for a specific shell
hoards completions install fish
hoards completions install bash
hoards completions install zsh

# Force reinstall (overwrite existing)
hoards completions install --force

# Remove completions
hoards completions uninstall
hoards completions uninstall fish  # Remove for specific shell
```

**Completion file locations:**
- Fish: `~/.config/fish/completions/hoards.fish`
- Bash: `~/.local/share/bash-completion/completions/hoards`
- Zsh: `~/.zfunc/_hoards`

For Zsh, you may need to add `~/.zfunc` to your fpath. The installer will offer to configure this automatically.

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

## Version Policies

Version policies control how tools are updated. This gives you fine-grained control over which updates to accept.

### Policy Types

| Policy | Behavior | Use Case |
|--------|----------|----------|
| **latest** | Accept any version update | Bleeding edge, test tools |
| **stable** | Only minor/patch updates (skip major) | Production tools, stability |
| **pinned** | Never update, keep current version | Critical dependencies |

### Policy Cascade

Policies are resolved in this order (highest to lowest priority):
1. **Tool-level override** - Policy set directly on the tool
2. **Bundle policy** - Policy inherited from a bundle containing the tool
3. **Source default** - Policy set for the package source (cargo, pip, etc.)
4. **Global default** - Falls back to `stable`

### Setting Policies

#### Tool-Level Policies

```bash
# Set policy for a specific tool
hoards policy set ripgrep latest
hoards policy set production-tool pinned

# Clear tool policy (use inherited)
hoards policy clear ripgrep
```

#### Bundle Policies

```bash
# Set policy for all tools in a bundle
hoards policy set-bundle dev-tools latest
hoards policy set-bundle critical-tools pinned

# Clear bundle policy
hoards policy clear-bundle dev-tools
```

#### Source Defaults

```bash
# Set default policy for a package source
hoards policy set-source cargo latest
hoards policy set-source apt stable
hoards policy set-source pip pinned

# Clear source-specific policy
hoards policy clear-source cargo
```

#### Global Default

```bash
# Set the global default policy (affects all tools without specific policies)
hoards policy set-default stable
```

### Viewing Policies

```bash
# Show all configured policies
hoards policy show
```

Example output:
```
Version Policies

Global Default:
  stable

Source Defaults:
  cargo: latest
  apt: stable

Bundle Policies:
  dev-tools: latest (5 tools)
  critical: pinned (3 tools)

Tool Overrides:
  ripgrep: latest
  production-db: pinned

Policy Summary:
  latest - Accept any version update (major, minor, patch)
  stable - Only accept minor and patch updates (skip major)
  pinned - Never update, keep current version
```

### Version Indicators

When viewing tools (in list or TUI), version indicators show update status:

| Icon | Meaning |
|------|---------|
| `↑` | Update available (allowed by policy) |
| `⚠` | Major update skipped (stable policy) |
| `📌` | Tool is pinned |

### Example Workflow

```bash
# 1. Set conservative defaults
hoards policy set-default stable

# 2. Allow bleeding edge for dev tools
hoards policy set-source cargo latest

# 3. Pin critical production tools
hoards policy set production-tool pinned

# 4. Create a bundle with its own policy
hoards bundle create experimental tool1 tool2 tool3
hoards policy set-bundle experimental latest

# 5. Review all policies
hoards policy show
```

---

## Usage Tracking

Hoards tracks how often you use your tools. There are two tracking modes:

### Tracking Modes

**Scan Mode (Manual)**: Periodically parse your shell history files.
```bash
hoards usage config --mode scan
hoards usage scan  # Run periodically
```

**Hook Mode (Automatic)**: Real-time tracking via shell hooks (recommended).
```bash
hoards usage config --mode hook
```

### Setting Up Hook Mode

When you switch to hook mode, hoards will offer to set up your shell automatically:

```bash
$ hoards usage config --mode hook

> Switching to hook mode...
> Detected shell: zsh

? Add hook to ~/.zshrc automatically? [Y/n] y

> Adding hook to ~/.zshrc...
+ Hook added successfully!

> Restart your shell or run: source ~/.zshrc
+ Configuration saved.
```

**Supported shells:**
- **Fish**: Adds hook to `~/.config/fish/config.fish`
- **Zsh**: Adds hook to `~/.zshrc`
- **Bash**: Downloads `bash-preexec` and adds hook to `~/.bashrc`

### Manual Hook Setup

If you prefer manual setup, decline the automatic option and add the hook yourself:

**Fish** (`~/.config/fish/config.fish`):
```fish
function __hoard_log --on-event fish_preexec
    command hoards usage log "$argv[1]" &>/dev/null &
    disown 2>/dev/null
end
```

**Zsh** (`~/.zshrc`):
```zsh
preexec() { command hoards usage log "$1" &>/dev/null & }
```

**Bash** (`~/.bashrc`):
```bash
[[ -f ~/.bash-preexec.sh ]] && source ~/.bash-preexec.sh
preexec() { command hoards usage log "$1" &>/dev/null & }
```

### Usage Commands

```bash
# View/change tracking configuration
hoards usage config
hoards usage config --mode scan
hoards usage config --mode hook

# Show hook setup instructions
hoards usage init
hoards usage init fish  # For specific shell

# Manual history scan (scan mode)
hoards usage scan
hoards usage scan --dry-run  # Preview without saving
hoards usage scan --reset    # Clear counts first

# View usage statistics
hoards usage show
hoards usage show --limit 50

# View usage for specific tool
hoards usage tool ripgrep

# Reset all counters
hoards usage reset
hoards usage reset --force  # Skip confirmation
```

### How It Works

- **Scan mode**: Parses `~/.local/share/fish/fish_history`, `~/.bash_history`, `~/.zsh_history`
- **Hook mode**: Shell calls `hoards usage log <cmd>` on every command (runs in background, no slowdown)
- Both modes update the same counters - you can switch between them without losing data

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

## Package Managers

Hoards supports multiple package managers for tracking and managing tools.

### Supported Sources

| Source | Platform | Scan | Updates | Install | Notes |
|--------|----------|------|---------|---------|-------|
| **Cargo** | Cross-platform | ✅ | ✅ | ✅ | Rust packages from crates.io |
| **Apt** | Debian/Ubuntu | ✅ | ✅ | ✅ | System packages |
| **Pip** | Cross-platform | ✅ | ✅ | ✅ | Python packages from PyPI |
| **Npm** | Cross-platform | ✅ | ✅ | ✅ | Node.js global packages |
| **Brew** | macOS/Linux | ✅ | ✅ | ✅ | Homebrew formulae |
| **Flatpak** | Linux | ✅ | ✅ | ✅ | Universal Linux packages |
| **Manual** | Any | ❌ | ❌ | ❌ | User-tracked tools |

### How Scanning Works

Each source uses different detection methods:

- **Cargo**: Scans `~/.cargo/bin/` for binaries
- **Apt**: Queries `dpkg` for installed packages
- **Pip**: Runs `pip list` to enumerate packages
- **Npm**: Runs `npm list -g` for global packages
- **Brew**: Runs `brew list` for installed formulae
- **Flatpak**: Runs `flatpak list` for installed apps

### Enabling/Disabling Sources

Configure in `~/.config/hoards/config.toml`:

```toml
[sources]
cargo = true
apt = true
pip = false    # Disable pip scanning
npm = false    # Disable npm scanning
brew = false
flatpak = true
manual = true
```

Or use the TUI config menu (`c` key) to toggle sources interactively.

### Cross-Source Migration

Tools may be available from multiple sources. Hoards can detect migration opportunities:

```bash
# Find tools available from better sources
hoards ai migrate

# Migrate from apt to cargo (newer versions)
hoards ai migrate --from apt --to cargo

# Preview without making changes
hoards ai migrate --dry-run
```

### Source Priority

When a tool is available from multiple sources, hoards prefers:
1. **Cargo** - Latest versions, Rust ecosystem
2. **Brew** - Well-maintained, macOS native
3. **Pip/Npm** - Language-specific tools
4. **Apt** - System stability
5. **Flatpak** - Sandboxed apps

You can override by specifying the source during installation:

```bash
hoards install ripgrep --source cargo
```

---

## AI Features

AI helps with categorization, descriptions, and discovery.

### Provider Setup

Before using AI features, install and configure an AI CLI tool:

**Claude (Anthropic):**
```bash
# Install claude CLI
npm install -g @anthropic-ai/claude-cli
# Or via pip
pip install claude-cli

# Configure API key
export ANTHROPIC_API_KEY="your-key"
```

**Gemini (Google):**
```bash
# Install gemini CLI
pip install google-generativeai

# Configure API key
export GOOGLE_API_KEY="your-key"
```

**Codex (OpenAI):**
```bash
# Install codex CLI
npm install -g openai-codex

# Configure API key
export OPENAI_API_KEY="your-key"
```

Then configure hoards:
```bash
hoards ai config set claude  # or: gemini, codex, opencode
hoards ai config test        # Verify connection
```

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

### Smart Bundle Suggestions

AI analyzes your installed tools and usage patterns to suggest logical bundles:

```bash
# Get bundle suggestions based on your usage
hoards ai suggest-bundle

# Suggest a specific number of bundles
hoards ai suggest-bundle --count 3
```

**Interactive mode** (when running in a terminal):
- For each suggested bundle, choose an action:
  - `[c] Create` - Create the bundle in your database
  - `[i] Install` - Install missing tools from the suggestion
  - `[b] Both` - Create bundle and install missing tools
  - `[s] Skip` - Skip this suggestion

**How it works:**
1. Analyzes your installed tools and categories
2. Examines your shell history usage patterns
3. AI suggests bundles based on:
   - Tools you frequently use together
   - Complementary tool categories (e.g., "Modern Unix", "Git Power Tools")
   - Your actual usage counts (prioritizes tools you use most)

Example output:
```
📦 modern-unix - Modern replacements for traditional Unix utilities
   These tools provide better UX than classic Unix commands

   • bat (234x)
   • eza (189x)
   • ripgrep (847x)
   • fd (423x)

Action
> [c] Create bundle 'modern-unix'
  [i] Install missing tools only
  [b] Both - create bundle and install tools
  [s] Skip this suggestion
```

### Tool Cheatsheets

Generate concise quick reference cards for any tool:

```bash
# Generate a cheatsheet for a tool
hoards ai cheatsheet ripgrep

# Regenerate a cached cheatsheet
hoards ai cheatsheet git --refresh
```

Cheatsheets are cached locally and retrieved instantly on subsequent requests. Use `--refresh` to regenerate.

Example output:
```
┌──────────────────────────────────────────────┐
│  ripgrep (rg) - Fast recursive regex search  │
├──────────────────────────────────────────────┤
│ BASIC USAGE                                 │
│   rg pattern       Search current directory │
│   rg -i pattern            Case insensitive │
│                                              │
│ FILE FILTERING                              │
│   rg -t py pattern      Search Python files │
│   rg -g '*.json' pattern        Glob filter │
│                                              │
│ OUTPUT CONTROL                              │
│   rg -l pattern          Files with matches │
│   rg -C 3 pattern             Context lines │
└──────────────────────────────────────────────┘
```

### Contextual Discovery

Find tools based on what you're working on:

```bash
# Describe what you need
hoards ai discover "kubernetes development"
hoards ai discover "data science and machine learning"
hoards ai discover "rust CLI tools"

# Limit number of results
hoards ai discover "web scraping" --limit 5

# Skip GitHub stars lookup (faster)
hoards ai discover "image processing" --no-stars

# Show recommendations without installation prompt
hoards ai discover "database tools" --dry-run
```

The AI analyzes your installed tools and suggests complementary tools. Results include:
- Tool name and description
- Why it's useful for your workflow
- GitHub stars (for popularity reference)
- Installation source and command

**Interactive Installation**: After showing recommendations, you can select which tools to install directly from the results.

### Bundle Cheatsheets

Generate workflow-oriented guides for tool bundles:

```bash
# Generate a cheatsheet for an entire bundle
hoards ai bundle-cheatsheet modern-unix

# Regenerate if tools changed
hoards ai bundle-cheatsheet dev-tools --refresh
```

Bundle cheatsheets show how tools work together rather than just listing individual commands.

### Usage Analysis

Analyze your tool usage patterns to find optimization opportunities:

```bash
# Full analysis with AI insights
hoards ai analyze

# Quick analysis without AI
hoards ai analyze --no-ai

# Only show tools used at least 10 times
hoards ai analyze --min-uses 10

# Output as JSON for scripting
hoards ai analyze --json
```

**Analysis includes:**
- Traditional vs modern tool usage (grep→ripgrep, find→fd, etc.)
- High-value unused tools (popular but you don't use them)
- Personalized recommendations based on your workflow

### Migration Assistant

Find opportunities to migrate tools to better sources:

```bash
# Find all migration opportunities
hoards ai migrate

# Migrate specific sources
hoards ai migrate --from apt --to cargo

# Preview migrations
hoards ai migrate --dry-run

# JSON output
hoards ai migrate --json
```

**Benefits of migration:**
- Newer versions (apt packages often lag)
- Consistent updates (`cargo install` vs system updates)
- Cross-platform compatibility

---

## Terminal UI

Hoards includes a rich Terminal User Interface (TUI) for visual tool management.

```bash
# Launch the TUI
hoards tui

# Or simply (TUI is the default)
hoards
```

**Key features:**
- 5 tabs: Installed, Available, Updates, Bundles, Discover
- Fuzzy search with `/`
- Vim-style navigation (j/k/g/G)
- Multi-select with Space
- Command palette with `:`
- 6 built-in themes (cycle with `t`)
- Mouse support
- Version indicators (`↑` update, `⚠` major skipped, `📌` pinned)

### Version Information in TUI

The tool details panel shows version information:
- **Installed version**: Currently installed version
- **Available version**: Latest available version
- **Version policy**: Effective policy and its source (tool, bundle, source, or global)
- **Update type**: Major, minor, or patch update indicator

For the complete TUI guide, see [TUI_GUIDE.md](TUI_GUIDE.md).

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
