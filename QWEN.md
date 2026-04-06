# VibeVisor - Project Context

## Project Overview

**VibeVisor** is a high-performance, memory-secure virtualization application engineered specifically for macOS on Apple Silicon (M1/M2/M3/M4). It provides a professional-grade VM management interface using a curated QEMU backend with Rust-based control logic.

### Key Features
- **Apple Silicon Native**: Leverages Hypervisor.framework (HVF) for near-native performance
- **Rust Backend**: Memory-safe, async orchestration with Tokio
- **Modern UI**: Svelte 5 with Runes + Tailwind CSS, dark mode aesthetic
- **SPICE Display**: High-performance remote display protocol
- **QCOW2 Storage**: Snapshot support, linked clones
- **P/E-Core Allocation**: Intelligent core allocation for Apple's hybrid architecture
- **Security Hardened**: Production-ready with comprehensive security measures

### Architecture
- **Frontend**: Svelte 5 + SvelteKit + Tailwind CSS (SPA mode via adapter-static)
- **Backend**: Rust with Tauri 2 framework
- **Communication**: Tauri commands via IPC (invoke/call pattern)
- **Virtualization**: QEMU with HVF acceleration

## Building and Running

### Prerequisites
- macOS 14.0+ (Sonoma or later)
- Apple Silicon (M1/M2/M3/M4)
- Xcode Command Line Tools
- Node.js 20+
- Rust 1.75+
- QEMU 9.0+ (aarch64)

### Development Commands

```bash
# Install dependencies
npm install

# Run development server (starts both Vite and Tauri)
npm run tauri dev

# Build for production
npm run tauri build

# Type check frontend (0 errors required)
npm run check

# Watch mode for type checking
npm run check:watch

# Run Rust unit tests
cd src-tauri && cargo test

# Check Rust compilation
cd src-tauri && cargo check

# Format Rust code
cd src-tauri && cargo fmt

# Run Rust linter
cd src-tauri && cargo clippy
```

### Pre-commit Checklist
```bash
npm run check && cd src-tauri && cargo check --lib --bins && cargo test
```

## Project Structure

```
r-utm/
├── src/                              # Svelte 5 frontend
│   ├── app.css                       # Tailwind CSS
│   ├── app.html                      # HTML template
│   ├── routes/                       # SvelteKit routes
│   │   ├── +page.svelte             # Main VM management UI
│   │   └── +layout.svelte           # Layout wrapper
│   └── lib/
│       ├── components/              # UI components
│       │   ├── Chart.svelte         # Metrics visualization
│       │   ├── CreateVmWizard.svelte
│       │   ├── EditVmModal.svelte
│       │   ├── Settings.svelte
│       │   └── SpiceDisplay.svelte
│       └── stores/                  # Svelte stores (if any)
│
├── src-tauri/                       # Rust backend
│   ├── src/
│   │   ├── lib.rs                   # Tauri commands + AppState
│   │   ├── main.rs                  # Entry point
│   │   └── vibe_core/              # Core backend modules
│   │       ├── mod.rs               # Module exports
│   │       ├── config.rs            # TOML VM configuration
│   │       ├── launcher.rs          # QEMU process launcher
│   │       ├── qmp.rs               # QMP client (Unix sockets)
│   │       ├── storage.rs           # QCOW2 management
│   │       ├── monitor.rs           # VM monitoring
│   │       ├── kernel.rs            # Direct kernel boot
│   │       ├── cores.rs             # P/E-core allocation
│   │       ├── error.rs             # Error types
│   │       ├── security.rs          # Security utilities
│   │       └── rate_limiter.rs      # Rate limiting
│   ├── sidecars/qemu/              # Curated QEMU binaries
│   ├── icons/                       # App icons
│   ├── vibe.entitlements           # macOS entitlements
│   ├── tauri.conf.json             # Tauri configuration
│   └── Cargo.toml                   # Rust dependencies
│
├── static/                          # Static assets
├── build/                           # Build output
├── package.json                     # Node.js dependencies
├── svelte.config.js                 # SvelteKit configuration
├── tsconfig.json                    # TypeScript configuration
├── vite.config.js                   # Vite configuration
│
└── Documentation/
    ├── README.md                    # Project overview
    ├── AGENTS.md                    # Development guidelines
    ├── CONTRIBUTING.md              # Contribution guide
    ├── SECURITY.md                  # Security policy
    ├── CHANGELOG.md                 # Version history
    └── SECURITY_HARDENING_COMPLETE.md  # Security audit results
```

## Key Tauri Commands

