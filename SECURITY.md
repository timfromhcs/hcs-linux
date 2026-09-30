# Security Policy

The HCS Linux Project takes security, privacy, and user sovereignty seriously.

## Supported Versions

Only the current stable line receives active security updates and advisories.

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |
| < 0.1.0 | No        |

## Reporting a Vulnerability

If you discover a security vulnerability in HCS Linux or any of its core subcomponents (`hcsd`, `hcs-modeld`, `hcs-shell`, `hcs-security`, etc.), please do **NOT** open a public issue.

Instead, please report it via one of the following methods:
- Email: `timfromhcs@gmail.com` with subject `[SECURITY VULNERABILITY] HCS Linux`
- GitHub Private Vulnerability Reporting on the repository `timfromhcs/hcs-linux`

Please include:
1. Description of the vulnerability and attack vector.
2. Steps to reproduce or proof-of-concept code.
3. System configuration tested (OS version, hardware profile, model loaded).
4. Potential impact assessment.

We strive to acknowledge reports within 48 hours and provide a remediation timeline within 7 days.

## Agent Privilege & Security Model

HCS Linux implements strict least-privilege boundaries:
- The AI Brain and Prime Agent run under non-root user sessions with scoped file and network access.
- Actions classified as `PRIVILEGED` or `IRREVERSIBLE` require explicit user interactive authorization via Polkit / system modal dialogs.
- Root execution is never granted implicitly or automatically by any AI model.
- Model weights and GGUF files are verified against SHA-256 lock manifests before execution.
