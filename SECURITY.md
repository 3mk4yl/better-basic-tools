# Security Policy

## Supported versions

BBT is currently alpha software. Security reports are welcome for the current `main` branch and the latest published prerelease.

## Scope

BBT is intended to be a local, read-only Linux investigation toolkit. Current commands inspect host, filesystem, disk-usage, process, network-listener, and service metadata. They should not send telemetry, contact external services, modify files, start or stop services, or perform remediation.

The full security posture, guarantees, unsafe-code review, and threat model are documented in [`docs/security-model.md`](docs/security-model.md). Dependency and supply-chain policy is enforced via [`deny.toml`](deny.toml) (`cargo deny check`).

Security-sensitive issues include, but are not limited to:

- unintended file modification or deletion;
- unexpected network access or telemetry;
- leaking secrets in human, JSON/YAML, or agent output;
- command injection or unsafe shell execution;
- path traversal or symlink handling bugs that violate documented behavior;
- misleading risk labels that recommend destructive actions as safe.

## Reporting a vulnerability

Please do **not** open a public issue for a suspected vulnerability.

Preferred options:

1. Use GitHub's private vulnerability reporting / security advisory flow for this repository if available.
2. If private reporting is not available, open a minimal public issue stating that you have a security report and need a private contact path. Do not include exploit details or sensitive output in that issue.

Please include:

- affected version or commit;
- operating system and kernel;
- exact command line;
- expected behavior;
- observed behavior;
- sanitized logs or output;
- whether the issue is reproducible in a temporary test directory/container.

## Disclosure

This is an early project, so response times are best-effort. The goal is to acknowledge valid reports, reproduce them, fix them, and publish a note in the changelog or release notes when appropriate.
