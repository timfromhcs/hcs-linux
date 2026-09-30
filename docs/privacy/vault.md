# HCS Vault — Encrypted Storage

The HCS Vault provides hardware-backed or LUKS2-encrypted storage for sensitive documents, project keys, and personal cognitive memory databases.

## Features

- **Cipher:** `aes-xts-plain64` with 512-bit keys.
- **On-Demand Mounting:** Unlocked exclusively via user password or FIDO2 security keys.
- **Auto-Lock:** Automatically locks after configurable inactivity timeouts.
- **Amnesic Cleanup:** Keys in RAM are zeroized upon unmounting or system shutdown.
