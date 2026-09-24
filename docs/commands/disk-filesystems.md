# `bbt disk filesystems`

List all mounted filesystems with capacity and usage and return human-readable, structured, or agent-oriented output.

```bash
bbt disk filesystems
bbt disk filesystems --json
bbt disk filesystems --yaml
bbt disk filesystems --agent
```

## Purpose

`bbt disk filesystems` is the Better Basic Tools answer to the everyday `df` question:

> What is mounted, how big is each filesystem, and how full is it?

It is intentionally conservative:

- read-only;
- takes no arguments and requires no elevated privileges;
- reads `/proc/mounts` and queries each mount target with `statvfs`;
- filters pseudo-filesystem noise (`proc`, `sysfs`, `cgroup2`, `devpts`, and similar) so the list stays focused on real capacity;
- unescapes octal-escaped mount fields (`\040` and friends) before calling `statvfs`;
- reports `statvfs` failures as warnings with `null` capacity fields instead of hiding the mount;
- sorts entries by mount target for stable output.

The same enumeration backs the `filesystems` section of `bbt host snapshot`, so both commands always agree.

## Output modes

### Human

```text
Mounted filesystems: 3

TARGET                       FSTYPE           SIZE      AVAIL    USED  SOURCE
/                            ext4        250.9 GiB  219.3 GiB   12.6%  /dev/sda2
/dev/shm                     tmpfs         3.6 GiB    3.6 GiB    0.0%  tmpfs
/tmp                         tmpfs         3.6 GiB    3.6 GiB    0.2%  tmpfs

Safe next observations:
  bbt --json disk filesystems
  bbt --agent disk filesystems
```

### JSON envelope

```json
{
  "schema": "bbt.disk.filesystems.v1",
  "generated_at": "2026-07-26T12:00:00Z",
  "command": {
    "argv": ["bbt", "disk", "filesystems", "--json"],
    "argv_bytes_hex": ["626274", "6469736b", "66696c6573797374656d73", "2d2d6a736f6e"],
    "cwd": "/home/user/better-basic-tools",
    "effective_uid": 1000
  },
  "data": {
    "filesystems": [
      {
        "source": "/dev/sda2",
        "target": "/",
        "fstype": "ext4",
        "total_bytes": 269424332800,
        "available_bytes": 235467268096,
        "used_percent": 12.6
      }
    ],
    "warnings": []
  }
}
```

The schema lives at:

```text
schemas/disk-filesystems.v1.schema.json
```

### Agent report

`--agent` emits a `bbt.agent.report.v1` report with facts such as:

- `filesystem_count`
- `fullest_filesystem`

interpretation of the fullest filesystem's capacity level, and safe next-step commands only.

## Notes

- `total_bytes` and `available_bytes` derive from `statvfs` fragment-size arithmetic (`f_blocks * f_frsize`, `f_bavail * f_frsize`); `available_bytes` reflects space available to unprivileged users, matching `df` semantics.
- `used_percent` is `(total - available) / total`; it is `null` when `statvfs` reports zero total blocks (for example `bpf` mounts) or when `statvfs` fails.
- A mount whose `statvfs` call fails still appears in the list with `null` capacity fields, plus a `warnings[]` entry naming the target.
- Bind mounts and duplicate mounts of the same device each appear as their own entry, exactly as `/proc/mounts` reports them; totals are not deduplicated or summed.
- The pseudo-filesystem filter drops `autofs`, `binfmt_misc`, `cgroup`, `cgroup2`, `configfs`, `debugfs`, `devpts`, `devtmpfs`, `fusectl`, `hugetlbfs`, `mqueue`, `proc`, `pstore`, `securityfs`, `sysfs`, and `tracefs`; `tmpfs` is kept because it consumes real memory capacity.
- Current implementation is Unix/Linux-oriented.
