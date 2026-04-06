# VibeVisor

## Commands

```bash
npm run tauri dev       # Dev server
npm run tauri build     # Production build
npm run check           # Svelte type check (0 errors required)
cd src-tauri && cargo test  # Run Rust unit tests
```

**Before committing:** `npm run check && cd src-tauri && cargo check --lib --bins && cargo test`

## Architecture

- **Rust backend**: `src-tauri/src/vibe_core/` (config, launcher, qmp, storage, monitor, kernel, cores, error)
- **Frontend**: `src/routes/+page.svelte` + `src/lib/components/`
- **Tauri commands**: `src-tauri/src/lib.rs` with `#[tauri::command]`

## Tauri 2 Quirks

- Schema v2: `$schema: "https://schema.tauri.app/config/2"`
- No `devtools` field (deprecated)
- `tray-icon` feature in `Cargo.toml`, not config
- Plugins: dialog, fs, opener (use `@tauri-apps/plugin-*`)

## Rust Modules

- Import via `crate::vibe_core::*`
- Module names in `mod.rs`: `pub mod launcher;` (not `pub mod vibe_launcher;`)

## Svelte 5

- `$state()` for reactive state, `$effect()` for side effects
- No nested `<button>` elements (causes hydration warnings)
- Invoke Tauri: `invoke()` from `@tauri-apps/api/core`

## Tauri Commands

| Command | Purpose |
|---------|---------|
| `list_vms`, `create_vm`, `delete_vm` | VM CRUD |
| `start_vm`, `stop_vm`, `pause_vm`, `resume_vm` | Lifecycle |
| `update_vm` | Edit VM config |
| `export_vm`, `import_vm` | Migration |
| `get_vm_metrics`, `get_all_metrics` | Monitoring |
| `get_cpu_topology`, `suggest_core_allocation` | P/E-core |
| `get_spice_port`, `get_settings`, `save_settings` | Utilities |

## Security

- CSP enabled (see `tauri.conf.json`)
- Input validation on all user inputs
- Path traversal prevention in import/export
- SPICE authentication with auto-generated passwords

## Storage

- VM configs: `~/Library/Application Support/VibeVisor/vms/*.toml`
- App settings: `~/.config/VibeVisor/settings.toml`
- QCOW2 disks: `~/Library/Application Support/VibeVisor/vms/{vm-id}.qcow2`

## QEMU Search Order (lib.rs)

1. `/usr/local/bin/`
2. `/opt/homebrew/bin/`
3. `src-tauri/sidecars/qemu/`
4. System PATH

## Build Requirements

- macOS 14.0+, Apple Silicon
- QEMU 9.0+ with aarch64
- Node.js 20+

## Entitlements

`com.apple.security.hypervisor` at `src-tauri/vibe.entitlements`
