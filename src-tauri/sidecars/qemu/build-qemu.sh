#!/bin/bash
set -e

QEMU_VERSION="9.0.2"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BUILD_DIR="$SCRIPT_DIR/build"
OUTPUT_DIR="$SCRIPT_DIR/output"

echo "========================================"
echo "VibeVisor Curated QEMU Build"
echo "Apple Silicon (M1/M2/M3) Optimization"
echo "========================================"

cd "$SCRIPT_DIR"

if [ -d "$BUILD_DIR" ]; then
    echo "Cleaning previous build..."
    rm -rf "$BUILD_DIR" "$OUTPUT_DIR"
fi

mkdir -p "$BUILD_DIR" "$OUTPUT_DIR"

if [ ! -f "qemu-${QEMU_VERSION}.tar.xz" ]; then
    echo "Downloading QEMU $QEMU_VERSION..."
    curl -fSL "https://download.qemu.org/qemu-${QEMU_VERSION}.tar.xz" -o "qemu-${QEMU_VERSION}.tar.xz"
fi

echo "Extracting..."
tar xf "qemu-${QEMU_VERSION}.tar.xz"
cd "qemu-${QEMU_VERSION}"

echo "Configuring..."
./configure \
    --prefix="$OUTPUT_DIR" \
    --target-list=aarch64-softmmu \
    --enable-hvf \
    --enable-spice \
    --enable-virtfs \
    --enable-libusb \
    --disable-werror \
    --disable-debug \
    --disable-sdl \
    --disable-gtk \
    --disable-sdl-image \
    --disable-vnc \
    --disable-curses \
    --disable-bochs \
    --disable-dmg \
    --disable-qcow1 \
    --disable-vdi \
    --disable-vhdx \
    --disable-vmdk \
    --disable-qed \
    --disable-parallels \
    --disable-nbd \
    --disable-glusterfs \
    --disable-libiscsi \
    --disable-libnfs \
    --disable-rbd \
    --disable-smartcard \
    --disable-usb-redir \
    --disable-bsd-user \
    --disable-linux-user \
    --disable-windows-user \
    --disable-xen \
    --disable-capstone \
    --disable-kvm \
    --disable-whpx \
    --disable-docs \
    --disable-tools \
    --disable-system \
    --enable-modules \
    --with-sdlabi=2.0 \
    --with-coroutine=ucontext \
    --audio-drv-list= \
    --disable-pa \
    --disable-alsa \
    --disable-oss \
    --disable-jack \
    --disable-dsound \
    --disable-winaudio \
    --disable-guest-agent \
    --disable-guest-agent-msi \
    --disable-libudev \
    --disable-zstd \
    --disable-lzo \
    --disable-snappy \
    --disable-bzip2 \
    CFLAGS="-O3 -march=native -mtune=native" \
    LDFLAGS="-O3"

echo "Building (using $(sysctl -n hw.ncpu) cores)..."
make -j$(sysctl -n hw.ncpu)

echo "Installing..."
make install

echo "Stripping debug symbols..."
strip -S "$OUTPUT_DIR/bin/qemu-system-aarch64" 2>/dev/null || true
strip -S "$OUTPUT_DIR/bin/qemu-img" 2>/dev/null || true

echo ""
echo "========================================"
echo "Build complete!"
echo "Binaries: $OUTPUT_DIR/bin/"
echo ""
ls -lh "$OUTPUT_DIR/bin/"
echo ""
echo "For development, copy to:"
echo "  src-tauri/sidecars/qemu/"
echo "========================================"
