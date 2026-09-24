# Feature Backlog

Prioritized backlog for work beyond v0.2.0-alpha.2. The strategic phase view
lives in [`roadmap.md`](roadmap.md); this file is the working list. Items move
from here into implementation, gated by the standard quality gates
([`release-policy.md`](release-policy.md)).

Positioning constraint for every item: BBT stays a **local, read-only Linux
observation substrate**. No remediation, no network, no daemons.

## P1 — Deepen existing commands

The current surface is broad enough; the next value is depth.

### `host snapshot`: process-pressure summary

- Total/running/zombie process counts, top RSS consumers.
- Reuses `bbt-proc` collection; new `process_pressure` block in
  `host-snapshot.v1` (additive, optional field).
- Agent facts: `zombie_process_count`, `top_rss_process` at host level.

### `host snapshot`: failed-service summary

- Where systemd is available: failed-unit count and names via read-only
  `systemctl --failed --output=json` (or `show` per unit).
- Structured warning (not failure) on non-systemd hosts, matching the
  `service inspect` pattern.
- Agent fact: `failed_service_count` with `warning` severity when nonzero.

### `proc inspect`: open files and sockets detail mode

- Optional flag (e.g. `--detail fds`): open file descriptors, socket inodes
  correlated with `/proc/net`, working directory and executable presence
  checks (deleted-binary detection).
- Degrades to structured warnings when `/proc/<pid>/fd` is unreadable.
- Likely requires a v2 addition to `proc-inspect` schema or an additive
  optional block; prefer additive.

### `fs inspect`: richer permission diagnostics

- uid/gid name resolution (carry-over from roadmap Phase 2 leftover).
- Owner/group/current-user evidence chain for permission-denied cases.
- Broken-symlink diagnostics (target missing vs. loop vs. too many levels).
- Parent-directory permission chain inspection.

## P2 — Widen the command surface

The `disk` domain is now complete (`usage`, `filesystems`, `pressure`) —
three verified verbs backed by three distinct data sources (recursive
traversal, `/proc/mounts` + `statvfs`, and kernel PSI). `bbt fs list PATH`
and `bbt fs tree PATH` have since shipped as well — the non-recursive `ls`
counterpart with kind, size, owner, mode, and mtime per entry and a stable
name-byte sort, and the bounded-depth tree with per-subtree recursive totals
reusing the du-style `--depth` semantics from `disk usage`.
Remaining items in rough priority order:

1. **`bbt project inspect PATH`** — one-shot project context: VCS presence
   and status summary, language/build-system markers, largest artifacts.
   (Design doc first; scope creep risk is high.)

Each new command ships with: schema + validation case in
`scripts/validate_schemas.py` (enforced — the script fails on uncovered
schemas), command doc under `docs/commands/`, agent triage facts, and
structured-warning degradation.

## P3 — Ergonomics and distribution

- Optional compatibility aliases: `bls`, `bdu`, `bdf`, `bps` (thin argv
  remaps, human mode only; JSON/agent consumers should use canonical
  commands).
- Shell completions (`clap_complete`) shipped in release archives.
- Man pages generated from clap definitions.
- musl static builds as additional release targets (zero-dependency deploys
  onto minimal/container hosts).

## P4 — Internal quality

- `getpwuid` → `getpwuid_r` migration if any concurrency is introduced
  (documented constraint in [`security-model.md`](security-model.md)).
- Property/fuzz tests for `/proc` parsers (net hex addresses, status fields,
  cmdline splitting) — these parse kernel-formatted, not attacker-controlled,
  data, but robustness is cheap insurance.
- Golden-output snapshot tests for human mode to catch accidental wording
  drift.
- Container-based CI job exercising a non-systemd environment end to end.

## v1.0 gate

Schema freeze per [`release-policy.md`](release-policy.md): all `bbt.*.v1`
schemas become append-only. Before freezing:

- [ ] P1 items landed (they may still add optional schema fields).
- [ ] Cross-command consistency review: field naming, warning phrasing,
      severity conventions, evidence style.
- [ ] One external consumer exercise: drive a real agent workflow against the
      JSON/agent contracts and fix what chafes.

## Non-goals (permanent, from positioning)

Mutating/remediation commands, network features, daemon mode, fleet
telemetry, SIEM/EDR/scanner ambitions, and AI chat features remain out of
scope. Post-v1.0 discussions about a separate *action* layer must happen in a
separate project, not BBT.
