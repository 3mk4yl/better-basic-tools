# `bbt proc list`

Inventory of visible local processes with human, structured, or agent-oriented output.

```bash
bbt proc list
bbt proc list --json
bbt proc list --agent
```

## Purpose

`bbt proc list` reads numeric entries under `/proc` and returns a compact
process inventory: pid, parent pid, name, state, uid/user, resident memory,
and command line. It is the structured, agent-consumable counterpart to a
quick `ps` sweep.

It is read-only and local-only.

## JSON mode

```bash
bbt proc list --json
```

Emits the `bbt.proc.list.v1` envelope. `data` contains:

- `processes[]` — pid, parent_pid, name, state, uid, user, vm_rss_bytes, command_line;
- `process_count`;
- `warnings[]` — collection problems (vanished processes, permission limits).

Schema: [`schemas/proc-list.v1.schema.json`](../../schemas/proc-list.v1.schema.json).

## Agent mode

```bash
bbt proc list --agent
```

Emits a `bbt.agent.report.v1` report with triage facts:

- `process_count` and `current_user_process_count`;
- `root_process_count` — processes running as uid 0;
- `zombie_process_count` — `warning` severity when greater than zero;
- `top_rss_process` — pid, name, user, and VmRSS of the largest resident process.

Safe next steps point to the structured inventory and to
`bbt --agent net listeners` for socket correlation.

## Notes and limitations

- Kernel threads appear with empty command lines; that is expected.
- Short-lived processes may vanish mid-scan; these produce warnings, not failures.
- VmRSS is unavailable for kernel threads and reported as `null`.
- Visibility follows the invoking user's permissions; run as root to widen it.
