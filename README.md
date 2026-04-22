# VibeVisor

[![CI](https://github.com/pallab-js/r-utm/actions/workflows/ci.yml/badge.svg)](https://github.com/pallab-js/r-utm/actions/workflows/ci.yml)
[![Security Audit](https://img.shields.io/badge/Security-Audited-brightgreen.svg)](SECURITY.md)
[![Dependencies](https://img.shields.io/badge/Dependencies-0%20Vulnerabilities-brightgreen.svg)](SECURITY.md)
[![Tests](https://img.shields.io/badge/Tests-31%20Passing-brightgreen.svg)](src-tauri/src/vibe_core/)
[![Rust](https://img.shields.io/badge/Rust-Memory%20Safe-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

> High-performance Linux ARM64 virtualization for Apple Silicon (M1/M2/M3/M4) built with Rust and Tauri.

## Overview

VibeVisor is a lightweight, memory-secure virtualization apparatus engineered for running Linux ARM64 VMs on macOS Apple Silicon. It uses a curated QEMU backend with Rust-based control logic for a professional, high-velocity user experience.

**Why VibeVisor?**

- 🚀 **Near-Native Performance**: Leverages Apple's Hypervisor.framework (HVF) for minimal overhead
- 🔒 **Security-First Design**: Comprehensive hardening with CSPRNG, path validation, rate limiting, and sandboxed processes
- 🦀 **Rust Backend**: Memory-safe, async orchestration with zero `.unwrap()` calls in production
- 🎨 **Modern UI**: Svelte 5 with Runes + Tailwind CSS, dark mode aesthetic
- 💾 **QCOW2 Support**: Snapshot capabilities, linked clones, efficient storage

## Features

| Category | Features |
|----------|----------|
| **Performance** | HVF acceleration, P/E-core allocation, direct kernel boot, SPICE display |
| **Security** | CSPRNG passwords, path canonicalization, archive inspection, rate limiting, CSP |
| **Storage** | QCOW2 disk images, snapshots, import/export, linked clones |
| **Monitoring** | Real-time VM metrics, CPU topology detection, resource usage charts |
| **UI/UX** | Svelte 5 reactive components, dark mode, responsive design |

## Requirements

- **macOS**: 14.0+ (Sonoma or later)
- **Hardware**: Apple Silicon (M1/M2/M3/M4)
- **Tools**: Xcode Command Line Tools
- **QEMU**: 9.0+ (aarch64)

## Quick Start

### 1. Install Dependencies

```bash
# Install QEMU (required for VM execution)
brew install qemu

# Or build from source for optimal performance
./src-tauri/sidecars/qemu/build-qemu.sh
```

### 2. Clone & Setup

```bash
git clone https://github.com/pallab-js/r-utm.git
cd r-utm
npm install
```

### 3. Development

```bash
# Start the development server with hot reload
npm run tauri dev
```

### 4. Production Build

```bash
# Build the production application
npm run tauri build
```

The built application will be in `src-tauri/target/release/bundle/`.

## Project Structure

```
r-utm/
├── src/                          # Frontend (Svelte 5)
│   ├── app.css                   # Tailwind CSS styles
│   ├── app.html                  # HTML template
│   ├── routes/                   # SvelteKit routes
│   │   ├── +page.svelte          # Main VM management UI
│   │   └── +layout.svelte        # Layout wrapper
│   └── lib/
│       ├── components/           # Reusable UI components
│       │   ├── Chart.svelte      # Metrics visualization
│       │   ├── CreateVmWizard.svelte
│       │   ├── EditVmModal.svelte
│       │   ├── Settings.svelte
│       │   └── SpiceDisplay.svelte
│       └── stores/
│           └── metrics.ts        # VM metrics store
│
├── src-tauri/                    # Backend (Rust/Tauri 2)
│   ├── src/
│   │   ├── lib.rs                # Tauri commands + AppState
│   │   ├── main.rs               # Entry point
│   │   └── vibe_core/            # Core modules
│   │       ├── config.rs         # TOML VM configuration
│   │       ├── launcher.rs       # QEMU process launcher
│   │       ├── qmp.rs            # QMP client (Unix sockets)
│   │       ├── storage.rs        # QCOW2 management
│   │       ├── monitor.rs        # VM monitoring
│   │       ├── kernel.rs         # Direct kernel boot
│   │       ├── cores.rs          # P/E-core allocation
│   │       ├── error.rs          # Error types
│   │       ├── security.rs       # Security utilities
│   │       └── rate_limiter.rs   # Rate limiting
│   ├── capabilities/
│   │   └── default.json          # Tauri permissions
│   ├── sidecars/qemu/            # QEMU binaries
│   ├── icons/                    # Application icons
│   ├── tauri.conf.json           # Tauri configuration
│   ├── vibe.entitlements         # macOS entitlements
│   └── Cargo.toml                # Rust dependencies
│
├── .github/                      # GitHub configuration
│   ├── workflows/ci.yml          # CI/CD pipeline
│   ├── ISSUE_TEMPLATE/           # Issue templates
│   └── FUNDING.yml               # Sponsorship links
│
├── package.json                  # Node.js configuration
└── vite.config.js                # Vite bundler
```

## Architecture

### Backend (Rust)

The Rust backend provides robust, memory-safe control over QEMU:

```
┌─────────────────────────────────────────────────────────┐
│                    Tauri Commands                        │
│              (lib.rs - Frontend Bridge)                  │
└─────────────────────────┬───────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
    ┌─────▼─────┐  ┌─────▼─────┐  ┌─────▼─────┐
    │ launcher  │  │  config   │  │  storage  │
    │   (QEMU   │  │  (TOML    │  │  (QCOW2   │
    │  spawn)   │  │  parsing) │  │  mgmt)    │
    └─────┬─────┘  └───────────┘  └───────────┘
          │
    ┌─────▼─────┐
    │    qmp    │
    │  (Unix    │
    │  sockets) │
    └───────────┘
```

**Key Modules:**
- **`vibe-launcher`**: Async QEMU process spawning with HVF acceleration
- **`vibe-qmp`**: QEMU Machine Protocol client via Unix Domain Sockets
- **`vibe-storage`**: QCOW2 image management with snapshot support
- **`vibe-config`**: TOML-based VM configuration serialization
- **`vibe-security`**: Cryptographic operations and input validation
- **`vibe-monitor`**: Real-time VM metrics collection
- **`vibe-cores`**: Apple Silicon P/E-core allocation suggestions

### Frontend (Svelte 5)

The frontend uses Svelte 5's new Runes system for fine-grained reactivity:

- **`$state()`**: Reactive state management
- **`$derived()`**: Computed values
- **`$effect()`**: Side effects and subscriptions
- **Tailwind CSS v4**: Utility-first styling with dark mode

## Configuration

### VM Configuration (TOML)

VM configurations are stored in `~/Library/Application Support/VibeVisor/vms/`:

```toml
name = "My Linux VM"
id = "uuid-here"
architecture = "aarch64"

[hardware]
cpu_cores = 4
memory_mb = 4096
cpu_model = "host"

[storage]
disk_path = "/path/to/disk.qcow2"
disk_size_gb = 20
disk_type = "qcow2"
snapshot_enabled = true

[network]
enabled = true
mode = "nat"

[display]
spice_enabled = true
resolution = { width = 1920, height = 1080 }
```

### App Settings

Application settings are stored in `~/.config/VibeVisor/settings.toml` and can be configured via the Settings UI.

## Supported Guest Operating Systems

| Guest OS | Status | Notes |
|----------|--------|-------|
| **Linux (AArch64)** | ✅ Stable | Primary target, recommended |
| **Windows (ARM64)** | 🧪 Experimental | Limited driver support |
| **FreeBSD (AArch64)** | 🧪 Experimental | Community supported |
| **macOS (VM)** | ❌ Not Supported | Licensing restrictions |

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+K` | Open command palette |
| `Esc` | Close modals/dialogs |

## Troubleshooting

### QEMU Not Found

Ensure QEMU is installed and accessible:

```bash
which qemu-system-aarch64
which qemu-img
```

VibeVisor searches for QEMU in this order:
1. `/usr/local/bin/`
2. `/opt/homebrew/bin/` (Homebrew on Apple Silicon)
3. `src-tauri/sidecars/qemu/` (bundled)
4. System `PATH`

### Hypervisor Entitlement Missing

The app requires `com.apple.security.hypervisor` entitlement. This is automatically bundled in production builds.

### SPICE Display Issues

- Ensure you're using a SPICE-compatible viewer or the built-in display
- SPICE is bound to localhost only for security
- Check firewall settings if connecting remotely

### VM Won't Start

1. Check QEMU is installed: `qemu-system-aarch64 --version`
2. Verify VM configuration in `~/Library/Application Support/VibeVisor/vms/`
3. Check logs in the development console or Console.app

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Quick Start for Contributors

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes
4. Run tests: `npm run check && cd src-tauri && cargo test`
5. Commit with conventional commit message: `git commit -m 'feat: add amazing feature'`
6. Push and submit a PR

### Good First Issues

Look for issues labeled [`good-first-issue`](https://github.com/pallab-js/r-utm/labels/good-first-issue) for beginner-friendly tasks.

## Development Commands

```bash
# Frontend
npm run dev              # Start dev server
npm run build            # Build frontend
npm run check            # Type checking (Svelte)

# Backend (Rust)
cd src-tauri && cargo test       # Run tests
cd src-tauri && cargo clippy     # Linting
cd src-tauri && cargo fmt        # Formatting

# Full application
npm run tauri dev        # Development mode
npm run tauri build      # Production build
```

**Before committing:**
```bash
npm run check && cd src-tauri && cargo check --lib --bins && cargo test
```

## Security

VibeVisor implements multiple layers of security hardening:

- 🔐 **Cryptographic Security**: CSPRNG-generated passwords with 325+ bits of entropy
- 🛡️ **Command Injection Prevention**: All paths validated and canonicalized
- 📁 **Secure Temporary Files**: QMP sockets in restricted directories (`0o700`)
- 📦 **Archive Inspection**: Pre-extraction validation prevents malicious imports
- ⏱️ **Rate Limiting**: Token bucket algorithm prevents resource exhaustion
- 🚫 **Zero `.unwrap()` Calls**: Proper error handling throughout

See our [Security Policy](SECURITY.md) for details, including audit history and vulnerability reporting.

## License

This project is licensed under the [MIT License](LICENSE).

## Acknowledgments

- [QEMU](https://www.qemu.org/) - The open-source machine emulator
- [Tauri](https://tauri.app/) - Build smaller, faster, secure desktop apps
- [Svelte](https://svelte.dev/) - Cybernetically enhanced web apps
- [Apple Hypervisor.framework](https://developer.apple.com/documentation/hypervisor) - Native macOS virtualization
- All our [contributors](https://github.com/pallab-js/r-utm/graphs/contributors)

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=pallab-js/r-utm&type=Date)](https://star-history.com/#pallab-js/r-utm&Date)

---

**Made with ❤️ for the Apple Silicon community**
