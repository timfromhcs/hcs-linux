# Developer Mode

Developer Mode equips HCS Linux with standard system compilers, SDKs, and build chains while keeping the desktop environment clean and lightweight.

## Available Toolchains

- **Rust:** `rustc`, `cargo`, `clippy`, `rustfmt`
- **C/C++:** `gcc`, `g++`, `make`, `cmake`, `ninja`
- **Python:** `python3`, `pip`, `venv`, `pytest`
- **Version Control:** `git`, `gh`
- **System Debugging:** `strace`, `gdb`, `perf`, `htop`, `ripgrep`, `jq`

Activate Developer Mode via:
```bash
hcs-control developer enable
```
