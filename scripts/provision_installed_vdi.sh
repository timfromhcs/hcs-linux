#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_DIR="${REPO_ROOT}/target"
RAW_IMG="${TARGET_DIR}/hcs_installed.img"
MOUNT_DIR="/tmp/hcs_vdi_mount"

echo "=== HCS Linux - Provision Installed VDI Hard Disk ==="
mkdir -p "${TARGET_DIR}"
rm -f "${RAW_IMG}"

# 1. Create a 3 GB raw disk image
echo "[1/6] Allocating 3 GB raw disk image..."
truncate -s 3G "${RAW_IMG}"

# 2. Partition with MBR (DOS disklabel, partition 1 bootable)
echo "[2/6] Partitioning disk with MBR bootable partition..."
echo -e ',,83,*' | /usr/sbin/sfdisk "${RAW_IMG}" >/dev/null 2>&1

# 3. Setup loop device with partition scan
echo "[3/6] Attaching loop device..."
LOOP_DEV=$(sudo losetup -P -f --show "${RAW_IMG}")
echo "  Attached to: ${LOOP_DEV}"
LOOP_PART="${LOOP_DEV}p1"

# Cleanup trap to ensure loop detach and unmount
cleanup() {
    echo "  Cleaning up loop device and mount points..."
    sudo umount -f "${MOUNT_DIR}" 2>/dev/null || true
    sudo losetup -d "${LOOP_DEV}" 2>/dev/null || true
    rm -rf "${MOUNT_DIR}" 2>/dev/null || true
}
trap cleanup EXIT

# 4. Format ext4 with label HCS_ROOT
echo "[4/6] Formatting ext4 filesystem (LABEL=HCS_ROOT)..."
sudo mkfs.ext4 -F -L "HCS_ROOT" "${LOOP_PART}" >/dev/null 2>&1

# 5. Mount and install payload + kernel + GRUB
echo "[5/6] Populating rootfs and configuring bootloader..."
mkdir -p "${MOUNT_DIR}"
sudo mount "${LOOP_PART}" "${MOUNT_DIR}"

# Copy rootfs
if [ -d "${TARGET_DIR}/rootfs" ]; then
    sudo cp -a "${TARGET_DIR}/rootfs/"* "${MOUNT_DIR}/"
fi

# Ensure essential directories exist
sudo mkdir -p "${MOUNT_DIR}/boot/grub" \
              "${MOUNT_DIR}/etc" \
              "${MOUNT_DIR}/proc" \
              "${MOUNT_DIR}/sys" \
              "${MOUNT_DIR}/dev" \
              "${MOUNT_DIR}/run" \
              "${MOUNT_DIR}/tmp" \
              "${MOUNT_DIR}/home/hcs" \
              "${MOUNT_DIR}/root"

# Copy kernel & initrd
echo "  Installing Linux kernel and initrd into /boot..."
sudo cp -L /boot/vmlinuz "${MOUNT_DIR}/boot/vmlinuz"
sudo cp -L /boot/initrd.img "${MOUNT_DIR}/boot/initrd.img"
sudo chmod 644 "${MOUNT_DIR}/boot/vmlinuz" "${MOUNT_DIR}/boot/initrd.img"

# Write /etc/fstab
cat << 'EOF' | sudo tee "${MOUNT_DIR}/etc/fstab" >/dev/null
LABEL=HCS_ROOT / ext4 defaults,noatime 0 1
tmpfs /tmp tmpfs defaults,nosuid,nodev 0 0
EOF

# Write GRUB configuration with high-res graphical console
cat << 'EOF' | sudo tee "${MOUNT_DIR}/boot/grub/grub.cfg" >/dev/null
set timeout=8
set default=0

insmod all_video
insmod font
insmod gfxterm
insmod vbe
insmod vga
insmod ext2

set gfxmode=1024x768,auto
set gfxpayload=keep
terminal_output gfxterm

set menu_color_normal=light-gray/black
set menu_color_highlight=cyan/black

menuentry "HCS Linux 0.1.0-alpha.1 (Installed System)" {
    search --no-floppy --label HCS_ROOT --set=root
    linux /boot/vmlinuz root=/dev/sda1 rootfstype=ext4 rw noresume console=tty1 console=tty0 video=1024x768 init=/sbin/init
    initrd /boot/initrd.img
}

menuentry "HCS Linux Safe Mode" {
    search --no-floppy --label HCS_ROOT --set=root
    linux /boot/vmlinuz root=/dev/sda1 rootfstype=ext4 rw single noresume console=tty1 console=tty0 video=1024x768 init=/sbin/init
    initrd /boot/initrd.img
}
EOF

# Install GRUB MBR into loop device
echo "  Installing GRUB2 to MBR of ${LOOP_DEV}..."
sudo grub-install --target=i386-pc --boot-directory="${MOUNT_DIR}/boot" --modules="part_msdos ext2" "${LOOP_DEV}" >/dev/null 2>&1

echo "[6/6] Raw installed disk image prepared successfully at: ${RAW_IMG}"
echo "Ready for VBoxManage convertfromraw."
