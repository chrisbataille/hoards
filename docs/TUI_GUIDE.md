# Hoards TUI Guide

A comprehensive guide to the Terminal User Interface (TUI) for managing your CLI tools visually.

## Table of Contents

1. [Overview](#overview)
2. [Launching the TUI](#launching-the-tui)
3. [Interface Layout](#interface-layout)
4. [Tabs](#tabs)
5. [Navigation](#navigation)
6. [Input Modes](#input-modes)
7. [Actions](#actions)
8. [Configuration Menu](#configuration-menu)
9. [Themes](#themes)
10. [Mouse Support](#mouse-support)
11. [Tips & Workflows](#tips--workflows)
12. [Troubleshooting](#troubleshooting)

---

## Overview

The TUI provides a rich, interactive interface for managing your CLI tools. It offers:

- **Visual browsing** of all your tools across multiple views
- **Fuzzy search** with real-time filtering
- **Bulk operations** with multi-select support
- **Keyboard-driven** vim-style navigation
- **Mouse support** for point-and-click interaction
- **6 built-in themes** plus custom theme support
- **Undo/redo** for selection operations

---

## Launching the TUI

```bash
# Launch the TUI
hoards tui

# Or simply (TUI is the default when no command is given)
hoards
```

**Requirements:**
- Terminal with 256-color support (most modern terminals)
- Minimum size: 80x24 characters (responsive layout adapts to smaller)
- Unicode support for icons and sparklines

---

## Interface Layout

```
┌─────────────────────────────────────────────────────────────────┐
│  Installed │ Available │ Updates │ Bundles │ Discover          │ <- Tabs
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  > ripgrep        cargo  ████▂▁  ★ 48.2K  [search]             │ <- Tool List
│    fd             cargo  ███▄▂▁  ★ 34.1K  [files]              │
│    bat            cargo  █▄▂▁▁▁  ★ 49.8K  [cli]               │
│    eza            cargo  ▄▂▁▁▁▁  ★ 12.3K  [files]              │
│    delta          cargo  ▁▁▁▁▁▁  ★ 23.4K  [git]               │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│  [AI] [GH] Last sync: Today 3:45 PM │ Sort: usage │ v0.2.1    │ <- Footer
└─────────────────────────────────────────────────────────────────┘
```

**Visual Elements:**
- **Usage Sparklines**: 7-day usage trend (████▂▁ = high to low)
- **GitHub Stars**: ★ with formatted count (K for thousands)
- **Labels**: Colored tags in brackets
- **Source Badge**: Installation source (cargo, apt, pip, etc.)
- **Selection Indicator**: `>` for current, `*` for multi-selected
- **Version Indicators**: `↑` update available, `⚠` major skipped, `📌` pinned

---

## Tabs

### 1. Installed Tab
Shows all tools that are both tracked AND installed on your system.

**Columns:** Name, Source, Usage Sparkline, Stars, Labels

**Actions available:**
- View details (Enter)
- Uninstall (D)
- Update if available (u)
- Add to bundle

### 2. Available Tab
Shows tools tracked in your database but NOT currently installed.

**Use cases:**
- Tools you've uninstalled but want to track
- Tools recommended by others
- Tools from imported bundles

**Actions available:**
- Install (i)
- View details (Enter)
- Remove from database (D)

### 3. Updates Tab
Shows installed tools that have newer versions available, filtered by version policies.

**Columns:** Name, Current Version, Available Version, Source, Policy Indicator

**Version Indicators:**
- `↑` - Update available and allowed by policy
- `⚠` - Major update available but skipped (stable policy)
- `📌` - Tool is pinned (no updates)

**Actions available:**
- Update selected (u)
- Update all (with confirmation)
- Check for updates (r to refresh)
- Cycle version policy (p)

### 4. Bundles Tab
Shows your tool bundles (grouped collections).

**Displays:**
- Bundle name and description
- Tool count and installation status
- Individual tools within bundle

**Actions available:**
- Install entire bundle (i)
- Track missing tools to Available (a)
- View bundle details (Enter)

### 5. Discover Tab
Search and discovery interface for finding new tools.

**Features:**
- Search local database
- Search package registries
- AI-powered discovery (if configured)

---

## Navigation

### Basic Movement

| Key | Action |
|-----|--------|
| `j` or `↓` | Move down |
| `k` or `↑` | Move up |
| `g` | Jump to first item |
| `G` | Jump to last item |
| `Ctrl+d` | Page down (half screen) |
| `Ctrl+u` | Page up (half screen) |
| `Page Down` | Full page down |
| `Page Up` | Full page up |

### Tab Navigation

| Key | Action |
|-----|--------|
| `1` | Go to Installed tab |
| `2` | Go to Available tab |
| `3` | Go to Updates tab |
| `4` | Go to Bundles tab |
| `5` | Go to Discover tab |
| `]` or `Tab` | Next tab |
| `[` or `Shift+Tab` | Previous tab |

### Jump Navigation

| Key | Action |
|-----|--------|
| `f` | Enter jump mode |
| `f` + letter | Jump to first tool starting with letter |

**Example:** Press `f` then `r` to jump to "ripgrep"

### Search

| Key | Action |
|-----|--------|
| `/` | Enter search mode |
| `Esc` | Exit search mode |
| `n` | Next search match |
| `N` | Previous search match |

**Fuzzy matching:** Type partial names, e.g., "rg" matches "ripgrep"

---

## Input Modes

The TUI has four distinct input modes:

### Normal Mode (Default)
- Navigate with j/k or arrows
- Execute actions with keybindings
- All navigation keys active

### Search Mode
- Activated by `/`
- Type to filter tools in real-time
- Fuzzy matching with scoring
- Consecutive character matches score higher
- Press `Esc` to exit, `Enter` to confirm

### Command Mode
- Activated by `:`
- Vim-style command entry
- Tab completion for commands
- Command history with ↑/↓

**Available commands:**
```
:help          - Show help overlay
:quit / :q     - Exit TUI
:theme [name]  - Change theme
:sort [field]  - Change sort (name/usage/recent)
:filter [src]  - Filter by source
:fav           - Toggle favorites filter
:config        - Open configuration menu
:1-5           - Go to tab by number
:install       - Install selected
:delete        - Delete selected
:update        - Update selected
:undo / :z     - Undo last action
:redo / :y     - Redo undone action
```

### Jump Mode
- Activated by `f`
- Press a letter to jump to first matching tool
- Automatically returns to Normal mode after jump

---

## Actions

### Selection

| Key | Action |
|-----|--------|
| `Space` | Toggle selection on current item |
| `Ctrl+a` | Select all items |
| `x` | Clear all selections |

### Tool Actions

| Key | Action |
|-----|--------|
| `i` | Install selected tool(s) |
| `D` | Uninstall/delete selected (with confirmation) |
| `u` | Update selected tool(s) |
| `p` | Cycle version policy (latest → stable → pinned) |
| `Enter` | Toggle details popup |
| `r` | Refresh current view |

### Undo/Redo

| Key | Action |
|-----|--------|
| `Ctrl+z` | Undo last selection/filter change |
| `Ctrl+y` | Redo undone change |

### Other

| Key | Action |
|-----|--------|
| `?` | Show help overlay |
| `c` | Open configuration menu |
| `t` | Cycle through themes |
| `q` or `Esc` | Quit (or close popup/menu) |

### Details Popup

Press `Enter` on any tool to view its details popup:

```
┌─────────────────────────────────────────────┐
│  ripgrep                                     │
├─────────────────────────────────────────────┤
│  Fast regex search tool                      │
│                                              │
│  Source:    cargo                            │
│  Category:  search                           │
│  Stars:     ★ 48.2K                          │
│                                              │
│  Version:   14.0.3 → 14.1.0 (minor)         │
│  Policy:    stable (from: cargo default)    │
│                                              │
│  Labels:    rust, cli, grep, search          │
└─────────────────────────────────────────────┘
```

**Version information shown:**
- **Current version**: Currently installed version
- **Available version**: Latest available (if different)
- **Update type**: major, minor, or patch
- **Policy**: Effective policy and where it's inherited from:
  - `tool override` - Set directly on this tool
  - `bundle: <name>` - Inherited from a bundle
  - `<source> default` - From source-level config
  - `global default` - Fallback default (stable)

---

## Configuration Menu

Access with `c` key or `:config` command.

The configuration menu allows you to customize hoards settings without editing files directly.

### Sections

**1. AI Provider**
- None (disabled)
- Claude (Anthropic)
- Gemini (Google)
- Codex (OpenAI)
- Opencode

**2. Theme**
- Catppuccin Mocha (default dark)
- Catppuccin Latte (light)
- Dracula
- Nord
- Tokyo Night
- Gruvbox
- Custom (if custom-theme.json exists)

**3. Package Sources**
Enable/disable which package managers to track:
- Cargo (Rust)
- Apt (Debian/Ubuntu)
- Pip (Python)
- Npm (Node.js)
- Brew (Homebrew)
- Flatpak
- Manual

**4. Usage Tracking**
- Hook mode (real-time, recommended)
- Scan mode (periodic history parsing)

### Navigation

| Key | Action |
|-----|--------|
| `j/k` or `↑/↓` | Move between options |
| `Tab` | Next section |
| `Shift+Tab` | Previous section |
| `Space` | Toggle checkbox / Select radio |
| `Enter` | Save and close |
| `Esc` | Cancel and close |

**Note:** Theme changes preview immediately. Other changes apply on save.

---

## Themes

### Built-in Themes

| Theme | Description |
|-------|-------------|
| **Catppuccin Mocha** | Warm pastel dark theme (default) |
| **Catppuccin Latte** | Warm pastel light theme |
| **Dracula** | Vibrant dark theme with purple accents |
| **Nord** | Cool blue-tinted dark theme |
| **Tokyo Night** | Neon-inspired dark theme |
| **Gruvbox** | Retro warm dark theme |

### Changing Themes

**Quick cycle:** Press `t` to cycle through themes

**Command:** `:theme [name]` (e.g., `:theme dracula`)

**Config menu:** Press `c`, navigate to Theme section

### Custom Themes

Create a custom theme by adding `~/.config/hoards/custom-theme.json`:

```json
{
  "$schema": "https://raw.githubusercontent.com/chrisbataille/hoards/main/schema/custom-theme.schema.json",
  "name": "My Theme",
  "base": {"r": 30, "g": 30, "b": 46},
  "surface0": {"r": 49, "g": 50, "b": 68},
  "surface1": {"r": 69, "g": 71, "b": 90},
  "text": {"r": 205, "g": 214, "b": 244},
  "subtext0": {"r": 166, "g": 173, "b": 200},
  "blue": {"r": 137, "g": 180, "b": 250},
  "green": {"r": 166, "g": 227, "b": 161},
  "yellow": {"r": 249, "g": 226, "b": 175},
  "red": {"r": 243, "g": 139, "b": 168},
  "mauve": {"r": 203, "g": 166, "b": 247},
  "peach": {"r": 250, "g": 179, "b": 135},
  "teal": {"r": 102, "g": 178, "b": 168}
}
```

Generate a template:
```bash
hoards tui
# Then use command: :create-theme
```

---

## Mouse Support

The TUI supports mouse interaction for common operations:

| Action | Effect |
|--------|--------|
| **Click tab** | Switch to that tab |
| **Click list item** | Select that item |
| **Right-click item** | Toggle multi-selection |
| **Scroll wheel** | Navigate up/down |
| **Click in popup** | Interact with popup elements |

**Note:** Mouse support requires a compatible terminal. Most modern terminals (iTerm2, Alacritty, Kitty, Windows Terminal, GNOME Terminal) support mouse events.

---

## Tips & Workflows

### Quick Tool Installation

1. Press `2` to go to Available tab
2. Press `/` and type tool name
3. Press `Enter` to select
4. Press `i` to install

### Bulk Update

1. Press `3` to go to Updates tab
2. Press `Ctrl+a` to select all
3. Press `u` to update all selected
4. Confirm when prompted

### Create Bundle from Selection

1. On Installed tab, select tools with `Space`
2. Use `:bundle create mytools` command
3. Tools are now grouped

### Find Unused Tools

1. Use `:sort usage` to sort by usage
2. Press `G` to go to bottom (least used)
3. Review tools with no sparkline activity

### Quick Theme Preview

Press `t` repeatedly to cycle through themes and find one you like. The change is immediate but only persists if you save in the config menu.

---

## Troubleshooting

### Terminal Too Small

The TUI adapts to terminal size but requires minimum 80x24. If you see layout issues:
- Resize terminal window
- Use a smaller font size
- The TUI will show a warning if too small

### Colors Look Wrong

Ensure your terminal supports 256 colors:
```bash
echo $TERM
# Should be xterm-256color, screen-256color, etc.
```

Set if needed:
```bash
export TERM=xterm-256color
```

### Mouse Not Working

Some terminals need mouse support enabled:
- **tmux:** Add `set -g mouse on` to ~/.tmux.conf
- **screen:** May not support mouse fully

### Slow Performance

If the TUI feels sluggish:
- Large databases (10,000+ tools) may be slower
- Disable sparklines by filtering to reduce rendering
- Ensure terminal hardware acceleration is enabled

### Keys Not Responding

Check for conflicts with:
- Terminal key bindings
- tmux/screen prefix keys
- Shell key bindings (especially Ctrl combinations)

### Theme Not Applying

- Ensure terminal supports true color (24-bit)
- Check `$COLORTERM` is set to `truecolor`
- Try a different theme to isolate the issue

---

## Quick Reference Card

```
NAVIGATION                    TABS
j/k or ↑/↓  Move up/down     1-5    Direct tab access
g/G         First/Last       [/]    Prev/Next tab
Ctrl+d/u    Half-page        Tab    Next tab
f+letter    Jump to letter

SELECTION                     ACTIONS
Space       Toggle select    i      Install
Ctrl+a      Select all       D      Delete/Uninstall
x           Clear selection  u      Update
                             p      Cycle version policy
                             Enter  Details popup
                             r      Refresh

MODES                         OTHER
/           Search mode      ?      Help
:           Command mode     c      Config menu
Esc         Exit mode/popup  t      Cycle theme
                             q      Quit

VERSION INDICATORS
↑  Update available (allowed by policy)
⚠  Major update skipped (stable policy)
📌 Tool is pinned (no updates)
```

---

*For CLI commands, see the [User Guide](USER_GUIDE.md). For API usage, see the [API Reference](API.md).*
