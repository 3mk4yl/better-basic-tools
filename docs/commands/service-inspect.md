# `bbt service inspect NAME`

Inspect one systemd service with human, structured, or agent-oriented output.

```bash
bbt service inspect ssh.service
bbt service inspect ssh.service --json
bbt service inspect ssh.service --agent
```

## Purpose

`bbt service inspect` queries `systemctl show` with a fixed, read-only property
list and returns load/active/sub state, unit-file state, description, main PID,
fragment path, and last main-process exit status.

It is read-only: BBT never starts, stops, restarts, enables, or reloads
services. `systemctl` is invoked with an argument vector, so names are never
interpreted by a shell. Names beginning with `-` are rejected before invocation
so an option-like unit name cannot be passed to `systemctl`.

## JSON mode

```bash
bbt service inspect ssh.service --json
```

Emits the `bbt.service.inspect.v1` envelope. `data` contains:

- `name`, `exists`;
- `load_state`, `active_state`, `sub_state`, `unit_file_state`;
- `description`, `main_pid`, `fragment_path`, `exec_main_status`;
- `warnings[]`.

Missing or unloaded services return `exists: false` with a warning and exit
code `0` — a missing service is a valid local fact, not a CLI failure (see
[`docs/exit-codes.md`](../exit-codes.md)).

Schema: [`schemas/service-inspect.v1.schema.json`](../../schemas/service-inspect.v1.schema.json).

## Agent mode

```bash
bbt service inspect ssh.service --agent
```

Emits a `bbt.agent.report.v1` report with facts:

- `service_exists`, `active_state`, `main_pid`;
- `service_state_problem` — `true` with `warning` severity when the service
  exists but is not active (report severity is raised to `warning` as well).

## Notes and limitations

- Requires systemd. On non-systemd hosts or when `systemctl` is unavailable,
  the command returns a structured `systemctl unavailable` warning instead of
  failing.
- User units are not queried separately yet; the system manager is asked.
