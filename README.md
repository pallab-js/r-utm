# VibeVisor

[![CI](https://github.com/pallab-js/r-utm/actions/workflows/ci.yml/badge.svg)](https://github.com/pallab-js/r-utm/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-36%20passing-brightgreen.svg)](src-tauri/src/vibe_core/)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A lightweight desktop app for running Linux ARM64 virtual machines on Apple Silicon.
Built with Rust + Tauri over a QEMU backend, with a Svelte 5 UI.

> **Platform:** macOS 14+ on Apple Silicon (M1–M4). Linux/aarch64 guests only.

---

## Features

- **Native acceleration** — QEMU with Apple's Hypervisor.framework (HVF).
- **Full VM lifecycle** — create, start, stop, pause, resume, edit, delete.
- **QCOW2 storage** — disk images with snapshot support.
- **SPICE display** — localhost-only, with an auto-generated per-session password.
- **Live metrics** — per-VM CPU and memory usage.
- **Import / export** — portable `.tar.gz` bundles with pre-extraction archive inspection.
- **Security hardening** — CSPRNG passwords, path validation, per-command rate limiting, scoped filesystem capabilities, and a strict CSP.

## Requirements

| Dependency | Version |
|------------|---------|
| macOS      | 14.0+ (Apple Silicon) |
| QEMU       | 9.0+ (`qemu-system-aarch64`, `qemu-img`) |
| Node.js    | 20+ |
| Rust       | stable toolchain |

```bash
brew install qemu
```

## Quick Start

```bash
git clone https://github.com/pallab-js/r-utm.git
cd r-utm
npm install

npm run tauri dev      # run in development
npm run tauri build    # build a release bundle
```

Release artifacts are written to `src-tauri/target/release/bundle/`.

## Project Layout

```
r-utm/
├── src/                       # Frontend (SvelteKit + Svelte 5 Runes)
│   ├── routes/+page.svelte    # Main VM management UI
│   └── lib/
│       ├── components/        # Chart, CreateVmWizard, EditVmModal,
│       │                      #   Settings, SpiceDisplay
│       └── stores/metrics.ts  # Reactive metrics store
│
└── src-tauri/                 # Backend (Rust + Tauri 2)
    ├── src/lib.rs             # Tauri commands + AppState
    └── src/vibe_core/
        ├── config.rs          # VM config (TOML) + validation
        ├── launcher.rs        # QEMU process launcher (HVF, SPICE, QMP)
        ├── qmp.rs             # QEMU Machine Protocol client (Unix socket)
        ├── storage.rs         # QCOW2 / snapshot management
        ├── monitor.rs         # Per-VM CPU/memory metrics
        ├── kernel.rs          # Direct kernel boot args
        ├── cores.rs           # Apple Silicon P/E-core allocation
        ├── security.rs        # CSPRNG, path & archive validation
        ├── rate_limiter.rs    # Per-command token-bucket limiter
        └── error.rs           # Error types
```

## Tauri Commands

| Area        | Commands |
|-------------|----------|
| CRUD        | `list_vms`, `create_vm`, `update_vm`, `delete_vm` |
| Lifecycle   | `start_vm`, `stop_vm`, `pause_vm`, `resume_vm`, `get_vm_state` |
| Migration   | `export_vm`, `import_vm` |
| Monitoring  | `get_vm_metrics`, `get_all_metrics` |
| Display     | `get_spice_port`, `get_spice_credentials` |
| CPU         | `get_cpu_topology`, `suggest_core_allocation` |
| Settings    | `get_settings`, `save_settings`, `get_default_settings` |

## Configuration

VM configs are stored as TOML in `~/Library/Application Support/VibeVisor/vms/`:

```toml
name = "My Linux VM"
id = "uuid-here"
architecture = "aarch64"

[hardware]
cpu_cores = 4          # 1–32
memory_mb = 4096       # 256–131072
cpu_model = "host"

[storage]
disk_path = "/path/to/disk.qcow2"
disk_size_gb = 20      # 1–1024
disk_type = "qcow2"
snapshot_enabled = true

[network]
enabled = true
mode = "nat"           # nat | bridged | shared

[display]
spice_enabled = true
resolution = { width = 1920, height = 1080 }
```

App settings live in `~/.config/VibeVisor/settings.toml`.

## Development

```bash
# Frontend
npm run dev            # dev server
npm run check          # Svelte type check

# Backend
cd src-tauri
cargo test             # unit tests
cargo clippy           # lint
cargo fmt              # format
```

Before committing:

```bash
npm run check && cd src-tauri && cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

## Security

- SPICE binds to `127.0.0.1` only, with a CSPRNG-generated ticket password per session.
- All user-supplied paths are canonicalized and confined before being passed to QEMU.
- Import archives are inspected (size + path-traversal checks) before extraction.
- State-changing commands are rate-limited via a token bucket.
- Filesystem access is scoped in `src-tauri/capabilities/default.json`.
- CI runs `cargo audit` and `npm audit` on every push.

See [SECURITY.md](SECURITY.md) to report a vulnerability.

## Troubleshooting

**QEMU not found** — ensure `qemu-system-aarch64` and `qemu-img` are on your `PATH`.
VibeVisor searches `/usr/local/bin`, `/opt/homebrew/bin`, the bundled `sidecars/qemu/`, then `PATH`.

**VM won't start** — confirm QEMU runs (`qemu-system-aarch64 --version`) and check the config
in `~/Library/Application Support/VibeVisor/vms/`.

**SPICE won't connect** — the port and password are generated at launch; retrieve them via
`get_spice_credentials`. SPICE is localhost-only by design.

## Contributing

Contributions are welcome — see [CONTRIBUTING.md](CONTRIBUTING.md).
Fork, branch, run the pre-commit checks above, and open a PR with a
[conventional commit](https://www.conventionalcommits.org/) title.

## License

[MIT](LICENSE) © pallab-js

## Acknowledgments

[QEMU](https://www.qemu.org/) · [Tauri](https://tauri.app/) · [Svelte](https://svelte.dev/) · [Apple Hypervisor.framework](https://developer.apple.com/documentation/hypervisor)
