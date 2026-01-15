# Hoard v2.0 Implementation Plan

*Created: January 2026*
*Based on: Strategic research and competitive analysis*

---

## Vision

Transform hoard from a CLI tool tracker into the **AI-powered developer tool management platform** - combining multi-source tracking, usage analytics, intelligent discovery, and a modern TUI.

**Tagline:** *"Know what you use. Discover what you need."*

---

## Phase Overview

| Phase | Focus | Duration | Status |
|-------|-------|----------|--------|
| 1 | CLI Simplification | 2-3 weeks | ✅ Complete |
| 2 | AI Enhancements | 2-3 weeks | 🔄 In Progress (2.1, 2.2, 2.4 done) |
| 3 | TUI MVP | 4-6 weeks | 🔲 Not Started |
| 4 | TUI Polish | 2-3 weeks | 🔲 Not Started |

---

## Phase 1: CLI Simplification

**Goal:** Reduce cognitive load from 27 commands to ~15 organized commands + 3 workflows.

### 1.1 Create Unified `sync` Command

**Current state:**
```bash
hoard scan              # Discover tools
hoard sync              # Update status
hoard fetch-descriptions # Get descriptions
hoard gh sync           # GitHub data
hoard usage scan        # Usage tracking
```

**Target state:**
```bash
hoard sync                    # Smart sync (status only)
hoard sync --scan             # Include discovery
hoard sync --github           # Include GitHub data
hoard sync --usage            # Include usage tracking
hoard sync --all              # Everything
hoard sync --dry-run          # Preview changes
```

**Tasks:**
- [x] Add `--scan` flag to sync command
- [x] Add `--github` flag to sync command
- [x] Add `--usage` flag to sync command
- [x] Add `--all` flag combining all operations
- [x] Deprecate standalone `scan` (keep as alias for 1 version)
- [x] Deprecate `fetch-descriptions` (merge into sync)
- [x] Update help text and documentation

---

### 1.2 Create `discover` Command Group

**Current state:**
```bash
hoard list              # List tools
hoard search <query>    # Search
hoard categories        # Show categories
hoard labels            # Show labels
hoard suggest           # Missing tools
hoard gh search         # GitHub search
```

**Target state:**
```bash
hoard discover                      # Interactive discovery menu
hoard discover list [filters]       # List tools (absorbs `list`)
hoard discover search <query>       # Local + GitHub search
hoard discover categories           # Browse by category
hoard discover labels               # Browse by label
hoard discover similar <tool>       # NEW: Find related tools
hoard discover trending             # NEW: Popular tools (GitHub stars)
hoard discover recommended          # Absorbs `recommend`
```

**Tasks:**
- [x] Create `discover` command group in cli.rs
- [x] Move `list` to `discover list` (keep alias)
- [x] Move `search` to `discover search` (keep alias)
- [x] Move `categories` to `discover categories`
- [x] Move `labels` to `discover labels`
- [x] Move `suggest` to `discover missing`
- [x] Move `recommend` to `discover recommended`
- [x] Implement `discover similar <tool>` (same category + labels)
- [x] Implement `discover trending` (top GitHub stars)
- [x] Merge `gh search` into `discover search --github`

---

### 1.3 Create `insights` Command Group

**Current state:**
```bash
hoard usage show        # Usage stats
hoard usage tool <name> # Tool usage
hoard unused            # Unused tools
hoard stats             # Database stats
hoard doctor            # Health check
hoard info              # Database info
```

**Target state:**
```bash
hoard insights                  # Overview dashboard
hoard insights usage [tool]     # Usage statistics
hoard insights unused           # Unused tools
hoard insights health           # Absorbs `doctor`
hoard insights stats            # Absorbs `stats` + `info`
```

**Tasks:**
- [x] Create `insights` command group in cli.rs
- [x] Implement `insights` overview (combined stats)
- [x] Move `usage show` to `insights usage`
- [x] Move `usage tool` to `insights usage <tool>`
- [x] Move `unused` to `insights unused`
- [x] Merge `doctor` + `stats` + `info` into `insights health`
- [x] Deprecate standalone commands (keep aliases for 1 version)

