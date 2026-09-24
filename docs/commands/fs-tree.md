# `bbt fs tree PATH`

Show a bounded-depth directory tree with recursive per-subtree size totals and
return human-readable, structured, or agent-oriented output.

```bash
bbt fs tree                      # PATH defaults to .
bbt fs tree /var/log
bbt fs tree /var/log --depth 2
bbt fs tree /var/log --one-filesystem
bbt fs tree /var/log --json
bbt fs tree /var/log --yaml
bbt fs tree /var/log --agent
```

## Purpose

`bbt fs tree` is the Better Basic Tools answer to the everyday `tree` question:

> How is this directory structured, and where does the space actually go?

It is intentionally conservative:

- read-only;
- recursive for *totals*, bounded for *reporting*: `--depth N` limits how many
  levels of `children` are reported, while every reported node (including the
  root) always carries its full recursive subtree totals. `--depth 0` reports
  only the root node; `--depth 1` reports the immediate children with *their*
  recursive totals. `bbt disk usage` shares these semantics for its supported
  depths `0` and `1`; use `fs tree` for deeper per-subtree reporting;
- never follows symlinks: every node is described with `lstat` semantics, so a
  symlink to a directory is reported as a symlink leaf and is not recursed
  into;
- counts hard-linked file content once, like `du`;
- `--one-filesystem` skips entries on a different filesystem device than the
  root path and counts them in `skipped_different_filesystem`;
- sorts children by name bytes for stable, deterministic output;
- degrades to structured warnings with exit `0`: a missing `PATH`, a
  non-directory `PATH` (reported as a single leaf node), an unreadable
  subdirectory mid-traversal (skipped with a warning; the rest of the tree is
  still reported), and entries whose metadata cannot be read all produce
  `warnings[]` entries instead of failures;
- preserves non-UTF-8 names: `name` is lossy for display, `name_bytes_hex`
  carries the exact bytes.

Sizes are reported both ways, matching `bbt disk usage`: `total_bytes` is
allocated bytes (`st_blocks * 512`), `apparent_bytes` is the `lstat` length.

## Output modes

### Human

```text
Directory tree: docs

Max depth:           2
One filesystem:      no
Total:               132.0 KiB (135168)
Apparent size:       93.0 KiB (95276)
Entries seen:        27
Skipped other fs:    0

docs  [132.0 KiB]
├── commands  [48.0 KiB]
│   ├── disk-usage.md  [4.0 KiB]
│   └── fs-list.md  [8.0 KiB]
├── purpose.md  [8.0 KiB]
└── roadmap.md  [8.0 KiB]

Safe next observations:
  bbt --json fs tree -- docs
  bbt --agent fs tree -- docs
```

Every node is annotated with its recursive allocated size; directory
annotations therefore include everything below them, even below the reporting
depth cutoff.

### JSON envelope

```json
{
  "schema": "bbt.fs.tree.v1",
  "generated_at": "2026-07-26T12:00:00Z",
  "command": {
    "argv": ["bbt", "fs", "tree", "docs", "--depth", "1", "--json"],
    "argv_bytes_hex": ["626274", "6673", "74726565", "646f6373", "2d2d6465707468", "31", "2d2d6a736f6e"],
    "cwd": "/home/user/better-basic-tools",
    "effective_uid": 1000
  },
  "data": {
    "path": "docs",
    "path_bytes_hex": "646f6373",
    "absolute_path": "/home/user/better-basic-tools/docs",
    "exists": true,
    "max_depth": 1,
    "one_filesystem": false,
    "root": {
      "name": "docs",
      "name_bytes_hex": "646f6373",
      "kind": "directory",
      "total_bytes": 135168,
      "apparent_bytes": 95276,
      "entries_seen": 27,
      "children": [
        {
          "name": "commands",
          "name_bytes_hex": "636f6d6d616e6473",
          "kind": "directory",
          "total_bytes": 49152,
          "apparent_bytes": 30887,
          "entries_seen": 11
        },
        {
          "name": "purpose.md",
          "name_bytes_hex": "707572706f73652e6d64",
          "kind": "regular-file",
          "total_bytes": 8192,
          "apparent_bytes": 6744
        }
      ]
    },
    "entries_seen": 27,
    "max_depth_reached": 2,
    "skipped_different_filesystem": 0,
    "warnings": []
  }
}
```

`root` is a recursive `TreeNode`: directories carry `entries_seen` (entries in
that subtree, including the directory itself) and — at levels above the depth
cutoff — a `children` array; the `children` key is omitted below the cutoff
and on non-directory nodes. `root` is `null` only when `PATH` does not exist.

The schema lives at:

```text
schemas/fs-tree.v1.schema.json
```

### Agent report

`--agent` emits a `bbt.agent.report.v1` report with facts such as:

- `path_exists`
- `total_bytes` (recursive allocated bytes for the root)
- `entries_seen` (full recursive walk; `--depth` limits reporting only)
- `max_depth_reached` (deepest level observed vs. the requested reporting depth)
- `largest_subtree` (largest immediate child by recursive allocated bytes)

interpretation when warnings limited completeness, and safe next-step commands
only (for example `bbt --agent disk usage --depth 1` on the largest subtree,
or `bbt --agent fs list` for one directory's flat contents).

## Notes

- `--depth` never changes totals: traversal is always complete, so the root
  and every reported directory carry exact recursive subtree totals — only the
  amount of reported structure shrinks. `bbt disk usage` shares this behavior
  for depths `0` and `1`, while `fs tree` supports deeper reporting.
- `total_bytes` sums allocated blocks (`st_blocks * 512`) and `apparent_bytes`
  sums `lstat` lengths, both including directory inodes themselves — identical
  accounting to `bbt disk usage` on the same path.
- Hard links are counted once per `(device, inode)` pair; later occurrences
  report `0` bytes, so subtree totals never double-count content.
- `kind` reuses the exact `fs inspect`/`fs list` file-kind vocabulary, so
  consumers can share parsing logic across the `fs` commands.
- An unreadable subdirectory is still reported as a node (with an empty
  `children` array where reporting depth allows) and its own allocated size;
  its contents are missing from totals, and a `warnings[]` entry names it.
- Sort order is by raw name bytes, not locale collation, so output is
  identical across environments.
- Use `bbt fs list` for one directory's flat contents with ownership and mode
  detail, and `bbt disk usage` for a largest-children ranking instead of a
  tree.
- Current implementation is Unix/Linux-oriented.
