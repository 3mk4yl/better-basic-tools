# Purpose

Better Basic Tools exists because Linux investigation work is still full of fragile shell glue.

Humans and agents repeatedly answer the same local questions by chaining tools like `ps`, `grep`, `awk`, `find`, `du`, `stat`, `df`, `mount`, `systemctl`, `journalctl`, `ss`, and `/proc` reads. Those commands are powerful, but the resulting pipelines are often hard to read, hard to verify, easy to break, and expensive for AI agents to reason about.

BBT turns those common local investigation tasks into safe, repeatable commands with human-readable output, stable structured data, and agent-oriented reports.

## One sentence

> **BBT helps humans and agents understand Linux systems without fragile shell glue.**

## Slightly longer

> **Better Basic Tools is a local Linux investigation toolkit for humans and AI agents. It replaces common ad hoc pipe-chains with deterministic commands that return clear summaries, stable JSON/YAML, evidence-backed facts, and safe next observations about hosts, filesystems, disks, processes, services, and runtime state.**

## Why BBT exists

Classic Linux tools are excellent. BBT does not exist because `ps`, `du`, `stat`, `df`, or `find` are bad.

BBT exists because modern Linux work has more consumers than the original tools were designed for:

- humans who want clear operational answers quickly;
- scripts that need stable contracts instead of parsed terminal text;
- AI agents that need compact facts, evidence, and risk-aware next observations;
- defensive engineers who need repeatable local evidence while investigating systems.

A human can understand a long pipeline if they wrote it. An agent may generate one, but then has to parse noisy text, recover from edge cases, and explain what happened. BBT should make the common path boring and reliable.

## Core job

BBT's core job is **local situation awareness**.

It should answer questions like:

- What machine am I on?
- What user and privilege context am I running under?
- What is this path?
- Is this file readable, writable, executable, or a symlink?
- Where is disk usage going?
- What is this process?
- What command started it?
- What user owns it?
- What is listening on this host?
- What state is this service in?
- What logs or evidence should I inspect next? *(future)*

BBT should provide those answers directly, without forcing every human or agent to rediscover the right pile of Linux commands.

## Not just preflight

BBT is useful before another tool runs, but that is not the whole product.

A good local workflow has three phases:

1. **Before** — establish context and verify inputs.
2. **During** — inspect runtime state, processes, services, listeners, and resources.
3. **After** — verify artifacts, outputs, logs, and evidence.

Running a specialized tool illustrates this pattern:

- BBT captures host context.
- BBT verifies the project and its input paths.
- The specialized tool performs its domain-specific job.
- BBT verifies the generated artifacts.
- BBT inspects any helper processes the run produced.

That does not make BBT a replacement for the specialized tool. It makes BBT the local evidence layer around specialized tools.

## Human and agent contract

Every command should support three views of the same local truth:

1. **Human mode** — concise terminal output for direct use.
2. **Data mode** — complete JSON/YAML with versioned schemas.
3. **Agent mode** — compact facts, evidence, interpretation, severity, and safe next observations.

Agent mode is not magic and should not require a model inside BBT. It is deterministic structure shaped for reasoning.

## Defensive use case

BBT is not a defensive platform, SIEM, EDR, scanner, or incident-response suite.

But BBT should be especially useful to defenders because defensive work depends on trustworthy local facts:

- What changed?
- What is running?
- What owns this process?
- What is listening?
- What files are suspicious or unexpectedly large?
- What evidence exists locally?
- What can I safely inspect next?

The defensive angle is a demanding validation domain because it stresses the qualities BBT cares about:

- read-only by default;
- evidence-first;
- low surprise;
- structured outputs;
- safe next observations;
- repeatable local facts.

BBT should support defenders the same way it supports developers, operators, and agents: by making Linux state easier to inspect and harder to misunderstand.

## What BBT is

BBT is:

- local-first;
- deterministic;
- read-only by default;
- structured;
- auditable;
- human-readable;
- agent-readable;
- evidence-oriented;
- useful for coding, debugging, operations, and defensive investigation.

## What BBT is not

BBT is not:

- a byte-for-byte GNU coreutils clone;
- a replacement for mature Linux tools;
- an AI chatbot;
- a cloud-connected sysadmin agent;
- an EDR, SIEM, vulnerability scanner, or pentest framework;
- an exploit toolkit;
- a wrapper around arbitrary shell commands;
- a tool that should execute risky recommendations automatically.

## Product boundary

The tool reports facts. Humans, scripts, and agents make decisions.

BBT may suggest next observations, but those suggestions must be conservative, evidence-backed, and risk-labeled. It should prefer read-only observations and clearly separate risky or mutating actions.

## North star

BBT succeeds when an engineer or agent reaches for:

```bash
bbt host snapshot --agent
bbt fs inspect PATH --agent
bbt disk usage PATH --agent
bbt proc inspect PID --agent
bbt proc list --agent
bbt net listeners --agent
bbt service inspect NAME --agent
```

instead of inventing another fragile pipe-chain.

The long-term destination extends that vocabulary with clearly future commands:

```bash
bbt project inspect PATH --agent   # future
bbt logs inspect SOURCE --agent    # future
```

The purpose stays the same across all of them:

> **Structured Linux facts for humans and agents.**