---

### 1.4 Reorganize AI Commands

**Current state:**
```bash
hoard ai set <provider>     # Config
hoard ai show               # Config
hoard ai test               # Config
hoard ai categorize         # Operation
hoard ai describe           # Operation
hoard ai suggest-bundle     # Operation
```

**Target state:**
```bash
# Configuration
hoard ai config set <provider>
hoard ai config show
hoard ai config test

# Operations (renamed to "enrich")
hoard ai enrich                     # Interactive menu
hoard ai enrich --categorize        # Categorize tools
hoard ai enrich --describe          # Generate descriptions
hoard ai enrich --all               # Both operations
hoard ai enrich --dry-run           # Preview changes
```

**Tasks:**
- [x] Create `ai config` subcommand group
- [x] Move `ai set/show/test` to `ai config set/show/test`
- [x] Create `ai enrich` with flags
- [x] Deprecate `ai categorize` and `ai describe` (suggest new commands)
- [x] Move `ai suggest-bundle` to Phase 2 (AI enhancements)

---

### 1.5 Add Workflow Commands

**New commands for common multi-step operations:**

```bash
hoard init
# First-time setup wizard:
# 1. Scan system for tools
# 2. Sync installation status
# 3. Fetch descriptions
# 4. Optionally: GitHub sync, AI categorization
# Interactive prompts guide the user

hoard maintain
# Daily/weekly maintenance:
# 1. Sync status
# 2. Check for updates
# 3. Scan usage
# 4. Show health issues
# Can be run with --auto for non-interactive

hoard cleanup
# Cleanup wizard:
# 1. Show unused tools
# 2. Show orphaned entries
# 3. Fix health issues
# 4. Optionally: Remove unused tools
# Interactive confirmation for destructive actions
```

**Tasks:**
- [x] Implement `init` command with interactive wizard
- [x] Implement `maintain` command with `--auto` flag
- [x] Implement `cleanup` command with confirmations
- [x] Add progress indicators for multi-step operations
- [x] Update Fish completions for new commands

---

### 1.6 Simplify GitHub Integration

**Current state (6 commands):**
```bash
hoard gh sync
hoard gh fetch <tool>
hoard gh search <query>
hoard gh info <tool>
hoard gh rate-limit
hoard gh backfill
```

**Target state (integrated into other commands):**
```bash
hoard sync --github              # Absorbs gh sync
hoard show <tool>                # Shows GitHub info inline
hoard discover search --github   # Absorbs gh search
hoard insights health            # Shows rate limit status

# Keep only for power users:
hoard gh fetch <tool>            # Force fetch single tool
hoard gh backfill                # Fill from cache
```

**Tasks:**
- [x] Add GitHub info to `show` command output
- [x] Add `--github` flag to `sync`
- [x] Add `--github` flag to `discover search`
- [x] Add rate limit to `insights health`
- [x] Deprecate `gh sync`, `gh search`, `gh info`, `gh rate-limit`
- [x] Keep `gh fetch` and `gh backfill` for advanced use

---

### 1.7 Update Documentation & Completions

**Tasks:**
- [x] Update USER_GUIDE.md with new command structure
- [x] Update API.md with new exports
- [x] Update README.md quick start
- [x] Rewrite Fish completions for new structure
- [x] Add deprecation warnings for old commands
- [x] Create migration guide for existing users

---

## Phase 2: AI Enhancements

**Goal:** Add AI-powered features that differentiate hoard from competitors.

### 2.1 GitHub README Extraction

```bash
hoard ai extract <github-url>
hoard ai extract https://github.com/BurntSushi/ripgrep

# Output:
# Extracted from README:
#   Name: ripgrep
#   Binary: rg
#   Source: cargo
#   Install: cargo install ripgrep
#   Description: ripgrep recursively searches directories...
#   Category: search (detected)
#
# Add to database? [Y/n]
```

**Implementation:**
1. Fetch README.md via GitHub API
2. Send to Claude with extraction prompt
3. Parse structured response
4. Validate extracted data
5. Optionally add to database

