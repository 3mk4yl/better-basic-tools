# `bbt host snapshot`

Capture a read-only, one-shot host situation report for humans, scripts, and agents.

```bash
bbt host snapshot
bbt host snapshot --json
bbt host snapshot --yaml
bbt host snapshot --agent
```

## Purpose

`bbt host snapshot` answers:

> What machine am I on, what OS/kernel is running, what does memory pressure look like, and which mounted filesystems matter?

It is intentionally local and conservative:

- read-only;
- no network calls;
- no service control;
- Linux `/proc` and `uname` based;
- emits warnings instead of failing when optional fields cannot be collected.

## Collected fields

- Hostname from `/proc/sys/kernel/hostname`.
- Kernel information from `uname(2)`.
- OS identity from `/etc/os-release`.
- Current real/effective user and group IDs, with passwd username/home/shell where available.
- CPU logical processor count and first model/vendor/frequency summary from `/proc/cpuinfo`.
- Uptime from `/proc/uptime`.
- Load average from `/proc/loadavg`.
- Memory summary from `/proc/meminfo`.
- Filesystem usage from `/proc/mounts` + `statvfs(2)`.

Pseudo/noisy filesystems such as `proc`, `sysfs`, `cgroup2`, `devtmpfs`, and similar are filtered from the filesystem summary.

## JSON envelope

JSON output uses schema:

```text
bbt.host.snapshot.v1
```

Schema file:

```text
schemas/host-snapshot.v1.schema.json
```

Example shape:

```json
{
  "schema": "bbt.host.snapshot.v1",
  "generated_at": "2026-05-29T12:00:00Z",
  "command": {
    "argv": ["bbt", "host", "snapshot", "--json"],
    "argv_bytes_hex": ["626274", "686f7374", "736e617073686f74", "2d2d6a736f6e"],
    "cwd": "/home/user/better-basic-tools",
    "effective_uid": 1000
  },
  "data": {
    "hostname": "workstation",
    "kernel": {
      "sysname": "Linux",
      "release": "7.0.0-15-generic",
      "version": "#15-Ubuntu SMP",
      "machine": "x86_64"
    },
    "os": {
      "pretty_name": "Ubuntu ...",
      "id": "ubuntu",
      "version_id": "..."
    },
    "current_user": {
      "uid": 1000,
      "effective_uid": 1000,
      "gid": 1000,
      "effective_gid": 1000,
      "username": "alice",
      "home": "/home/alice",
      "shell": "/bin/bash"
    },
    "cpu": {
      "logical_cpus": 16,
      "model_name": "Example CPU",
      "vendor_id": "GenuineIntel",
      "cpu_mhz": 2400.0
    },
    "uptime": { "seconds": 12345.67 },
    "load_average": {
      "one_minute": 0.12,
      "five_minutes": 0.20,
      "fifteen_minutes": 0.25
    },
    "memory": {
      "total_bytes": 33554432,
      "available_bytes": 16777216,
      "free_bytes": 8388608,
      "used_percent": 50.0
    },
    "filesystems": [],
    "warnings": []
  }
}
```

## Agent report

`--agent` emits `bbt.agent.report.v1` with facts including:

- `hostname`
- `kernel_release`
- `cpu_logical_cpus`
- `current_user`
- `memory_used_percent`
- `filesystem_count`

Safe next steps are read-only commands only. The root disk-usage follow-up is depth-limited:

```bash
bbt --agent disk usage --depth 1 -- /
```
