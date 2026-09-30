# Subagent Framework & Permission Matrix

The `hcs-agents` subsystem coordinates scoped autonomous subagents operating under strict least-privilege principles.

## Capability Permission Matrix

| Capability | Read Roles | Execution Roles | Confirmation Required? |
|------------|------------|-----------------|------------------------|
| `filesystem.read` | All | All | No |
| `filesystem.write` | None | Coder, Debugger, ReleaseManager | Workspace-scoped |
| `filesystem.delete` | None | Filesystem Manager | **Yes** (Privileged) |
| `process.spawn` | None | Coder, QA, Verifier, System | Safe non-root commands |
| `network.fetch` | None | Researcher, SecurityLab | No |
| `package.install`| None | System Manager | **Yes** (Interactive polkit) |
| `root.execute` | **Forbidden** | **Forbidden** | **Never allowed** |

## Experience & Event Ledger

Every task logs a machine-readable entry containing:
- Unique Task ID & timestamp
- Redacted User Intent
- Model name and pinned revision
- Objective execution outcome (exit code, unit test pass count, changed file paths)
- Peak RSS memory footprint
- Layered Judge scoring breakdown (0-100 across 9 dimensions)