**Tasks:**
- [x] Create extraction prompt template
- [x] Implement GitHub README fetching
- [x] Implement AI extraction with Claude
- [x] Parse and validate response
- [x] Add interactive confirmation
- [x] Handle edge cases (no README, multiple install methods)
- [x] Cache extractions to avoid repeat API calls

---

### 2.2 Smart Bundle Suggestions

```bash
hoard ai suggest-bundles

# Output:
# Based on your usage patterns:
#
# 📦 "Modern Unix" Bundle
#    You use ripgrep (847x) and fd (423x) heavily.
#    Suggested additions:
#    • eza - modern ls replacement (12K ★)
#    • zoxide - smarter cd (22K ★)
#    • dust - intuitive du (8K ★)
#    [c]reate  [i]nstall all  [s]kip
#
# 📦 "Git Power Tools" Bundle
#    You use git (2341x) and delta (156x).
#    Suggested additions:
#    • lazygit - TUI for git (45K ★)
#    • gh - GitHub CLI (47K ★)
#    [c]reate  [i]nstall all  [s]kip
```

**Implementation:**
1. Analyze installed tools and usage patterns
2. Send context to Claude
3. Get bundle suggestions with reasoning
4. Present interactive menu
5. Create bundle and/or install tools

**Tasks:**
- [x] Create bundle suggestion prompt template
- [x] Gather context (installed tools, usage, categories)
- [x] Implement AI suggestion call
- [x] Parse bundle suggestions
- [x] Implement interactive selection UI
- [x] Connect to bundle create/install commands

---

### 2.3 Contextual Tool Discovery

```bash
hoard ai discover "I'm setting up a Kubernetes development environment"

# Output:
# For Kubernetes development, I recommend:
#
# Essential:
#   kubectl     - Kubernetes CLI (installed ✓)
#   k9s         - TUI for Kubernetes (25K ★)
#   helm        - Package manager for K8s (27K ★)
#
# Productivity:
#   kubectx     - Switch contexts easily (18K ★)
#   stern       - Multi-pod log tailing (7K ★)
#   k3d         - Local K8s clusters (5K ★)
#
# [i]nstall selected  [b]undle all  [s]how details
```

**Tasks:**
- [ ] Create discovery prompt template
- [ ] Implement natural language query handling
- [ ] Query GitHub for tool popularity
- [ ] Present categorized suggestions
- [ ] Allow batch installation

---

### 2.4 Tool Cheatsheet Generation

```bash
hoard ai cheatsheet ripgrep

# Output:
# ┌─────────────────────────────────────────┐
# │ ripgrep (rg) - Fast grep replacement    │
# ├─────────────────────────────────────────┤
# │ BASIC USAGE                             │
# │   rg pattern              Search files  │
# │   rg -i pattern           Ignore case   │
# │   rg -w pattern           Whole words   │
# │                                         │
# │ FILE FILTERING                          │
# │   rg -t rust pattern      Rust files    │
# │   rg -g '*.md' pattern    Glob pattern  │
# │   rg --hidden pattern     Hidden files  │
# │                                         │
# │ OUTPUT                                  │
# │   rg -c pattern           Count matches │
# │   rg -l pattern           Files only    │
# │   rg -C 3 pattern         3 lines ctx   │
# └─────────────────────────────────────────┘
```

**Tasks:**
- [x] Create cheatsheet prompt template
- [x] Fetch --help output for tool
- [x] Generate concise cheatsheet with AI
- [x] Format for terminal display
- [x] Cache generated cheatsheets

---

### 2.5 Usage Analysis & Tips

```bash
hoard ai analyze

# Output:
# 📊 Usage Analysis
#
# 💡 Optimization Tips:
#
# 1. You use `grep` (234x) but have `ripgrep` installed.
#    ripgrep is 10x faster. Consider: alias grep='rg'
#
# 2. You use `find` (89x) but have `fd` installed.
#    fd is simpler and faster. Try: fd <pattern>
#
# 3. You use `cat` (567x) but have `bat` installed.
#    bat adds syntax highlighting. Consider: alias cat='bat'
#
# 4. High-value unused tools:
#    • zoxide (0 uses) - smarter cd, learns your habits
#    • delta (0 uses) - better git diffs
```

