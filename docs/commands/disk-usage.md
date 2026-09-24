# `bbt disk usage`

Calculate read-only disk usage for one path and return human-readable, structured, or agent-oriented output.

```bash
bbt disk usage PATH
bbt disk usage PATH --json
bbt disk usage PATH --yaml
bbt disk usage PATH --agent
bbt disk usage PATH --depth 1 --top 5 --one-filesystem --json
```

## Purpose

`bbt disk usage` is the Better Basic Tools answer to the everyday `du` question:

> How much space does this path use, what are its biggest immediate children, and what could not be inspected?

It is intentionally conservative:

- read-only;
- does not delete or mutate files;
- uses `symlink_metadata`, so symlink targets are not traversed as regular children;
- reports unreadable/failed entries instead of hiding them;
- preserves raw Unix path bytes as lowercase hex for non-UTF-8 paths;
- supports `--depth 0` (totals only) and `--depth 1` (immediate children) without changing recursive totals; larger values are rejected at CLI parsing because this command reports at most one child level — use `bbt fs tree PATH --depth N` for deeper per-subtree reports;
- supports `--top N` to control how many largest immediate children are shown;
- supports `--one-filesystem` to avoid descending onto different filesystem devices.

## Output modes

### Human

```text
Disk usage: /var/log

Exists:              yes
Max depth:           unlimited
One filesystem:      no
Top limit:           10
Total:              1.2 GiB (1234567890)
Apparent size:      1.1 GiB (1134567890)
Entries seen:        1204
Directories seen:    41
Files seen:          1159
Symlinks seen:       4
Unreadable entries:  0
Skipped other fs:    0

Largest children:
     900.0 MiB  directory    /var/log/journal
      60.0 MiB  regular file /var/log/syslog

Safe next observations:
  bbt --json disk usage -- /var/log
  bbt --agent disk usage -- /var/log
```

### JSON envelope

```json
{
  "schema": "bbt.disk.usage.v1",
  "generated_at": "2026-05-29T12:00:00Z",
  "command": {
    "argv": ["bbt", "disk", "usage", "/var/log", "--json"],
    "argv_bytes_hex": ["626274", "6469736b", "7573616765", "2f7661722f6c6f67", "2d2d6a736f6e"],
    "cwd": "/home/user/better-basic-tools",
    "effective_uid": 1000
  },
  "data": {
    "path": "/var/log",
    "path_bytes_hex": "2f7661722f6c6f67",
    "absolute_path": "/var/log",
    "exists": true,
    "max_depth": null,
    "one_filesystem": false,
    "top_limit": 10,
    "total_bytes": 1234567890,
    "apparent_bytes": 1134567890,
    "entries_seen": 1204,
    "directories_seen": 41,
    "files_seen": 1159,
    "symlinks_seen": 4,
    "unreadable_entries": 0,
    "skipped_different_filesystem": 0,
    "largest_children": [],
    "errors": []
  }
}
```

The schema lives at:

```text
schemas/disk-usage.v1.schema.json
```

### Agent report

`--agent` emits a `bbt.agent.report.v1` report with facts such as:

- `path_exists`
- `total_bytes`
- `entries_seen`

and safe next-step commands only.

## Notes

- `total_bytes` uses Unix allocated blocks (`st_blocks * 512`) where available.
- `apparent_bytes` uses logical file length (`metadata.len()`).
- Largest children are immediate children, sorted by recursively calculated allocated bytes.
- `--depth 0` reports only the root row while still calculating recursive root totals, matching `du --max-depth=0`-style expectations.
- `--depth 1` reports immediate children with recursive child totals, matching `du --max-depth=1`-style expectations.
- `--depth` values greater than `1` are rejected as a CLI usage error: the flat `largest_children` field is an immediate-child summary, and deeper report trees are not exposed in the v1 schema.
- A directory encountered a second time on the same device (for example via a bind-mount cycle) is skipped once with a `directory-cycle` error entry instead of being re-traversed or double counted.
- `--one-filesystem` skips children whose device differs from the root path's device and increments `skipped_different_filesystem`.
- `--top 0` suppresses the largest-children list; `--top N` does not short-circuit traversal or change total calculations.
- Current implementation is Unix/Linux-oriented.
