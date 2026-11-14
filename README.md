# VibeVisor

[![CI](https://github.com/pallab-js/r-utm/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/pallab-js/r-utm/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Lightweight desktop app for running Linux ARM64 VMs on Apple Silicon. Built with **Rust + Tauri 2**, **QEMU** (HVF), and **Svelte 5**.

> macOS 14+ (M1–M4). Linux/aarch64 guests.

## Features

- Native acceleration via Apple Hypervisor.framework (HVF)
- VM lifecycle: create, start, stop, pause, resume, edit, delete
- QCOW2 storage with snapshot support
- SPICE display (localhost-only, per-session password)
- Live CPU/memory metrics
- Import/export `.tar.gz` bundles
- Light, dark, and system theme
- Hardened defaults: path validation, scoped capabilities, strict CSP, rate limiting

## Requirements

- macOS 14+ on Apple Silicon
- [QEMU](https://www.qemu.org/) 9+ (`brew install qemu`)
- Node.js 20+ and Rust (stable)

## Getting Started

```bash
git clone https://github.com/pallab-js/r-utm.git
cd r-utm
npm install

npm run tauri dev    # development
npm run tauri build  # release bundle -> src-tauri/target/release/bundle/
```

## Development

```bash
npm run check                         # Svelte/TS checks
npm run test:ui                       # Playwright UI tests
cd src-tauri && cargo test            # Rust tests
```

Before committing:

```bash
npm run check && cd src-tauri && cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

## Configuration

- VM configs: `~/Library/Application Support/VibeVisor/vms/*.toml`
- App settings: `~/.config/VibeVisor/settings.toml` (defaults, QEMU path, theme, behavior)

## Docs

- [CONTRIBUTING.md](CONTRIBUTING.md) — contributing guide
- [SECURITY.md](SECURITY.md) — reporting vulnerabilities
- [AGENTS.md](AGENTS.md) — project/architecture notes

## License

[MIT](LICENSE) © pallab-js