**Tasks:**
- [ ] Analyze usage patterns for inefficiencies
- [ ] Detect traditional vs modern tool usage
- [ ] Generate actionable recommendations
- [ ] Identify underutilized installed tools

---

### 2.6 Migration Assistant

```bash
hoard ai migrate --from apt --to cargo

# Output:
# 🔄 Migration Analysis: apt → cargo
#
# These tools have newer versions on cargo:
#
# Tool      apt ver    cargo ver   Benefit
# ────────────────────────────────────────
# bat       0.22.1     0.24.0      +themes, +paging
# fd        8.7.0      10.1.0      +2x speed
# ripgrep   13.0.0     14.1.0      +PCRE2
#
# Migration plan:
#   1. cargo install bat fd-find ripgrep
#   2. sudo apt remove bat fd-find ripgrep
#   3. Update database sources
#
# [m]igrate all  [s]elect  [c]ancel
```

**Tasks:**
- [ ] Compare versions across sources
- [ ] Identify migration candidates
- [ ] Generate migration plan
- [ ] Execute migration with rollback capability

---

### 2.7 Real-time Usage Tracking ✅

**Status:** COMPLETED

Shell hooks for real-time command tracking, eliminating the need for periodic history scans.

```bash
# Configure tracking mode
hoards usage config --mode hook

# Output:
# > Switching to hook mode...
# > Detected shell: zsh
#
# ? Add hook to ~/.zshrc automatically? [Y/n] y
#
# > Adding hook to ~/.zshrc...
# + Hook added successfully!
# + Configuration saved.
```

**Commands:**
```bash
hoards usage config              # View/change tracking mode
hoards usage config --mode scan  # Use history scanning
hoards usage config --mode hook  # Use shell hooks
hoards usage init [shell]        # Show/setup hook instructions
hoards usage log <cmd>           # Log a command (called by hook)
hoards usage reset [-f]          # Reset all counters
```

**Implementation:**
- [x] Add `UsageConfig` and `UsageMode` to config
- [x] Add `usage log` command for shell hooks
- [x] Add `usage init` command for setup instructions
- [x] Add `usage config` command for mode management
- [x] Add `usage reset` command for counter reset
- [x] Automatic shell hook setup for Fish, Zsh, Bash
- [x] Automatic bash-preexec download and installation
- [x] Idempotent setup (detects existing hooks)
- [x] Add `match_command_to_tool()` DB method for fast lookup

**Shell Support:**
| Shell | Config File | Hook Setup |
|-------|-------------|------------|
| Fish | `~/.config/fish/config.fish` | Automatic |
| Zsh | `~/.zshrc` | Automatic |
| Bash | `~/.bashrc` + `~/.bash-preexec.sh` | Automatic (downloads bash-preexec) |

---

## Phase 3: TUI MVP

**Goal:** Build a functional terminal UI using Ratatui.

### 3.1 Project Setup

**Tasks:**
- [ ] Add ratatui and crossterm dependencies
- [ ] Create `src/tui/` module structure
- [ ] Set up basic app state management
- [ ] Implement terminal initialization/cleanup
- [ ] Add `hoard tui` command entry point

**File structure:**
```
src/tui/
├── mod.rs          # Module exports
├── app.rs          # App state and logic
├── ui.rs           # UI rendering
├── event.rs        # Event handling
├── widgets/        # Custom widgets
│   ├── tool_list.rs
│   ├── tool_details.rs
│   └── status_bar.rs
└── tabs/           # Tab views
    ├── installed.rs
    ├── available.rs
    ├── updates.rs
    └── bundles.rs
```

---

### 3.2 Core Layout

```
┌─────────────────────────────────────────────────────────────┐
│ hoard  [1]Installed [2]Available [3]Updates [4]Bundles  [?] │
├────────────────────────┬────────────────────────────────────┤
│ Tools            [147] │ Details                            │
│ ────────────────────── │ ──────────────────────────────────│
│                        │                                    │
│  (list widget)         │  (details widget)                  │
│                        │                                    │
├────────────────────────┴────────────────────────────────────┤
│ (status bar with keybindings)                               │
└─────────────────────────────────────────────────────────────┘
```