| Command | Purpose |
|---------|---------|
| `list_vms` | List all VMs with state |
| `create_vm` | Create new VM |
| `delete_vm` | Delete VM and its disk |
| `start_vm` | Start a VM |
| `stop_vm` | Stop a VM via QMP |
| `pause_vm` | Pause a running VM |
| `resume_vm` | Resume a paused VM |
| `update_vm` | Update VM configuration |
| `export_vm` | Export VM to archive |
| `import_vm` | Import VM from archive |
| `get_vm_metrics` | Get VM resource metrics |
| `get_all_metrics` | Get metrics for all VMs |
| `get_cpu_topology` | Get Apple Silicon CPU topology |
| `suggest_core_allocation` | Suggest P/E-core allocation |
| `get_spice_port` | Get SPICE display port |
| `get_settings` | Get app settings |
| `save_settings` | Save app settings |
| `confirm_and_quit` | Graceful shutdown with VM cleanup |

## Storage Locations

- **VM configs**: `~/Library/Application Support/VibeVisor/vms/*.toml`
- **App settings**: `~/.config/VibeVisor/settings.toml`
- **QCOW2 disks**: `~/Library/Application Support/VibeVisor/vms/{vm-id}.qcow2`

## Development Conventions

### Rust Backend
- Use `crate::vibe_core::*` for importing modules
- Module names in `mod.rs`: `pub mod launcher;` (not `pub mod vibe_launcher;`)
- All operations return `Result<T, Error>` - no `.unwrap()` in production code
- Use `tracing` for logging (not `println!`)
- Error types defined in `vibe_core/error.rs` using `thiserror`
- Async code uses Tokio runtime

### Svelte 5 Frontend
- Use `$state()` for reactive state
- Use `$derived()` for computed values
- Use `$effect()` for side effects
- No nested `<button>` elements (causes hydration warnings)
- Invoke Tauri commands: `invoke()` from `@tauri-apps/api/core`
- Listen to events: `listen()` from `@tauri-apps/api/event`

### Security Requirements
- **Zero npm/cargo vulnerabilities** target
- **0 svelte-check errors** required before commits
- All user inputs validated and sanitized
- Paths canonicalized before use
- Archives inspected before extraction
- Rate limiting on expensive operations
- CSP enabled and configured

### Testing
- 31 unit tests covering security functions, config, cores, etc.
- Run tests: `cd src-tauri && cargo test`
- All tests must pass before merging

## Tauri 2 Specifics

- Schema v2: `$schema: "https://schema.tauri.app/config/2"`
- No `devtools` field (deprecated in v2)
- `tray-icon` feature in `Cargo.toml`, not config
- Plugins: dialog, fs, opener (use `@tauri-apps/plugin-*`)
- Capabilities-based permission system
- Use `Emitter` trait for emitting events to frontend

## QEMU Search Order

1. `/usr/local/bin/`
2. `/opt/homebrew/bin/`
3. `/opt/local/bin/`
4. `src-tauri/sidecars/qemu/`
5. System PATH

## Security Architecture

### Key Security Features
- **Cryptographic passwords**: 64-char CSPRNG passwords via `ring` crate
- **Secure temp files**: QMP sockets in secure dirs with 0o700 permissions
- **Command injection prevention**: Path validation before QEMU args
- **Archive inspection**: Pre-extraction validation (paths, size, suspicious entries)
- **Rate limiting**: Token bucket algorithm on expensive operations
- **Graceful shutdown**: Stops all VMs before exit with confirmation dialog

### Close Button Behavior
- **No running VMs**: Immediate clean exit
- **Running VMs present**: Shows confirmation dialog → Gracefully stops VMs via QMP → Force kills if needed → Exits cleanly
- Terminal always returns to prompt (no Ctrl+C needed)

## Supported Guest OSes

| Guest OS | Status | Notes |
|----------|--------|-------|
| Linux (AArch64) | Stable | Primary target |
| Windows (ARM64) | Experimental | Limited driver support |

## macOS Entitlements

Located at `src-tauri/vibe.entitlements`:
- `com.apple.security.hypervisor` - Required for HVF
- `com.apple.vm.networking` - Required for VM networking

## Common Issues

### QEMU Not Found
Ensure QEMU is in PATH: `which qemu-system-aarch64`

### Hypervisor Entitlement Missing
Should be automatically bundled. Check `vibe.entitlements`.

### SPICE Display Issues
Ensure using SPICE-compatible viewer or built-in display.

## Dependencies

### Rust (Key Dependencies)
- `tauri` v2 - Application framework
- `tokio` - Async runtime
- `ring` - Cryptographic operations
- `tempfile` - Secure temporary files
- `serde` + `toml` - Configuration serialization
- `uuid` - Unique identifiers

### Node.js (Key Dependencies)
- `@tauri-apps/api` v2 - Frontend API
- `@sveltejs/kit` v2.20+ - SvelteKit framework
- `svelte` v5 - UI framework with Runes
- `tailwindcss` v4 - Utility-first CSS

## Version Information

- **App Version**: 1.0.0
- **Last Security Audit**: April 2026
- **Security Status**: ✅ Production-ready (0 vulnerabilities)
- **Tests**: 31 passing
