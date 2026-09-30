# What is HCS Linux?

HCS Linux is an AI-native, privacy-oriented, CPU-first Linux distribution.

## Core Philosophy

Rather than treating artificial intelligence as a cloud web service or an overlay window, HCS Linux integrates local machine intelligence directly into the operating system architecture:

1. **Local-First & Private:** All inference, memory indexing, and event logging happen on your local hardware. No remote telemetry or cloud forwarding.
2. **CPU-First Performance:** Optimized for contemporary x86_64 processors using AVX/AVX2 vector instructions, eliminating mandatory high-power GPU requirements.
3. **Strict Memory Budgets:** Built to operate stably within `<= 6 GB RAM` idle and `<= 8 GB RAM` peak.
4. **Deterministic Verification:** Every system action and generated code snippet is validated against objective evidence (exit codes, unit tests, filesystem checks).