**Tasks:**
- [ ] Implement main layout with constraints
- [ ] Create header with tab bar
- [ ] Create left panel (tool list)
- [ ] Create right panel (details)
- [ ] Create footer (status/help bar)
- [ ] Implement responsive resizing

---

### 3.3 Navigation & Input

**Tasks:**
- [ ] Implement vim-style navigation (j/k/h/l)
- [ ] Implement tab switching (1-4, [, ])
- [ ] Implement selection (space, v for range)
- [ ] Implement search mode (/)
- [ ] Implement help modal (?)
- [ ] Handle terminal resize events

---

### 3.4 Tab Views

**Installed Tab:**
- [ ] List installed tools with status indicators
- [ ] Show source, version, usage count
- [ ] Sort by name/usage/date
- [ ] Filter by category/label/source

**Available Tab:**
- [ ] List tools in database but not installed
- [ ] Show GitHub stars, descriptions
- [ ] Quick install action

**Updates Tab:**
- [ ] List tools with available updates
- [ ] Show current vs available version
- [ ] Batch update selection

**Bundles Tab:**
- [ ] List bundles with tool counts
- [ ] Show bundle contents
- [ ] Quick install bundle action

---

### 3.5 Actions

**Tasks:**
- [ ] Implement install action (i)
- [ ] Implement uninstall action (d)
- [ ] Implement update action (u)
- [ ] Implement edit action (e)
- [ ] Implement refresh action (r)
- [ ] Show confirmation dialogs for destructive actions
- [ ] Show progress indicators for long operations

---

## Phase 4: TUI Polish

**Goal:** Add advanced features and polish.

### 4.1 Visual Enhancements

**Tasks:**
- [ ] Add usage sparklines (7-day trend)
- [ ] Add health indicators (🟢🟡🔴)
- [ ] Add GitHub stars inline
- [ ] Implement theme support (Catppuccin, etc.)
- [ ] Add loading spinners
- [ ] Add success/error notifications

---

### 4.2 Advanced Features

**Tasks:**
- [ ] Implement undo/redo system
- [ ] Add command palette (:)
- [ ] Implement fuzzy search (fzf-style)
- [ ] Add mouse support (optional)
- [ ] Implement bulk operations UI
- [ ] Add AI assistant panel

---

### 4.3 Configuration

**Tasks:**
- [ ] Add TUI config file support
- [ ] Configurable keybindings
- [ ] Configurable colors/theme
- [ ] Configurable default view
- [ ] Persist window state

---

## Success Metrics

### Phase 1
- [x] Command count reduced from 27 to ~15
- [x] All commands have `--help` with examples
- [x] Fish completions fully updated
- [x] No breaking changes (aliases work)

### Phase 2
- [x] AI extraction works for 90%+ of GitHub repos
- [ ] Bundle suggestions rated useful by users
- [ ] Cheatsheets generated in <2 seconds
- [x] Real-time usage tracking via shell hooks
- [x] Auto-install shell completions (Fish, Bash, Zsh) during `hoards init`

### Phase 3
- [ ] TUI launches in <100ms
- [ ] All core operations available in TUI
- [ ] Responsive on 80x24 minimum terminal

### Phase 4
- [ ] Theme switching works
- [ ] Undo/redo for all destructive operations
- [ ] User satisfaction in feedback

---

## Technical Debt & Cleanup

**During implementation:**
- [ ] Add integration tests for new commands
- [ ] Update all documentation
- [ ] Remove deprecated code after 1 version
- [x] Ensure 0 clippy warnings maintained
- [x] Keep test count growing (currently 118)
- [x] Pre-commit hooks for code quality
- [x] Add cargo-deny for dependency auditing

---

## Technical Debt Audit (January 2026)

### Summary

| Metric | Value | Status |
|--------|-------|--------|
| Total Lines of Code | 13,564 | - |
| Test Count | 118 tests | ✅ Good |
| God Modules (>1500 lines) | 2 | 🔴 Needs work |
| Unwrap Calls | 53 (1 risky) | ✅ Low risk |
| Security Vulnerabilities | 0 | ✅ Clean |
| Outdated Dependencies | 0 | ✅ Current |

