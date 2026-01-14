# Contributing to Hoards

Thank you for your interest in contributing to Hoards! This document outlines our development workflow and guidelines.

## Git Branching Strategy

We use a simplified Git Flow model with the following branches:

### Branch Structure

| Branch | Purpose | Base |
|--------|---------|------|
| `main` | Stable, release-ready code. Tagged for versions. | - |
| `develop` | Integration branch. Next release prep. | `main` |
| `feature/*` | New features | `develop` |
| `fix/*` | Bug fixes | `develop` |
| `hotfix/*` | Urgent production fixes | `main` |
| `release/*` | Release preparation | `develop` |

### Branch Naming Convention

```
feature/short-description    # New functionality (e.g., feature/add-nix-source)
fix/issue-description        # Bug fixes (e.g., fix/history-parsing-crash)
hotfix/critical-issue        # Production emergencies (e.g., hotfix/db-corruption)
release/vX.Y.Z               # Release candidates (e.g., release/v0.2.0)
```

### Workflow

```
main ←── release/v0.2.0 ←── develop ←── feature/xyz
  ↑
  └── hotfix/critical-bug (urgent fixes bypass develop)
```

#### Feature Development

1. **Create branch** from `develop`:
   ```bash
   git checkout develop
   git pull origin develop
   git checkout -b feature/your-feature-name
   ```

2. **Develop** your feature with atomic commits

3. **Push** and create PR to `develop`:
   ```bash
   git push -u origin feature/your-feature-name
   ```

4. **Merge** via squash merge after approval

#### Bug Fixes

Same as features, but use `fix/` prefix:
```bash
git checkout -b fix/description-of-bug
```

#### Hotfixes (Production Emergencies)

1. Branch from `main`:
   ```bash
   git checkout main
   git pull origin main
   git checkout -b hotfix/critical-issue
   ```

2. Fix the issue, test thoroughly

3. Merge to **both** `main` (with tag) and `develop`:
   ```bash
   # PR to main first, then after merge:
   git checkout develop
   git merge main
   ```

#### Releases

1. **Create release branch** from `develop`:
   ```bash
   git checkout develop
   git checkout -b release/v0.2.0
   ```

2. **Prepare release**:
   - Update version in `Cargo.toml`
   - Update CHANGELOG.md
   - Final testing

3. **Merge to main** and tag:
   ```bash
   # After PR approval and merge to main:
   git checkout main
   git pull
   git tag -a v0.2.0 -m "Release v0.2.0"
   git push origin v0.2.0
   ```

4. **Back-merge to develop**:
   ```bash
   git checkout develop
   git merge main
   git push origin develop
   ```

## Commit Messages

Follow the conventional commits format:

```
type(scope): short description

[optional body]

[optional footer]
```

### Types

| Type | Description |
|------|-------------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `test` | Adding or updating tests |
| `chore` | Maintenance tasks (deps, CI, etc.) |
| `perf` | Performance improvement |

### Examples

```
feat(scanner): add nix package source detection
fix(history): handle empty history files gracefully
docs(readme): add installation instructions for Arch Linux
refactor(db): extract common query patterns
test(ai): add integration tests for Claude provider
chore(deps): update clap to 4.5
```

## Pull Request Requirements

Before submitting a PR, ensure:

- [ ] All tests pass: `cargo test`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Code is formatted: `cargo fmt`
- [ ] Fish completions updated (if CLI changed): `shell/fish/completions/hoards.fish`
- [ ] Commit messages follow convention
- [ ] PR description explains the change

### PR Title Format

Use the same format as commit messages:
```
feat(scope): description
```

### Squash Merge

We use squash merging to keep `develop` and `main` history clean. The PR title becomes the commit message.

## Development Setup

```bash
# Clone and setup
git clone https://github.com/YOUR_USERNAME/hoards.git
cd hoards

# Enable pre-commit hooks (required)
git config core.hooksPath .githooks

# Ensure you're on develop for new work
git checkout develop
git pull origin develop

# Create your feature branch
git checkout -b feature/your-feature
```

### Pre-commit Hooks

We use git hooks to ensure code quality before each commit. The hooks automatically run:

1. `cargo fmt --check` - Format verification
2. `cargo clippy` - Lint checks
3. `cargo test` - Test suite

To enable hooks after cloning:
```bash
git config core.hooksPath .githooks
```

If you need to bypass hooks temporarily (not recommended):
```bash
git commit --no-verify -m "message"
```

## Code Style

See [CLAUDE.md](./CLAUDE.md) for code style guidelines:

- **DRY**: Extract common logic into reusable functions
- **TDD**: Write tests first for new features
- Keep functions focused and single-purpose
- Use builder pattern for structs

## Questions?

Open an issue for discussion before starting large changes.
