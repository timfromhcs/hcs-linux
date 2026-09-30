# Private Mode & Tor Integration

HCS Linux incorporates a built-in privacy subsystem that isolates network traffic and prevents identity leakage.

## Architecture

- **Local Tor Daemon:** Operates on `127.0.0.1:9050` (SOCKS5) and `127.0.0.1:9040` (Transparent Proxy mode).
- **NFTables Isolation:** When Private Mode is active, outgoing non-Tor TCP traffic is dropped by kernel packet filters.
- **DNS Leak Protection:** System resolved routes DNS queries exclusively through Tor DNS listeners.
- **No Cloud Telemetry:** Core daemons (`hcsd`, `hcs-modeld`, `hcs-shell`) have zero outbound telemetry phone-home endpoints.
