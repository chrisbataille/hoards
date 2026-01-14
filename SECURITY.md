# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in Hoards, please report it responsibly.

### How to Report

1. **Do NOT** open a public GitHub issue for security vulnerabilities
2. Email the maintainer directly at **chris@chrisbataille.com**
3. Include as much detail as possible:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment**: Within 48 hours of your report
- **Initial Assessment**: Within 1 week
- **Resolution Timeline**: Depends on severity, typically 2-4 weeks

### Scope

This security policy covers:

- The `hoards` CLI application
- Database handling and storage
- Shell history parsing
- External API integrations (GitHub, AI providers)

### Out of Scope

- Third-party dependencies (report to their maintainers)
- Issues in your local shell configuration
- Social engineering attacks

## Security Best Practices for Users

1. **API Keys**: Never commit AI provider API keys to version control
2. **Database**: The SQLite database is stored locally; protect your home directory
3. **Shell History**: Hoards reads shell history files; ensure they don't contain secrets
4. **GitHub Token**: Use fine-grained tokens with minimal required permissions

Thank you for helping keep Hoards secure!
