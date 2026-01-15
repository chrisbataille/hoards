# Migration Guide

This guide helps existing users transition to the new command structure introduced in v0.2.0.

## Overview of Changes

The CLI has been reorganized from 27+ individual commands into logical command groups:

- **`sync`** - All data synchronization
- **`discover`** - Finding and browsing tools
- **`insights`** - Analytics and health checks
- **`ai config`** / **`ai enrich`** - AI configuration and operations

Most old commands still work as aliases, but will show deprecation warnings.

---

## Command Migration Reference

### Sync Commands

| Old Command | New Command | Notes |
|-------------|-------------|-------|
| `hoard scan` | `hoards sync --scan` | Alias works |
| `hoard sync` | `hoards sync` | Same |
| `hoard fetch-descriptions` | `hoards sync --descriptions` | Deprecated |
| `hoard gh sync` | `hoards sync --github` | Deprecated |
| `hoard usage scan` | `hoards sync --usage` | Deprecated |

**New unified command:**
```bash
# Do everything at once
hoards sync --all

# Or combine flags
hoards sync --scan --github --usage
```

### Discovery Commands

| Old Command | New Command | Notes |
|-------------|-------------|-------|
| `hoard list` | `hoards discover list` | Alias works |
| `hoard search <q>` | `hoards discover search <q>` | Alias works |
| `hoard categories` | `hoards discover categories` | Deprecated |
| `hoard labels` | `hoards discover labels` | Deprecated |
| `hoard suggest` | `hoards discover missing` | Deprecated |
| `hoard recommend` | `hoards discover recommended` | Deprecated |
| `hoard gh search <q>` | `hoards discover search <q> --github` | Deprecated |

**New commands:**
```bash
hoards discover similar <tool>    # Find related tools
hoards discover trending          # Popular by GitHub stars
```

### Insights Commands

| Old Command | New Command | Notes |
|-------------|-------------|-------|
| `hoard usage show` | `hoards insights usage` | Deprecated |
| `hoard usage tool <name>` | `hoards insights usage <name>` | Deprecated |
| `hoard unused` | `hoards insights unused` | Alias works |
| `hoard stats` | `hoards insights stats` | Deprecated |
| `hoard info` | `hoards insights stats` | Deprecated |
| `hoard doctor` | `hoards insights health` | Deprecated |
| `hoard gh rate-limit` | `hoards insights health` | Deprecated |

**New command:**
```bash
hoards insights overview    # Combined dashboard
```

### AI Commands

| Old Command | New Command | Notes |
|-------------|-------------|-------|
| `hoard ai set <p>` | `hoards ai config set <p>` | Deprecated |
| `hoard ai show` | `hoards ai config show` | Deprecated |
| `hoard ai test` | `hoards ai config test` | Deprecated |
| `hoard ai categorize` | `hoards ai enrich --categorize` | Deprecated |
| `hoard ai describe` | `hoards ai enrich --describe` | Deprecated |

**New unified command:**
```bash
hoards ai enrich --all      # Both categorize and describe
hoards ai enrich --dry-run  # Preview changes
```

### GitHub Commands

| Old Command | New Command | Notes |
|-------------|-------------|-------|
| `hoard gh sync` | `hoards sync --github` | Deprecated |
| `hoard gh search <q>` | `hoards discover search <q> --github` | Deprecated |
| `hoard gh info <tool>` | `hoards show <tool>` | Deprecated (info shown inline) |
| `hoard gh rate-limit` | `hoards insights health` | Deprecated |
| `hoard gh fetch <tool>` | `hoards gh fetch <tool>` | Kept for power users |
| `hoard gh backfill` | `hoards gh backfill` | Kept for power users |

### Workflow Commands (New)

These are new commands that simplify common multi-step operations:

```bash
hoards init      # First-time setup wizard
hoards maintain  # Daily maintenance (sync + health check)
hoards cleanup   # Find and remove unused tools
```

---

## Database Migration

The database location has changed:

| OS | Old Path | New Path |
|----|----------|----------|
| Linux | `~/.local/share/hoard/hoard.db` | `~/.local/share/hoards/hoards.db` |
| macOS | `~/Library/Application Support/hoard/hoard.db` | `~/Library/Application Support/hoards/hoards.db` |

**Automatic migration:** The first time you run `hoards`, it will automatically copy your existing database to the new location if found.

**Manual migration:**
```bash
# Linux
mkdir -p ~/.local/share/hoards
cp ~/.local/share/hoard/hoard.db ~/.local/share/hoards/hoards.db

# macOS
mkdir -p ~/Library/Application\ Support/hoards
cp ~/Library/Application\ Support/hoard/hoard.db ~/Library/Application\ Support/hoards/hoards.db
```

---

## Configuration Migration

Config files have moved:

| Old Path | New Path |
|----------|----------|
| `~/.config/hoard/config.toml` | `~/.config/hoards/config.toml` |
| `~/.config/hoard/prompts/` | `~/.config/hoards/prompts/` |
| `~/.config/hoard/topic-mapping.toml` | `~/.config/hoards/topic-mapping.toml` |

**Manual migration:**
```bash
mkdir -p ~/.config/hoards
cp -r ~/.config/hoard/* ~/.config/hoards/
```

---

## Shell Completions

If you had Fish completions installed, update them:

```bash
# Remove old completions
rm ~/.config/fish/completions/hoard.fish

# The new completions are installed automatically with:
hoards --generate-completions fish > ~/.config/fish/completions/hoards.fish
```

---

## Deprecation Timeline

- **v0.2.x**: Old commands work but show deprecation warnings
- **v0.3.0**: Old command aliases will be removed

We recommend updating your scripts and muscle memory now.

---

## Quick Reference Card

```bash
# Daily workflow
hoards maintain              # Quick sync + health check

# Full sync
hoards sync --all            # Scan + GitHub + usage + descriptions

# Find tools
hoards discover list         # List all
hoards discover search grep  # Search
hoards discover trending     # Popular tools

# Analytics
hoards insights overview     # Dashboard
hoards insights unused       # Tools you don't use

# AI features
hoards ai config set claude  # Set provider
hoards ai enrich --all       # Categorize + describe

# Bundles
hoards bundle create <name> <tools...>
hoards bundle install <name>
```

---

## Getting Help

```bash
hoards --help
hoards <command> --help
hoards discover --help
hoards insights --help
hoards ai --help
```

For issues or questions, please open an issue at:
https://github.com/chrisbataille/hoards/issues
