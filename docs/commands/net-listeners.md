# `bbt net listeners`

Inventory of listening TCP/UDP sockets with human, structured, or agent-oriented output.

```bash
bbt net listeners
bbt net listeners --json
bbt net listeners --agent
```

## Purpose

`bbt net listeners` parses `/proc/net/tcp`, `tcp6`, `udp`, and `udp6` and
returns listening sockets with protocol, address, port, inode, and — where
visible — owning processes. It is the structured, agent-consumable counterpart
to `ss -tulpn`, without requiring elevated privileges to produce useful output.

It is strictly read-only: BBT never opens sockets; it reads kernel-provided
files. IPv4 and IPv6 addresses are decoded to canonical form (`127.0.0.1`,
`::1`, `::`), never raw kernel hex.

## JSON mode

```bash
bbt net listeners --json
```

Emits the `bbt.net.listeners.v1` envelope. `data` contains:

- `listeners[]` — protocol (`tcp`/`tcp6`/`udp`/`udp6`), address, port, inode,
  pids, process names;
- `listener_count`;
- `warnings[]` — attribution or parsing limits.

Schema: [`schemas/net-listeners.v1.schema.json`](../../schemas/net-listeners.v1.schema.json).

## Agent mode

```bash
bbt net listeners --agent
```

Emits a `bbt.agent.report.v1` report with triage facts:

- `listener_count` and `socket_owner_mapping_available`;
- `externally_bound_listener_count` — listeners on `0.0.0.0` or `::`;
  `warning` severity when greater than zero;
- `privileged_listener_count` — listeners below port 1024; `warning` severity
  when greater than zero;
- `unattributed_listener_count` — listeners without visible process ownership.

## Notes and limitations

- Process attribution requires reading `/proc/<pid>/fd`; sockets owned by other
  users' processes appear unattributed unless BBT runs with wider privileges.
- UDP sockets in state `07` are included as listening-ish unconnected sockets.
- Namespaced processes (containers) may expose listeners the host view cannot
  attribute.
