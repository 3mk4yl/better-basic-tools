# `bbt disk pressure`

Report kernel I/O pressure stall information (PSI) and return human-readable, structured, or agent-oriented output.

```bash
bbt disk pressure
bbt disk pressure --json
bbt disk pressure --yaml
bbt disk pressure --agent
```

## Purpose

`bbt disk pressure` answers a question capacity numbers cannot:

> Are processes on this machine actually waiting on I/O right now?

It reads the kernel's Pressure Stall Information for I/O from `/proc/pressure/io`. This measures time processes spend stalled waiting on I/O — it is unrelated to filesystem capacity (`bbt disk filesystems`).

It is intentionally conservative:

- read-only;
- takes no arguments and requires no elevated privileges;
- reads only `/proc/pressure/io`;
- when PSI is unavailable (kernel older than 4.20, `CONFIG_PSI` disabled, or `psi=0` boot parameter) it reports `available: false` with a warning instead of failing.

Each PSI line carries three sliding-window averages and a cumulative total:

- `avg10` / `avg60` / `avg300` — percentage of wall time stalled over the last 10/60/300 seconds;
- `total` — cumulative stall time in microseconds since boot (`total_stalled_usec` in bbt output);
- `some` — at least one task was stalled on I/O;
- `full` — all non-idle tasks were stalled on I/O simultaneously.

## Output modes

### Human

```text
Disk I/O pressure (PSI)

          AVG10    AVG60   AVG300      TOTAL STALL
some      0.31%    0.25%    0.10%         1216.5 s
full      0.31%    0.25%    0.10%         1192.6 s

some = at least one task stalled on I/O; full = all non-idle tasks stalled.

Safe next observations:
  bbt --json disk pressure
  bbt --agent disk pressure
```

When PSI is unavailable the table is replaced by a clear statement and a `Warnings:` section.

### JSON envelope

```json
{
  "schema": "bbt.disk.pressure.v1",
  "generated_at": "2026-07-26T12:00:00Z",
  "command": {
    "argv": ["bbt", "disk", "pressure", "--json"],
    "argv_bytes_hex": ["626274", "6469736b", "7072657373757265", "2d2d6a736f6e"],
    "cwd": "/home/user/better-basic-tools",
    "effective_uid": 1000
  },
  "data": {
    "available": true,
    "some": {
      "avg10": 0.31,
      "avg60": 0.25,
      "avg300": 0.1,
      "total_stalled_usec": 1216511894
    },
    "full": {
      "avg10": 0.31,
      "avg60": 0.25,
      "avg300": 0.1,
      "total_stalled_usec": 1192581388
    },
    "warnings": []
  }
}
```

The schema lives at:

```text
schemas/disk-pressure.v1.schema.json
```

### Agent report

`--agent` emits a `bbt.agent.report.v1` report with facts such as:

- `psi_available`
- `some_avg10`, `some_avg60`, `some_avg300`
- `full_pressure`

interpretation of the current stall level (normal below 10% `some` avg10, elevated from 10%, high from 25%), and safe next-step commands only (for example correlating with `bbt proc list` or `bbt disk filesystems`).

## Notes

- PSI averages are percentages of wall time, not of disk bandwidth; a `some` value of 100% means at least one task was stalled during the entire window.
- `full` is usually lower than `some`; sustained high `full` values mean the whole workload is blocked on I/O.
- `total_stalled_usec` is monotonically increasing since boot; deltas between two runs give exact stall time for the interval.
- A malformed or partially readable `/proc/pressure/io` degrades gracefully: parseable lines are kept, the rest become `null` plus a `warnings[]` entry.
- This command reports I/O pressure only; the kernel exposes `/proc/pressure/cpu` and `/proc/pressure/memory` in the same format for other resources.
- Current implementation is Linux-only by nature of the data source.
