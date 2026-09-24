# `bbt proc inspect PID`

Inspect one Linux process from `/proc` using read-only local observations.

```bash
bbt proc inspect 1234
bbt proc inspect 1234 --json
bbt proc inspect 1234 --yaml
bbt proc inspect 1234 --agent
```

## Purpose

`bbt proc inspect PID` answers:

> What is this process, who owns it, where did it start from, what is its parent/child context, and how much memory is it using?

It is intentionally conservative:

- read-only;
- no signals;
- no process control;
- no network calls;
- Linux `/proc` based;
- missing fields are warnings, not fatal errors, because processes can exit during inspection.

## Collected fields

- PID visibility from `/proc/PID`.
- Name, state, parent PID, UID/GID, threads, and memory from `/proc/PID/status`.
- Fallback name/state/parent PID from `/proc/PID/stat`.
- Command line from `/proc/PID/cmdline`, including lossy strings and raw argument bytes as hex.
- Executable and current working directory from `/proc/PID/exe` and `/proc/PID/cwd` symlinks.
- Child PIDs by scanning visible `/proc/*/status` entries for matching `PPid`.
- Username from the local passwd database where available.

## JSON envelope

JSON output uses schema:

```text
bbt.proc.inspect.v1
```

Schema file:

```text
schemas/proc-inspect.v1.schema.json
```

Example shape:

```json
{
  "schema": "bbt.proc.inspect.v1",
  "generated_at": "2026-05-30T12:00:00Z",
  "command": {
    "argv": ["bbt", "proc", "inspect", "1234", "--json"],
    "argv_bytes_hex": ["626274", "70726f63", "696e7370656374", "31323334", "2d2d6a736f6e"],
    "cwd": "/home/user/project",
    "effective_uid": 1000
  },
  "data": {
    "pid": 1234,
    "exists": true,
    "name": "python",
    "state": "S (sleeping)",
    "parent_pid": 1,
    "child_pids": [],
    "command_line": ["python", "server.py"],
    "command_line_bytes_hex": ["707974686f6e", "7365727665722e7079"],
    "executable": "/usr/bin/python3.12",
    "cwd": "/srv/app",
    "uid": 1000,
    "user": "alice",
    "gid": 1000,
    "threads": 4,
    "memory": {
      "vm_peak_bytes": 123456,
      "vm_size_bytes": 123456,
      "vm_rss_bytes": 654321,
      "vm_hwm_bytes": 654321
    },
    "warnings": []
  }
}
```

## Agent report

`--agent` emits `bbt.agent.report.v1` with facts including:

- `process_exists`
- `process_state`
- `parent_pid`
- `child_pid_count`
- `rss_bytes`

Safe next steps are read-only commands only, including structured process output and a surrounding host snapshot.
