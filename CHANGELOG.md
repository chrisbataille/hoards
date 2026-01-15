# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/chrisbataille/hoards/compare/v0.1.3...v0.2.0) - 2026-01-15

### Documentation

- add technical debt audit findings to implementation plan
- add ai extract to all documentation
- mark Phase 2.1 (ai extract) as complete
- update documentation for new command structure
- update README and add community health files ([#9](https://github.com/chrisbataille/hoards/pull/9))

### Features

- add AI-powered tool cheatsheet generation
- enhance AI bundle suggestions with usage patterns ([#13](https://github.com/chrisbataille/hoards/pull/13))
- add shell completion auto-install command ([#12](https://github.com/chrisbataille/hoards/pull/12))
- add real-time usage tracking via shell hooks
- add ai extract command for GitHub README extraction

### Miscellaneous

- remove .serena/ from gitignore (already tracked)
- add cargo-deny configuration for dependency auditing

### Refactoring

- migrate extraction cache from JSON files to SQLite

## [0.1.3](https://github.com/chrisbataille/hoards/compare/v0.1.2...v0.1.3) - 2026-01-14

### Bug Fixes

- correct CLI name from hoard to hoards ([#4](https://github.com/chrisbataille/hoards/pull/4))

### Features

- add pre-commit hooks for code quality ([#5](https://github.com/chrisbataille/hoards/pull/5))

## [0.1.2] - 2025-01-14

### Added
- Shell completions command using clap_complete
- Config/dotfiles management commands (link, unlink, sync, status)
- Initial project structure

### Features
- Multi-source tool tracking (cargo, apt, pip, npm, brew, flatpak)
- Usage analytics via shell history parsing
- AI integration for categorization and discovery
- GitHub sync for stars, descriptions, topics
- Bundle management for grouping related tools
