# Security Model

Better Basic Tools (BBT) is a **local, read-only Linux observation toolkit**. This document defines its security posture, guarantees, and known limitations.

## Guarantees

### Read-only by design

BBT commands observe local system state. They do not:

- modify, create, or delete files (outside of standard stdout/stderr);
- start, stop, restart, or reconfigure services;
- kill or signal processes;
- change permissions, ownership, or mounts;
- write to `/proc`, `/sys`, or any kernel interface.

The only external process BBT executes is `systemctl show` with a fixed, read-only property query (`bbt service inspect`). Arguments are passed as an argument vector — never through a shell — so there is no shell-injection surface.

### No network, no telemetry

- No network I/O of any kind: no HTTP clients, no TLS stacks, no sockets opened.
- No telemetry, analytics, or phone-home behavior.
- `bbt net listeners` reads `/proc/net/{tcp,tcp6,udp,udp6}` as files; it never opens sockets.

This is enforced mechanically in `deny.toml`: `reqwest`, `hyper`, `ureq`, `curl`, `openssl`, `native-tls`, `rustls`, and `tokio` are banned from the dependency graph, and `cargo deny check bans` fails CI if any of them appear.

### Recommendations are labeled, never executed

Agent-mode output suggests next observations but never runs them. Suggested commands carry explicit risk labels (`read-only`, `destructive-medium`, `privileged`, …) and `requires_confirmation` flags. The consuming human or agent decides; BBT only reports.

## Unsafe code review

All `unsafe` blocks are thin wrappers around well-known libc calls, with null-pointer checks before dereference:

| Location | Call | Purpose |
|---|---|---|
| `bbt-fs` | `libc::access` | current-user read/write/execute checks |
| `bbt-proc` | `libc::geteuid`, `libc::getpwuid` | uid and username resolution |
| `bbt-host` | `libc::uname`, `libc::getuid`/`geteuid`/`getgid`/`getegid`, `libc::getpwuid`, `libc::statvfs` | kernel/user/filesystem facts |

No unsafe pointer arithmetic, no transmutes, no FFI beyond libc. Note: `getpwuid` is not thread-safe (returns a pointer to static storage); BBT is single-threaded per invocation, so this is safe in the current architecture. If concurrency is ever added, these must migrate to `getpwuid_r`.

## Dependency and supply-chain policy

- `deny.toml` gates advisories (RustSec), license allowlist, banned network crates, wildcard versions, and unknown registries/git sources.
- Run locally or in CI with:

```bash
cargo deny check
```

- All workspace crates are `publish = false`; distribution is via GitHub release binaries built from tagged source.

### Known accepted risks

- None currently. The YAML output mode uses `serde_norway`, an actively
  maintained fork of the archived `serde_yaml`; it processes only BBT's own
  serialized data, never untrusted input.

## Threat model

### In scope

- BBT must not corrupt or mutate the observed system.
- BBT must not leak data off-host.
- BBT must not provide a privilege-escalation or injection primitive (no shell interpolation, no writable interfaces).
- BBT must degrade safely on hostile local state: unreadable paths, vanishing processes, non-UTF-8 filenames, and huge inputs produce structured warnings, not crashes.

### Out of scope

- BBT does not defend against a hostile root user; it runs with the invoker's privileges and sees only what those privileges allow.
- BBT output can contain sensitive local facts (process command lines, listener inventory, usernames, paths). **Treat BBT JSON/agent output as sensitive** and do not pipe it to untrusted or remote services without review.
- BBT is not a scanner, EDR, SIEM, or integrity monitor. It reports point-in-time facts and makes no tamper-resistance claims — an attacker with local control can fake `/proc` views via namespaces, LD_PRELOAD, or rootkits.

## Privilege guidance

- Run BBT unprivileged by default. Most commands work fully as a normal user.
- Running as root widens visibility (all processes' fd tables, all paths) but is never required for BBT itself to function safely.
- Suggested `sudo bbt …` next steps in agent reports are labeled `privileged` and require confirmation by policy.

## Reporting vulnerabilities

See [`SECURITY.md`](../SECURITY.md) for the disclosure process.
