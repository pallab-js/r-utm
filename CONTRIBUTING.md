# Contributing to VibeVisor

Thank you for your interest in contributing to VibeVisor! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Submitting Pull Requests](#submitting-pull-requests)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Documentation](#documentation)
- [Release Process](#release-process)

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/r-utm.git
   cd r-utm
   ```
3. **Add the upstream remote**:
   ```bash
   git remote add upstream https://github.com/pallab-js/r-utm.git
   ```
4. **Install dependencies**:
   ```bash
   npm install
   ```

## Development Setup

### Prerequisites

- macOS 14.0+ (Sonoma or later)
- Apple Silicon (M1/M2/M3/M4)
- Node.js 20+
- Rust 1.70+ (with Cargo)
- QEMU 9.0+ (aarch64)

### Running the Development Server

```bash
npm run tauri dev
```

### Running Tests

```bash
# Rust backend tests
cd src-tauri && cargo test

# Frontend type checking
npm run check

# Full verification (run before committing)
npm run check && cd src-tauri && cargo check --lib --bins && cargo test
```

## Making Changes

1. **Create a feature branch** from the latest `main`:
   ```bash
   git fetch upstream
   git checkout main
   git pull upstream main
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following our coding standards

3. **Write or update tests** for your changes

4. **Verify your changes**:
   ```bash
   npm run check && cd src-tauri && cargo test
   ```

## Submitting Pull Requests

### PR Checklist

- [ ] Tests have been added/updated and pass
- [ ] `npm run check` returns 0 errors
- [ ] `cargo test` passes in `src-tauri/`
- [ ] Documentation has been updated
- [ ] Changes are noted in the PR description
- [ ] Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/)

### PR Guidelines

1. **Title**: Use conventional commit format
   - `feat:`, `fix:`, `docs:`, `style:`, `refactor:`, `test:`, `chore:`
   - Example: `feat: add SPICE display compression option`

2. **Description**: Include
   - What changed and why
   - Link to related issues
   - Testing done
   - Screenshots for UI changes

3. **Keep it focused**: One feature/fix per PR

4. **Rebase before submitting**:
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

## Coding Standards

### Rust Backend

- Follow [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Error handling: Use `Result<T, Error>` with `thiserror`
- Async: Use Tokio for async operations
- Documentation: Document all public functions with `///`

### Frontend (Svelte 5)

- Use Svelte 5 Runes (`$state`, `$derived`, `$effect`)
- TypeScript for all new code
- Run `npm run check` before committing
- No nested `<button>` elements (causes hydration warnings)
- Follow existing component patterns

### Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer(s)]
```

**Types**: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `ci`, `build`

**Examples**:
```
feat(vm): add core pinning support
fix(qmp): handle unix socket timeout
docs(security): update vulnerability reporting process
test(launcher): add qmp connection retry test
```

## Testing

### Running Tests

```bash
# All Rust tests
cd src-tauri && cargo test

# Specific module test
cd src-tauri && cargo test -p vibevisor_lib vibe_core::config

# Frontend type checking
npm run check
```

### Writing Tests

- **Rust**: Place tests in the same file as the code, in a `#[cfg(test)]` module
- **Test coverage**: Aim for 80%+ coverage on new code
- **Security tests**: All security functions must have tests

## Documentation

### What to Document

- **Public APIs**: All Tauri commands
- **Complex logic**: Algorithm explanations
- **Architecture decisions**: ADRs for major changes
- **Setup instructions**: New dependencies or steps

### Documentation Files

- `README.md`: Project overview and quick start
- `SECURITY.md`: Security policy and architecture
- `CHANGELOG.md`: Version history (follow [Keep a Changelog](https://keepachangelog.com/))
- `AGENTS.md`: Development guidelines for AI assistants
- Code comments: Why, not what

## Release Process

1. **Version bump**: Update version in `package.json` and `src-tauri/Cargo.toml`
2. **Update CHANGELOG.md**: Move changes from "Unreleased" to version section
3. **Create release tag**: `git tag -a v1.x.x -m "Release v1.x.x"`
4. **Push**: `git push && git push --tags`

### Semantic Versioning

We follow [SemVer](https://semver.org/):
- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

## Areas We Need Help

Good first contributions:
- Documentation improvements
- UI polish and accessibility
- Test coverage increases
- Bug fixes (look for `good-first-issue` label)

Advanced contributions:
- New VM features
- Performance optimizations
- Security enhancements
- QEMU integration improvements

## Questions?

- **General discussion**: Open a GitHub Discussion
- **Bug reports**: Use the bug report template
- **Feature requests**: Open an issue with the feature request template
- **Security issues**: Email security@vibevisor.app (DO NOT open public issues)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
