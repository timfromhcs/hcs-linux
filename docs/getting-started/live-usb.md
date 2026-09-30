# HCS Linux Live USB & Amnesic Sessions

HCS Linux can run directly from any USB flash drive without touching your internal hard drives.

## Creating a Live USB

### On Linux
```bash
sudo dd if=HCS-Linux-0.1.0-alpha.1-amd64.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

### On Windows
Use Rufus or BalenaEtcher to write `HCS-Linux-0.1.0-alpha.1-amd64.iso` in DD/hybrid image mode.

## Live Modes

- **Standard Live:** Full hardware access with RAM overlay. Changes are discarded on reboot unless an encrypted persistence partition is attached.
- **Amnesic Private Mode:** Network traffic is routed exclusively through Tor, memory scrubbers run on shutdown, and zero swap/disk write is permitted.
