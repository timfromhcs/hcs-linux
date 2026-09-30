# Contributing to HCS Linux

Thank you for your interest in contributing to HCS Linux!

## Code of Conduct

We are committed to providing a friendly, safe, and welcoming environment for all contributors. Please be respectful and constructive in all discussions.

## Branch Strategy & Pull Requests

We use a structured Git workflow:
- `main`: Protected stable branch. Tagged releases only. No direct pushes.
- `dev`: Active integration branch.
- `feature/*`: New features and modules.
- `fix/*`: Bug fixes.
- `security/*`: Security patches and sandboxing improvements.
- `perf/*`: Performance and RAM budget optimizations.
- `release/*`: Release candidate stabilization.

All changes must pass CI checks, unit/integration tests, license validation, and RAM budget checks before merging into `dev` or `main`.

## Conventional Commits

Commit messages must follow the format `<type>: <description>`:
- `feat:` A new feature or capability
- `fix:` A bug fix
- `security:` Security boundary, sandboxing, or permission fix
- `perf:` RAM optimization, speed improvement, context reduction
- `build:` Build scripts, packaging, live-build, Makefiles
- `test:` Unit tests, integration tests, VM tests, QA scripts
- `docs:` Documentation, help system, architecture notes
- `refactor:` Code restructuring without functional change
- `chore:` Dependency bump, formatting, maintenance
- `release:` Release tagging and artifact generation

## Testing & Quality Requirements

Before submitting a PR:
1. Ensure all code passes `cargo test`, `cargo fmt --check`, and `cargo clippy`.
2. Python scripts must pass validation without runtime syntax errors.
3. Every new agent tool or model endpoint must include automated test coverage.
4. Memory and RAM impacts must be measured against the <= 6GB idle / <= 8GB peak budget.
5. All external sources, dependencies, and model files must be pinned in lockfiles.
