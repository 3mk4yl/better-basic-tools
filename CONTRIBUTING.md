# Contributing to Better Basic Tools

Thanks for helping make BBT more useful for humans and agents investigating Linux systems.

## Project scope

BBT is a local, read-only Linux investigation toolkit. It should provide stable structured facts, restrained human output, and agent-facing reports with evidence and safe next observations.

Good contributions usually fit one of these categories:

- bug fixes for existing commands;
- clearer human output;
- stronger JSON/YAML schema coverage;
- safer agent-mode facts, evidence, or next observations;
- documentation improvements;
- new read-only investigation commands.

Out of scope for now:

- destructive remediation actions;
- exploit automation;
- background daemons or cloud services;
- telemetry;
- commands that require secrets or credentials by default.

## Before opening an issue

For bugs, include:

- BBT version or commit;
- Linux distribution and kernel;
- command run;
- expected result;
- actual result;
- sanitized output if possible.

For command proposals, include:

- the investigation question the command answers;
- equivalent shell pipeline(s) people use today;
- proposed human output;
- proposed structured fields;
- what safe next observations would help an agent or operator.

## Development

```bash
git clone https://github.com/akinteldev/better-basic-tools.git
cd better-basic-tools
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

If you change JSON output, update or add schema/tests under `schemas/` and `tests/`.

## Pull request checklist

- [ ] The change is read-only/local-only unless clearly documented otherwise.
- [ ] Human output remains concise and terminal-friendly.
- [ ] Structured output keeps stable field names or bumps schema version.
- [ ] Agent output includes evidence and conservative risk labels.
- [ ] Tests pass.
- [ ] Docs/examples are updated when behavior changes.

## Style notes

- Prefer boring, deterministic behavior over cleverness.
- Evidence should point to observed facts, not guesses.
- Safe next observations should gather more information, not mutate the system.
- If a command may be expensive, expose limits and make them visible in output.
