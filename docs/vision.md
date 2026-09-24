# Vision: structured Linux facts for humans and agents

**ASI — Agentic System Interface for Linux** is the concept behind Better Basic
Tools. The purpose is simple:

> **BBT helps humans and agents understand Linux systems without fragile shell glue.**

The terminal will not disappear. But the consumers of terminal tools are
changing. Linux operations are increasingly performed by a mix of humans,
scripts, and agentic AI systems. Those consumers need different views of the same
operating-system truth.

## The problem

Classic UNIX tools are excellent at exposing text streams. But when coding,
debugging, operating, or investigating a system, humans and agents often need to
stitch together many separate commands:

```bash
df -h
du -sh /*
mount
lsblk
stat /path
getfacl /path
ps aux
systemctl status service
journalctl -xeu service
```

This is powerful, but it has costs:

- fragile parsing;
- repeated shell round trips;
- locale/unit ambiguity;
- missing provenance;
- little explanation;
- no risk classification;
- expensive context for LLM-based agents.

## The opportunity

Better Basic Tools should provide **local situation awareness** for common Linux
investigation tasks:

- a concise human summary;
- stable raw structured facts;
- interpretations backed by evidence;
- safe next observations;
- risk-labeled possible actions.

The goal is not to automate judgment away. The goal is to make system state more
legible to both humans and agents.

## Human-native, agent-readable

The project should never feel like software only for robots. A sysadmin should
want to use it directly:

```bash
bbt host snapshot
bbt fs inspect .
bbt disk usage . --depth 1
bbt disk pressure
```

But the same commands should also offer schemas that agents can rely on:

```bash
bbt host snapshot --agent
bbt fs inspect . --json
```

## Determinism over magic

Better Basic Tools should not embed an LLM in the core toolchain. It should not
make network calls, send telemetry, or invent advice. Diagnostics should be
rule-based, explainable, local, and auditable.

The intelligence lives in:

- good schemas;
- good domain modeling;
- clear evidence;
- conservative heuristics;
- risk-aware next steps.

## Defensive fit

BBT is not a defensive platform, SIEM, EDR, scanner, or pentest framework. It is
a local evidence layer. That makes it especially useful for defenders because
defensive work depends on repeatable facts: what is running, what changed, what
is listening, what files exist, what artifacts were produced, and what safe
observations should happen next.

Defensive local investigation inside controlled, authorized environments is a
demanding validation domain for these qualities. BBT should provide facts; the
environment's design, firewalling, containment, and response decisions remain
outside BBT.
