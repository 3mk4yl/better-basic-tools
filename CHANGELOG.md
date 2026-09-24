# Changelog

## v0.2.0-alpha.2 - 2026-08-08

### Changed

- `bbt disk usage --depth` now accepts only `0` (totals only) and `1`
  (immediate children). Larger values previously parsed but silently reported
  the same immediate-child surface as `--depth 1`; they are now rejected at
  CLI parsing with a pointer to `bbt fs tree PATH --depth N`.
- Documentation refresh for the public alpha: Linux-only support is stated
  explicitly, and installation guidance now directs readers to GitHub Releases.

### Fixed

- `service inspect` rejects unit names beginning with `-` before invoking
  `systemctl`, so an option-like name can no longer be passed as a
  `systemctl` option.
- `proc list` and `proc inspect` decode procfs text lossily instead of
  requiring valid UTF-8, so a process whose kernel-reported name contains
  non-UTF-8 bytes is no longer dropped from listings or turned into a
  decoding error.
- `disk usage` and `fs tree` now track visited directories by
  `(st_dev, st_ino)` and skip a same-device revisit (for example a
  bind-mount cycle) with one bounded `directory-cycle` error/warning instead
  of re-traversing or double counting the subtree.

## v0.2.0-alpha.1 - 2026-07-11

Production-readiness hardening pass. Positioning locked: **BBT is a local Linux
observation substrate for agents and operators.**

### Changed

- **`bbt disk usage --depth` now follows `du --max-depth` semantics.** Depth
  limits *reporting* depth only; totals and per-child sizes are always full
  recursive subtree sums. Previously `--depth N` silently truncated traversal
  and underreported usage (e.g. a populated `target/` directory was invisible
  at `--depth 1`). `--depth 0` reports recursive root totals without child rows.
- `fs inspect` on permission-denied paths now returns structured output with
  exit code `0`, `exists: false`, `kind: unknown`, and a `warnings[]` entry
  instead of failing with a raw OS error. The `warnings` field was added to
  `FsInspection` and the `fs-inspect.v1` schema; agent severity reflects
  incomplete inspections.
- `net listeners` decodes IPv6 `/proc/net` addresses to canonical form
  (`::1`, `::`) instead of raw kernel hex.
- Existing-but-inactive services now produce `warning` severity in
  `service inspect --agent` (previously `info`).

### Added

- `bbt version` subcommand.
- JSON schemas for the full command surface: `proc-list.v1`,
  `net-listeners.v1`, `service-inspect.v1`; an integration test enforces that
  every JSON command has a matching schema file.
- Agent triage facts:
  - `disk usage`: `largest_child` fact plus a follow-up next step into the
    largest child;
  - `proc list`: `root_process_count`, `zombie_process_count` (warning when
    nonzero), `top_rss_process`;
  - `net listeners`: `externally_bound_listener_count` and
    `privileged_listener_count` (warning when nonzero),
    `unattributed_listener_count`;
  - `service inspect`: `service_state_problem`.
- `docs/exit-codes.md` — exit-code policy (structured success vs. usage error),
  pinned by the new `cli_contract` test suite.
- `docs/security-model.md` — guarantees, unsafe-code inventory, threat model.
- `docs/release-policy.md` — versioning, quality gates, v1.0 schema-freeze plan.
- `deny.toml` supply-chain policy: RustSec advisories, license allowlist,
  banned network/TLS crates (mechanically enforcing the local-only posture),
  crates.io-only sources. Wired into CI via `cargo-deny-action`.
- CI: dedicated schema-validation job running `scripts/validate_schemas.py`
  against live command output.
- Release automation: tag-driven workflow building x86_64 and aarch64
  linux-gnu archives (binary + docs + schemas) with SHA-256 checksums and
  auto-generated GitHub release notes.
- Command docs for `proc list`, `net listeners`, and `service inspect`.

### Fixed

- RUSTSEC-2026-0190: `anyhow` unsoundness — updated 1.0.102 → 1.0.103.

### Internal

- All workspace crates marked `publish = false` (distribution is via GitHub
  release binaries).
- systemctl invocation gained a test seam; systemctl-unavailable hosts are
  covered by unit tests.
- YAML output migrated from archived `serde_yaml` to the maintained
  `serde_norway` fork (identical output, no API change).

## v0.1.0-alpha.1 - 2026-05-30

First alpha candidate for Better Basic Tools: the local situation-awareness core.

### Purpose

BBT helps humans and agents understand Linux systems without fragile shell glue. It is a local Linux investigation toolkit that produces human-readable output, stable JSON/YAML, and agent reports with evidence-backed facts and safe next observations.

### Included commands

- `bbt host snapshot`
  - Hostname, OS/kernel, current user, CPU, load, memory, filesystem pressure, JSON/YAML/agent output.
- `bbt fs inspect PATH`
  - Path metadata, file kind, ownership/mode where available, access checks, JSON/YAML/agent output.
- `bbt disk usage PATH`
  - Recursive disk usage with depth, one-filesystem, largest-child summaries, JSON/YAML/agent output.
- `bbt proc inspect PID`
  - Process metadata from `/proc`, command line, parent/children, user, state, memory, JSON/YAML/agent output.

### Output contracts

- Human mode for direct terminal use.
- `--json` and `--yaml` for complete structured data.
- `--agent` for compact facts, evidence, interpretation, severity, and safe next observations.
- Initial schemas are included under `schemas/`.

### Polish and fixes

- Cleaned `proc inspect` agent summary so parent PID displays as a user-facing value instead of Rust `Option` debug formatting.

### Boundaries

This alpha is not a complete Linux investigation suite yet. BBT is not a SIEM, EDR, scanner, pentest framework, exploit toolkit, or AI chatbot. It reports local facts; humans, scripts, and agents make decisions.

### Expected next work

- Likely next commands: `proc list`, `net listeners`, `service inspect`, and `project inspect PATH`.