**Overall Debt Score: MEDIUM** - Well-structured codebase with two organizational issues.

### Critical Issues

#### 1. God Modules

| File | Lines | Issue |
|------|-------|-------|
| `src/db.rs` | 1,701 | 11 different concerns mixed together |
| `src/main.rs` | 1,607 | All CLI routing + command logic |

**Recommended Action:** Split into focused modules (see roadmap below).

#### 2. Test Gap

`src/main.rs` has 0 unit tests (1,607 lines untested at unit level).
Integration tests via database tests provide some coverage.

### Positive Findings

- ✅ No security vulnerabilities (cargo audit clean)
- ✅ All dependencies at latest versions
- ✅ All licenses MIT-compatible
- ✅ No TODO/FIXME comments
- ✅ No circular dependencies
- ✅ No unsafe code blocks
- ✅ Good test coverage in core modules (db, models, sources)

### Remediation Roadmap

#### Quick Wins (This Sprint)
- [x] Add `deny.toml` for dependency auditing
- [ ] Fix unwrap in `src/updates.rs:39`
- [ ] Add file size warnings to CI

#### Short-Term (Next 2 Sprints)
- [ ] Split `src/db.rs` into focused modules:
  ```
  src/db/
  ├── mod.rs          (re-exports, Database struct)
  ├── tools.rs        (tool CRUD)
  ├── bundles.rs      (bundle operations)
  ├── configs.rs      (config operations)
  ├── labels.rs       (label operations)
  ├── github.rs       (github data)
  ├── usage.rs        (usage tracking)
  ├── extractions.rs  (AI extraction cache)
  └── schema.rs       (table definitions)
  ```
- [ ] Extract command routing from `src/main.rs`

#### Long-Term (Next Quarter)
- [ ] Create output formatting abstraction (`ui::` module)
- [ ] Add comprehensive integration tests for main.rs
- [ ] Target 80% overall test coverage

### Prevention Measures

**Code Review Checklist:**
- No new files >500 lines
- No new functions >50 lines
- Tests required for new functionality
- No new `unwrap()` in production code

**CI Quality Gates:**
- `cargo deny check` for dependencies
- `cargo audit` for security
- `cargo clippy` for linting
- File size monitoring

---

## Open Questions

1. **Backwards compatibility:** How long to maintain aliases?
2. **AI provider:** Default to Claude? Support local models?
3. **TUI as default?** Should `hoard` without args launch TUI?
4. **Sync daemon?** Background process for auto-updates?

---

## Appendix: Command Migration Guide

| Old Command | New Command | Status |
|-------------|-------------|--------|
| `hoard scan` | `hoard sync --scan` | Alias kept |
| `hoard list` | `hoard discover list` | Alias kept |
| `hoard search` | `hoard discover search` | Alias kept |
| `hoard categories` | `hoard discover categories` | Deprecated |
| `hoard labels` | `hoard discover labels` | Deprecated |
| `hoard suggest` | `hoard discover missing` | Deprecated |
| `hoard recommend` | `hoard discover recommended` | Deprecated |
| `hoard usage show` | `hoard insights usage` | Deprecated |
| `hoard unused` | `hoard insights unused` | Alias kept |
| `hoard stats` | `hoard insights stats` | Deprecated |
| `hoard doctor` | `hoard insights health` | Deprecated |
| `hoard info` | `hoard insights stats` | Deprecated |
| `hoard ai set` | `hoard ai config set` | Deprecated |
| `hoard ai show` | `hoard ai config show` | Deprecated |
| `hoard ai test` | `hoard ai config test` | Deprecated |
| `hoard ai categorize` | `hoard ai enrich --categorize` | Deprecated |
| `hoard ai describe` | `hoard ai enrich --describe` | Deprecated |
| `hoard gh sync` | `hoard sync --github` | Deprecated |
| `hoard gh search` | `hoard discover search --github` | Deprecated |
| `hoard gh info` | `hoard show <tool>` | Deprecated |
| `hoard gh rate-limit` | `hoard insights health` | Deprecated |
