# Troubleshooting Guide

Common diagnostic procedures for HCS Linux.

## Diagnosing RAM Consumption

If the system approaches the 8 GB peak target:
```bash
# Check current resident set size of models
hcs-control status

# Force model cache unload
hcsd --force-unload-models
```

## Verifying Network & Tor Status

```bash
# Inspect Tor circuit and SOCKS port
hcs-control privacy

# Verify nftables filter rules
sudo nft list ruleset
```

## Cognitive Memory Integrity

If SQLite reports corruption or indexing delays:
```bash
# Rebuild FTS5 virtual tables and consolidate records
hcs-search --rebuild-index
```
