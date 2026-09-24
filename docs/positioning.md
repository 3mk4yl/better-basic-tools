# Positioning

## One-line description

**Better Basic Tools is a local Linux observation substrate for agents and operators.**

Alternate short phrases:

> **BBT helps agents and operators understand Linux systems without fragile shell glue.**

> **Structured Linux facts for agents and operators.**

## Longer description

Better Basic Tools is a Rust-based, strictly read-only Linux observation layer
for agentic AI, scripts, and human operators. It turns common ad hoc pipe-chains
into deterministic commands with clear summaries, stable JSON schemas,
evidence-backed facts, and risk-labeled next observations — without sacrificing
UNIX composability or script safety.

"Substrate" is deliberate: BBT is the factual layer that investigation, triage,
and decision workflows are built on. It reports; it never decides, remediates,
or acts.

## Differentiation

Compared with GNU coreutils:

- less focused on perfect historical compatibility;
- more focused on diagnostics, schemas, and agent-readable context.

Compared with `uutils`:

- not a Rust clone of existing coreutils;
- a new interface layer for structured system understanding.

Compared with tools like `eza`, `bat`, `dust`, and `bottom`:

- less focused on individual interactive UX;
- more focused on a cohesive system vocabulary across domains.

Compared with observability stacks:

- local-first;
- command-line native;
- no daemon required for the early MVP;
- works as a direct terminal tool;
- optimized for immediate local investigation rather than fleet telemetry.

Compared with AI shell agents:

- not an agent;
- provides the factual substrate agents can safely consume.

## Enforced boundaries

Positioning is backed mechanically, not just rhetorically:

- read-only posture and threat model: [`security-model.md`](security-model.md);
- no network/TLS crates in the dependency graph: enforced by `deny.toml` in CI;
- structured-output contracts: every JSON command ships a schema, validated
  against live output in CI;
- exit-code policy separating "valid observation" from "CLI failure":
  [`exit-codes.md`](exit-codes.md).

## Audience

- Agent framework authors who need a trustworthy local fact layer.
- Linux power users and sysadmins.
- Developers who want better local diagnostics.
- DevOps and platform teams.
- Security/defensive tooling builders.
- Anyone who wants structured, explainable OS state without a heavy stack.

## Defensive positioning

BBT is not a defensive toolkit by itself. It does not replace scanners, SIEMs,
EDRs, forensic suites, or incident-response platforms.

It does support defenders because it makes local Linux state easier to inspect
and harder to misunderstand. The strongest early defensive story is Blue/Observer
work in a controlled cyber range: BBT captures local facts and evidence while the
range defines containment, scope, scoring, and response rules.
