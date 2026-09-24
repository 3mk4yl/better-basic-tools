# Roadmap

## Phase 0 — Thesis and skeleton

- [x] Name project: Better Basic Tools.
- [x] Define concept: ASI — Agentic System Interface for Linux.
- [x] Define purpose: structured local Linux investigation for humans and agents.
- [x] Create Rust workspace skeleton.
- [x] Document purpose, vision, output contract, and architecture.

## Phase 1 — Core contracts

Goal: establish reusable types and rendering across all commands.

- [x] Define `OutputFormat`: human, json, yaml, agent.
- [x] Define schema envelope types in `bbt-core`.
- [x] Define agent report types in `bbt-agent`.
- [x] Define risk labels, evidence, facts, next-step commands.
- [x] Add first integration tests for JSON/agent output stability.
- [x] Add initial `schemas/fs-inspect.v1.schema.json`.

## Phase 2 — `bbt fs inspect PATH`

Goal: one-shot path understanding.

- [x] Resolve path metadata.
- [x] Report file type, size, symlink target, timestamps.
- [x] On Unix, include uid/gid/mode.
- [x] Evaluate current-user readability/writability/executability.
- [x] Emit human, JSON, YAML, and agent modes.
- [ ] Explain permission-denied cases with richer owner/group/current-user evidence.

## Phase 3 — `bbt disk usage PATH`

Goal: explain disk usage without fragile command chains.

- [x] Walk directories safely and efficiently.
- [x] Support depth limits.
- [x] Support one-filesystem mode.
- [x] Support configurable largest-child count with `--top N`.
- [x] Identify largest children.
- [x] Emit human, JSON, YAML, and agent modes.
- [x] Add initial `schemas/disk-usage.v1.schema.json`.

## Phase 4 — `bbt host snapshot`

Goal: one-shot system situation report for humans and agents.

- [x] OS/kernel/hostname.
- [x] Current user summary.
- [x] CPU model/core summary.
- [x] Load average and memory summary.
- [x] Filesystem pressure summary.
- [ ] Process pressure summary.
- [ ] Optional systemd failed-service summary where available.
- [x] Agent summary with severity and next observations.
- [x] Add initial `schemas/host-snapshot.v1.schema.json`.

## Phase 5 — `bbt proc inspect PID`

Goal: process understanding for operational debugging.

- [x] Process metadata from `/proc`.
- [x] Command line, parent/children, user, state.
- [x] Memory where available.
- [x] Agent summary with risk-aware next observations.
- [x] Add initial `schemas/proc-inspect.v1.schema.json`.
- [ ] Open files and sockets as optional elevated/detail mode.

## Phase 6 — Alpha candidate hardening

Goal: stabilize the current proof into a coherent alpha candidate before adding
more surface area.

- [x] Fix `proc inspect` parent PID display so summaries do not expose Rust `Option` formatting.
- [x] Run full quality gates on a clean checkout.
- [x] Validate all shipped JSON schemas against real command output.
- [x] Review docs for purpose/positioning consistency.
- [x] Tag an alpha candidate once the current command set is coherent and clean.

## Phase 7 — Defensive-workflow validation

Goal: use BBT as a local evidence layer for defensive (Blue/Observer-style)
investigation workflows, not as an offensive automation toolkit.

- [ ] Capture host/path/disk/process evidence with existing commands in a
      controlled, authorized environment.
- [ ] Identify which missing commands block a compelling defensive workflow.
- [ ] Feed findings back into the next command roadmap.

## Phase 8 — Familiar and defensive-investigation commands

After the ASI concept is proven and the alpha candidate is stable, add daily-use
commands and aliases:

- [x] `bbt fs list`
- [x] `bbt fs tree`
- [x] `bbt disk filesystems`
- [x] `bbt proc list`
- [x] `bbt net listeners`
- [x] `bbt service inspect`
- [ ] `bbt project inspect PATH`
- [ ] optional aliases: `bls`, `bdu`, `bdf`, `bps`.

## Phase 9 — Production-readiness hardening (2026-07)

Goal: move from "strong alpha" toward a releasable, contract-stable tool.
Positioning locked: **local Linux observation substrate for agents and operators.**

- [x] `disk usage --depth` adopts `du --max-depth` semantics: depth limits
      reporting only; children always report full recursive subtree totals.
- [x] Complete schema coverage: `proc-list.v1`, `net-listeners.v1`,
      `service-inspect.v1` added; integration test enforces that every JSON
      command has a matching schema.
- [x] Hardening: canonical IPv6 decoding in `net listeners`; structured
      permission-denied warnings in `fs inspect` (exit 0, `warnings[]` field);
      systemctl-unavailable coverage in `service inspect`.
- [x] Agent triage facts: `largest_child` (disk), `root_process_count` /
      `zombie_process_count` / `top_rss_process` (proc list),
      `externally_bound_listener_count` / `privileged_listener_count` /
      `unattributed_listener_count` (net), `service_state_problem` (service).
- [x] CLI contract: `bbt version` subcommand; documented exit-code policy
      (`docs/exit-codes.md`) pinned by tests.
- [x] Supply chain: `deny.toml` (advisories, license allowlist, banned
      network/TLS crates, trusted sources); RUSTSEC-2026-0190 fixed
      (anyhow 1.0.103); `publish = false` across the workspace;
      `docs/security-model.md`.
- [x] CI/CD: three-job CI (rust, cargo-deny, live schema validation);
      tag-driven release workflow with x86_64 + aarch64 builds, checksums,
      and auto-generated release notes; `docs/release-policy.md`.
- [x] Documentation reframe around the observation-substrate positioning;
      command docs added for `proc list`, `net listeners`, `service inspect`.

## Toward v1.0

Detailed, prioritized items live in [`backlog.md`](backlog.md).

- [x] Prepare `v0.2.0-alpha.2` for the release pipeline.
- [x] Replace `serde_yaml` (archived upstream) with maintained `serde_norway` fork.
- [x] `bbt disk filesystems` — mounted filesystems, completes the disk
      domain's capacity view; relocated shared mount-reading logic into
      `bbt-core` so `host snapshot` and `disk filesystems` share one
      implementation.
- [x] `bbt disk pressure` — I/O Pressure Stall Information from
      `/proc/pressure/io`; graceful `available: false` degradation on
      kernels/configs without PSI. `disk` domain now has three verified
      verbs (`usage`, `filesystems`, `pressure`).
- [x] `bbt fs list` — non-recursive structured directory listing (the `ls`
      counterpart); per-entry kind, size, owner, mode, and mtime reusing the
      `fs inspect` field shapes, stable name-byte sort, lstat semantics
      (symlinks never followed), structured-warning degradation for missing,
      non-directory, and unreadable paths.
- [x] `bbt fs tree` — bounded-depth directory tree with per-subtree recursive
      totals (allocated and apparent bytes, hard links counted once); adopts
      the `disk usage --depth` reporting semantics (depth limits reported
      children only, every reported node carries full recursive totals),
      lstat semantics (symlinks to directories are leaves), `--one-filesystem`
      device pinning, and structured-warning degradation for missing,
      non-directory, and unreadable paths.
- [ ] Migrate `getpwuid` to `getpwuid_r` if concurrency is ever introduced.
- [ ] Host snapshot: process-pressure and failed-service summaries.
- [ ] `proc inspect`: open files/sockets as an optional detail mode.
- [ ] Schema freeze at v1.0.0 (`bbt.*.v1` become append-only) per
      `docs/release-policy.md`.
