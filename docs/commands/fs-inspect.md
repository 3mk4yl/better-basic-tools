# `bbt fs inspect`

Inspect one filesystem path and return human-readable, structured, or agent-oriented output.

```bash
bbt fs inspect PATH
bbt fs inspect PATH --json
bbt fs inspect PATH --yaml
bbt fs inspect PATH --agent
```

## Purpose

`bbt fs inspect` is the first Better Basic Tools command and the first concrete
ASI slice. It turns a path into a compact, explainable filesystem object report.

It is read-only and local-only. It does not follow destructive suggestions, make
network calls, or modify files.

## Human mode

```bash
bbt fs inspect Cargo.toml
```

Example shape:

```text
Cargo.toml

Type:        regular file
Exists:      yes
Size:        632 bytes
Owner:       1000
Group:       1000
Mode:        0664  rw-rw-r--
Modified:    2026-05-29T06:12:28.824841518Z

Access for current user:
  Read:      yes
  Write:     yes
  Execute:   no

Safe next observations:
  bbt fs inspect Cargo.toml --json
  bbt fs inspect Cargo.toml --agent
```

## JSON mode

```bash
bbt fs inspect Cargo.toml --json
```

Emits a stable schema envelope:

```json
{
  "schema": "bbt.fs.inspect.v1",
  "generated_at": "2026-05-29T09:23:20Z",
  "command": {
    "argv": ["bbt", "fs", "inspect", "Cargo.toml", "--json"],
    "argv_bytes_hex": [
      "626274",
      "6673",
      "696e7370656374",
      "436172676f2e746f6d6c",
      "2d2d6a736f6e"
    ],
    "cwd": "/home/user/better-basic-tools",
    "effective_uid": 1000
  },
  "data": {
    "path": "Cargo.toml",
    "path_bytes_hex": "436172676f2e746f6d6c",
    "exists": true,
    "kind": "regular-file"
  }
}
```

The actual `data` object includes path, path bytes as lowercase hex for non-UTF-8 Unix path recovery, canonical path where available, kind,
size, uid/gid, mode, current-user access, symlink target, timestamps, and warnings for incomplete inspection cases such as permission-denied metadata access.

## YAML mode

```bash
bbt fs inspect Cargo.toml --yaml
```

Same envelope as JSON, rendered as YAML.

## Agent mode

```bash
bbt fs inspect Cargo.toml --agent
```

Emits an agent report:

```json
{
  "schema": "bbt.agent.report.v1",
  "subject": {
    "domain": "fs",
    "command": "inspect",
    "target": "Cargo.toml"
  },
  "summary": "Cargo.toml is a regular file.",
  "severity": "ok",
  "facts": [
    {
      "key": "path_exists",
      "value": true,
      "severity": "info",
      "evidence": ["Cargo.toml exists"]
    }
  ],
  "safe_next_steps": [
    {
      "command": "bbt fs inspect Cargo.toml --json",
      "risk": "read-only",
      "purpose": "Fetch complete raw filesystem metadata",
      "requires_confirmation": false
    }
  ],
  "risky_next_steps": []
}
```

## Current implementation notes

Implemented in the first MVP slice:

- existing and missing paths;
- UTF-8 display path plus lowercase-hex raw path bytes;
- regular files, directories, symlinks, sockets, FIFOs, block/character devices;
- size in bytes;
- Unix uid/gid;
- octal and symbolic mode;
- current process read/write/execute access via `access(2)`;
- symlink target;
- modified/accessed/changed/created timestamps where available;
- structured warnings instead of hard failure for expected local inspection problems such as permission-denied metadata access;
- human, JSON, YAML, and agent renderers.

Planned refinements:

- uid/gid name resolution;
- richer permission reasoning with group membership evidence;
- broken symlink diagnostics;
- secret-bearing file cautions;
- parent-directory permission chain inspection.
