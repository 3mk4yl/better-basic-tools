# Decisions

These questions were open during project framing and are now accepted as initial
project direction. They can be revisited before a stable public API, but they are
no longer blockers for the first implementation.

## Binary and command naming

**Decision:** Ship the unified binary as `bbt`.

Rationale:

- `bbt` clearly abbreviates Better Basic Tools;
- it is less collision-prone than `bb`;
- `asi` is the concept, not the day-to-day command name;
- `bb` can remain a possible optional alias later.

## Project/concept naming

**Decision:** Project name is **Better Basic Tools**.

**Decision:** Concept name is **ASI — Agentic System Interface for Linux**.

**Decision:** Primary public phrase is:

> BBT helps humans and agents understand Linux systems without fragile shell glue.

**Decision:** Supporting phrase is:

> Structured Linux facts for humans and agents.

**Decision:** Historical/secondary phrase remains useful for context:

> GNU-like Linux tools for the agentic era.

## Agent mode naming

**Decision:** Use `--agent` for reasoning-oriented output.

Rationale:

- it clearly names the intended consumer;
- it differentiates agent reports from raw `--json` and `--yaml`;
- it preserves the three-layer model: human, data, agent.

## Suggestion/risk philosophy

**Decision:** Suggestions must be conservative, evidence-backed, and risk-labeled.

Rules:

- always prefer safe next observations over mutating actions;
- risky actions may be shown only when common, well-understood, and clearly
  labeled;
- suggestions must include purpose and risk;
- the tool must never execute suggested actions automatically;
- the tool reports facts and risk context; humans and agents make decisions.

## GNU compatibility stance

**Decision:** Familiarity matters, but perfect GNU compatibility is not the goal.

Rules:

- support common flags where they fit naturally;
- document intentional differences;
- do not claim byte-for-byte GNU compatibility;
- do not replace system binaries by default;
- compatibility is a feature, but not a prison.

## Embedded AI/LLM stance

**Decision:** Do not embed an LLM in the core tools.

Better Basic Tools should provide deterministic facts, diagnostic structure,
evidence, and risk-aware next steps. LLM agents can consume that output from
outside the tool.

The project should remain local-first, deterministic, auditable, and free of
hidden network calls or telemetry.

## Public launch angle

**Decision:** Position Better Basic Tools as an Agentic System Interface rather
than another collection of colorful replacements.

Best framing:

> Better Basic Tools is not another `ls` replacement. It is an Agentic System
> Interface for Linux: GNU-like tools for humans, scripts, and AI agents.

## Defensive use-case stance

**Decision:** BBT is not a defensive toolkit, SIEM, EDR, scanner, pentest
framework, or exploit toolkit.

BBT is a local Linux investigation layer that can support defenders, developers,
operators, scripts, and agents by producing repeatable local facts. Defensive
work is a demanding validation domain because it stresses read-only behavior,
evidence, safe next observations, and structured outputs.

**Decision:** Defensive validation should focus on Blue/Observer-style
workflows. Red usage should be limited to authorized in-scope local orientation
inside a controlled environment and must not define BBT's product identity.
