#!/usr/bin/env bash
# ==============================================================================
# Project ShadowOS — Stripped Minimal Linux Kernel Build Toolchain
# Target: vmlinux (uncompressed, stripped, < 4.5MB, boot time < 20ms)
# ==============================================================================
set -euo pipefail

KERNEL_VERSION="6.6.45"
KERNEL_TAR="linux-${KERNEL_VERSION}.tar.xz"
KERNEL_URL="https://cdn.kernel.org/pub/linux/kernel/v6.x/${KERNEL_TAR}"
BUILD_DIR="$(pwd)/build/kernel"
OUTPUT_DIR="$(pwd)/binaries"

mkdir -p "${BUILD_DIR}" "${OUTPUT_DIR}"

echo "[*] Downloading Linux Kernel ${KERNEL_VERSION}..."
if [ ! -f "${BUILD_DIR}/${KERNEL_TAR}" ]; then
    curl -fsSL "${KERNEL_URL}" -o "${BUILD_DIR}/${KERNEL_TAR}"
fi

echo "[*] Extracting kernel sources..."
tar -xf "${BUILD_DIR}/${KERNEL_TAR}" -C "${BUILD_DIR}"
cd "${BUILD_DIR}/linux-${KERNEL_VERSION}"

echo "[*] Applying minimal defconfig..."
make defconfig

echo "[*] Stripping bloat subsystems and enabling VirtIO / MicroVM requirements..."
./scripts/config --disable CONFIG_PCI
./scripts/config --disable CONFIG_ACPI
./scripts/config --disable CONFIG_SOUND
./scripts/config --disable CONFIG_USB
./scripts/config --disable CONFIG_WIRELESS
./scripts/config --disable CONFIG_DRM
./scripts/config --disable CONFIG_MODULES

# VirtIO & Storage Drivers
./scripts/config --enable CONFIG_VIRTIO
./scripts/config --enable CONFIG_VIRTIO_MMIO
./scripts/config --enable CONFIG_VIRTIO_PCI
./scripts/config --enable CONFIG_VIRTIO_FS
./scripts/config --enable CONFIG_VIRTIO_BALLOON
./scripts/config --enable CONFIG_OVERLAY_FS
./scripts/config --enable CONFIG_TMPFS
./scripts/config --enable CONFIG_TMPFS_POSIX_ACL
./scripts/config --enable CONFIG_FUSE_FS

# Zero-TCP AF_VSOCK Drivers
./scripts/config --enable CONFIG_VSOCKETS
./scripts/config --enable CONFIG_VIRTIO_VSOCKETS
./scripts/config --enable CONFIG_VHOST_VSOCK

echo "[*] Compiling uncompressed vmlinux..."
make -j"$(nproc)" vmlinux

cp vmlinux "${OUTPUT_DIR}/vmlinux"
echo "[✓] Successfully generated uncompressed kernel at: ${OUTPUT_DIR}/vmlinux"
ls -lh "${OUTPUT_DIR}/vmlinux"
