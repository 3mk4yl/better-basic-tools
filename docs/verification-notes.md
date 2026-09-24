# Verification notes

Environment-specific pitfalls discovered while adversarially checking bbt's
numeric output against ground-truth system tools. Read this before treating
any `bbt` vs. system-tool numeric mismatch as a bug.

## `du -b` is not GNU `du -b` on uutils/rust-coreutils systems

**Symptom:** `bbt disk usage PATH --json` reports `total_bytes` noticeably
higher (10-30%) than `du -sb PATH`, especially in directories with many
symlinks.

**Root cause:** This is a measurement artifact, not a bbt defect. On systems
running `uutils coreutils` (`rust-coreutils` package, `du --version` reports
`du (uutils coreutils) ...`), the `-b` flag behaves like GNU `du`'s
`--apparent-size`, not like true allocated-block accounting. `du -sb` and
`du -s --apparent-size --block-size=1` return identical output on such
systems. This silently invalidates `-b` as a block-usage baseline.

**Correct verification command:** use `du -s --block-size=1 PATH` (without
`-b`) to get true `st_blocks * 512` disk-block accounting to compare against
bbt's `total_bytes`. Confirmed byte-exact match against bbt on a real,
symlink-heavy directory (2026-07-25): both reported `1187737600` for the same
tree at the same moment.

**Check which `du` you have before trusting `-b`:**

```bash
du --version | head -1
# "du (GNU coreutils) ..."   -> -b is safe, true block accounting
# "du (uutils coreutils) ..." -> -b == --apparent-size, do NOT use for
#                                 block-usage comparisons; use
#                                 `du -s --block-size=1` instead
```

**For `apparent_bytes` comparisons:** uutils `du --apparent-size` counts only
regular files and symlinks. bbt's `apparent_bytes` includes directory
`st_size`, matching GNU `du --apparent-size` behavior instead. This produces
a small, expected, deliberate delta (directory count × directory block size,
typically 4096 bytes each) — not a bug, and not planned to change.

## General rule

Before filing or acting on a numeric discrepancy between bbt and a reference
tool, confirm:

1. Which implementation of the reference tool is actually installed
   (GNU vs. BSD vs. uutils/rust-coreutils vs. busybox) — flag semantics are
   not guaranteed identical across implementations even when the flag name
   matches.
2. The exact accounting mode being compared (apparent size vs. allocated
   disk blocks vs. logical file size) on both sides.
3. That both readings were taken at effectively the same moment, since
   caches/logs/temp files can churn between two sequential commands.

An adversarial check that skips step 1 can produce a confident, wrong bug
report — as happened on 2026-07-25 with `bbt disk usage`, which cost a full
investigation cycle before the "bug" was found to be a `du` measurement
artifact. See commit `845c978` for the resolution and the regression test
that came out of it.
