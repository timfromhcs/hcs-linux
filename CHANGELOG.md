# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha.1] - 2026-09-30

### Added
- Canonical repository architecture and bootstrap configuration.
- Debian 13 (Trixie) live-build configuration and automated hybrid ISO generation pipeline.
- Core daemon `hcsd` implementing Unix domain socket API and HCS Brain orchestration.
- Model daemon `hcs-modeld` with on-demand GGUF loading, explicit unloading, and RSS tracking.
- Memory management engine `hcs-memory` backed by SQLite FTS5 lexical search and hybrid retrieval.
- Subagent execution framework `hcs-agents` with least-privilege capability permissions.
- Modern Wayland shell `hcs-shell` design specifications for Niri and Quickshell.
- Deterministic evaluation, self-scoring judge protocol, and experience ledger.
- Hardware profiling and Edge model catalog with SHA-256 verification manifests.
- Calamares installer branding and offline documentation suite.
