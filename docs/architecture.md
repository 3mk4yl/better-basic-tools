# Architecture

Better Basic Tools is organized as a Rust workspace around a unified binary,
shared domain crates, and stable schema types.

## Workspace layout

```text
better-basic-tools/
  crates/
    bbt/          # unified `bbt` binary
    bbt-agent/    # agent report types, risk labels, evidence, suggestions
    bbt-cli/      # clap args, format selection, config loading
    bbt-core/     # shared output, errors, units, timestamps, host metadata
    bbt-disk/     # filesystems, mounts, usage, pressure diagnostics
    bbt-fs/       # path inspection, metadata, permissions, traversal
    bbt-host/     # host snapshot: OS, kernel, user, CPU, load, memory
    bbt-net/      # listening-socket inventory from /proc/net
    bbt-proc/     # process inspection, process pressure, procfs helpers
    bbt-service/  # systemd service inspection via read-only systemctl show
  docs/
  schemas/
```

## Unified binary

The primary interface is:

```bash
bbt <domain> <command> [args]
```

Current domains:

- `host` — host summary, health, snapshot;
- `fs` — files, directories, metadata, permissions;
- `disk` — filesystems, usage, pressure;
- `proc` — process lists, trees, inspection, pressure;
- `net` — listening-socket inventory;
- `service` — systemd service inspection;
- future: `logs`, `package`.

## Output layers

Every command should be built from the same internal model and then rendered in
one of three layers.

### Human mode

Default output for direct terminal use. It should be restrained, stable enough
for humans, and readable without being cute.

### Data mode

Complete raw structured output:

```bash
--format json
--format yaml
# convenience aliases:
--json
--yaml
```

Structured schemas are a compatibility contract and should be versioned.

### Agent mode

A compact diagnostic report for agentic systems:

- summary;
- facts;
- evidence;
- interpretation;
- safe next observations;
- risky possible actions;
- risk labels;
- command provenance.

Agent mode is not merely JSON. It is JSON organized for reasoning.

## Risk model

Suggested commands and possible actions should carry risk labels:

- `read-only` — observes state only;
- `mutating-low` — small reversible change;
- `mutating-medium` — system state change requiring care;
- `destructive-low` — deletes/regenerates bounded data;
- `destructive-medium` — may remove useful data or disrupt services;
- `destructive-high` — irreversible or broad destructive impact;
- `privileged` — requires elevated privileges;
- `networked` — contacts external systems.

Risk labels can be combined when needed.

## Error philosophy

Errors should include:

- what failed;
- why it likely failed;
- evidence available locally;
- whether elevated privileges may help;
- safe next observations.

Example:

```text
Cannot read /var/log/nginx/access.log

Reason:
  current user is not allowed to read this file

Evidence:
  owner: www-data
  group: adm
  mode: 0640
  current user groups: user,sudo,docker

Safe next observations:
  bbt fs inspect /var/log/nginx/access.log --agent
  id
```
