# `bbt fs list PATH`

List the immediate entries of one directory and return human-readable, structured, or agent-oriented output.

```bash
bbt fs list            # PATH defaults to .
bbt fs list /var/log
bbt fs list /var/log --json
bbt fs list /var/log --yaml
bbt fs list /var/log --agent
```

## Purpose

`bbt fs list` is the Better Basic Tools answer to the everyday `ls -l` question:

> What is in this directory, what kind of thing is each entry, and who owns it?

It is intentionally conservative:

- read-only;
- non-recursive — only the immediate entries of `PATH`, never a tree walk (`bbt disk usage` covers recursive totals);
- never follows symlinks: every entry is described with `lstat` semantics, so a symlink reports its own size, mode, and target, not the target's metadata;
- sorts entries by name bytes for stable, deterministic output;
- degrades to structured warnings with exit `0`: a missing `PATH`, a non-directory `PATH`, an unreadable directory, or individual entries whose metadata cannot be read all produce `warnings[]` entries instead of failures;
- preserves non-UTF-8 names: `name` is lossy for display, `name_bytes_hex` carries the exact bytes.

A non-directory `PATH` is reported as itself — one entry describing the path, plus a warning — matching `ls FILE` behavior instead of silently listing something else.

## Output modes

### Human

```text
Directory listing: docs

Entries: 3

MODE         OWNER   GROUP         SIZE  MODIFIED                        NAME
drwxrwxr-x    1000    1000         4096  2026-07-26T12:09:06.193484113Z  commands
-rw-rw-r--    1000    1000         6744  2026-07-13T18:58:27.089968992Z  purpose.md
lrwxrwxrwx    1000    1000           10  2026-07-26T12:00:00.000000000Z  latest -> purpose.md

Safe next observations:
  bbt --json fs list -- docs
  bbt --agent fs list -- docs
```

The first `MODE` character is the file type (`d` directory, `-` regular file, `l` symlink, `s` socket, `p` FIFO, `b`/`c` block/char device).

### JSON envelope

```json
{
  "schema": "bbt.fs.list.v1",
  "generated_at": "2026-07-26T12:00:00Z",
  "command": {
    "argv": ["bbt", "fs", "list", "docs", "--json"],
    "argv_bytes_hex": ["626274", "6673", "6c697374", "646f6373", "2d2d6a736f6e"],
    "cwd": "/home/user/better-basic-tools",
    "effective_uid": 1000
  },
  "data": {
    "path": "docs",
    "path_bytes_hex": "646f6373",
    "absolute_path": "/home/user/better-basic-tools/docs",
    "exists": true,
    "is_directory": true,
    "entry_count": 1,
    "entries": [
      {
        "name": "purpose.md",
        "name_bytes_hex": "707572706f73652e6d64",
        "kind": "regular-file",
        "size_bytes": 6744,
        "owner": { "uid": 1000, "name": null },
        "group": { "gid": 1000, "name": null },
        "mode": { "octal": "0664", "symbolic": "rw-rw-r--" },
        "modified": "2026-07-13T18:58:27.089968992Z",
        "symlink_target": null
      }
    ],
    "warnings": []
  }
}
```

The schema lives at:

```text
schemas/fs-list.v1.schema.json
```

### Agent report

`--agent` emits a `bbt.agent.report.v1` report with facts such as:

- `path_exists`, `is_directory`
- `entry_count`
- `hidden_entry_count` (names starting with a dot)
- `largest_entry` (by `lstat` size; directory sizes are not recursive)

interpretation when warnings limited completeness, and safe next-step commands only (for example `bbt --agent fs inspect` on the largest entry, or `bbt --agent disk usage` for recursive totals).

## Notes

- `size_bytes` is the `lstat` size of the entry itself: a directory reports its directory-file size (not its content total), a symlink reports its target-path length. Use `bbt disk usage` for recursive totals.
- `kind`, `owner`, `group`, `mode`, and `modified` reuse the exact `fs inspect` field shapes, so consumers can share parsing logic between the two commands.
- Entries whose metadata cannot be read (for example a readable but non-searchable directory) are skipped with a `warnings[]` entry naming the path; the rest of the listing is still returned.
- Sort order is by raw name bytes, not locale collation, so output is identical across environments.
- `entry_count` counts returned entries only; skipped-with-warning entries are not included.
- Current implementation is Unix/Linux-oriented.
