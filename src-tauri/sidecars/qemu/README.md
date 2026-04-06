# QEMU Sidecar for VibeVisor

This directory contains the curated QEMU binary optimized for Apple Silicon.

## Quick Start

### Option 1: Pre-built Binary (Recommended)
Download a pre-built `qemu-system-aarch64` from our releases or build with:

```bash
# Build from source
chmod +x build-qemu.sh
./build-qemu.sh

# Copy binaries to this directory
cp output/bin/* ./
```

### Option 2: Homebrew
```bash
brew install qemu
cp "$(brew --prefix qemu)/bin/qemu-system-aarch64" ./
cp "$(brew --prefix qemu)/bin/qemu-img" ./
```

## Build Requirements

- macOS 14.0+ (Sonoma or later)
- Apple Silicon (M1/M2/M3/M4)
- Xcode Command Line Tools
- ~4GB free disk space

## Build from Source

```bash
chmod +x build-qemu.sh
./build-qemu.sh
```

This will:
1. Download QEMU 9.0.2
2. Configure with Apple Silicon optimizations
3. Build `qemu-system-aarch64` and `qemu-img`
4. Strip debug symbols to reduce size

## Expected Output

After build, you should have:
```
qemu/
├── build-qemu.sh
├── README.md
├── qemu-system-aarch64  (~120MB stripped)
└── qemu-img             (~8MB stripped)
```

## Features Enabled

- **HVF**: Apple Hypervisor.framework acceleration
- **SPICE**: Remote display protocol
- **virtio-fs**: Shared filesystem (host <-> guest)
- **libusb**: USB device passthrough

## Features Disabled

- Audio subsystems
- VNC (SPICE preferred)
- SDL/GTK UI
- Legacy disk formats (qcow1, vdi, vmdk, etc.)
- Non-aarch64 targets
