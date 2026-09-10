#!/usr/bin/env bash
# ==============================================================================
# Project ShadowOS — Alpine Minimal Rootfs Build Toolchain
# Target: rootfs.ext4 (< 25MB, Musl Libc, BusyBox, shadow-guest-agent)
# ==============================================================================
set -euo pipefail

ALPINE_VERSION="3.20.0"
ARCH="$(uname -m)"
ALPINE_TAR="alpine-minirootfs-${ALPINE_VERSION}-${ARCH}.tar.gz"
ALPINE_URL="https://dl-cdn.alpinelinux.org/alpine/v3.20/releases/${ARCH}/${ALPINE_TAR}"
BUILD_DIR="$(pwd)/build/rootfs"
OUTPUT_DIR="$(pwd)/binaries"
ROOTFS_MNT="${BUILD_DIR}/mnt"

mkdir -p "${BUILD_DIR}" "${OUTPUT_DIR}" "${ROOTFS_MNT}"

echo "[*] Downloading Alpine Linux minirootfs..."
if [ ! -f "${BUILD_DIR}/${ALPINE_TAR}" ]; then
    curl -fsSL "${ALPINE_URL}" -o "${BUILD_DIR}/${ALPINE_TAR}"
fi

echo "[*] Creating blank 64MB ext4 image..."
dd if=/dev/zero of="${OUTPUT_DIR}/rootfs.ext4" bs=1M count=64
mkfs.ext4 -F "${OUTPUT_DIR}/rootfs.ext4"

echo "[*] Mounting rootfs loop device..."
sudo mount -o loop "${OUTPUT_DIR}/rootfs.ext4" "${ROOTFS_MNT}"

echo "[*] Unpacking Alpine base..."
sudo tar -xzf "${BUILD_DIR}/${ALPINE_TAR}" -C "${ROOTFS_MNT}"

echo "[*] Installing static shadow-guest-agent binary..."
# Note: Binary compiled with `cargo build --target x86_64-unknown-linux-musl --release`
if [ -f "$(pwd)/target/x86_64-unknown-linux-musl/release/shadow-guest-agent" ]; then
    sudo cp "$(pwd)/target/x86_64-unknown-linux-musl/release/shadow-guest-agent" "${ROOTFS_MNT}/sbin/shadow-guest-agent"
fi

echo "[*] Installing custom /init script..."
sudo tee "${ROOTFS_MNT}/init" > /dev/null << 'EOF'
#!/bin/sh
mount -t proc none /proc
mount -t sysfs none /sys
mount -t devtmpfs none /dev
mkdir -p /dev/pts /dev/shm /workspace /tmp
mount -t devpts none /dev/pts
mount -t tmpfs none /dev/shm
mount -t tmpfs none /tmp

# Mount virtio-fs host workspace
mount -t virtiofs shadow-workspace /workspace 2>/dev/null || true

# Hand off to guest execution daemon
if [ -x /sbin/shadow-guest-agent ]; then
    exec /sbin/shadow-guest-agent
else
    exec /bin/sh
fi
EOF

sudo chmod +x "${ROOTFS_MNT}/init"
sudo umount "${ROOTFS_MNT}"

echo "[✓] Successfully built guest rootfs at: ${OUTPUT_DIR}/rootfs.ext4"
ls -lh "${OUTPUT_DIR}/rootfs.ext4"
