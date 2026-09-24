# Output Contract

Better Basic Tools should treat structured output as a public API.

## Modes

- Human: `bbt disk usage /`
- JSON: `bbt disk usage / --json`
- YAML: `bbt disk usage / --yaml`
- Agent: `bbt disk usage / --agent`

## Schema envelope

JSON and YAML data responses use this common envelope:

```json
{
  "schema": "bbt.<domain>.<command>.v1",
  "generated_at": "2026-05-29T12:00:00Z",
  "command": {
    "argv": ["bbt", "disk", "usage", "/", "--json"],
    "argv_bytes_hex": ["626274", "6469736b", "7573616765", "2f", "2d2d6a736f6e"],
    "cwd": "/home/user/example",
    "effective_uid": 1000
  },
  "data": {}
}
```

## Agent report envelope

Agent mode should use a reasoning-oriented envelope:

```json
{
  "schema": "bbt.agent.report.v1",
  "generated_at": "2026-05-29T12:00:00Z",
  "subject": {
    "domain": "disk",
    "command": "usage",
    "target": "/"
  },
  "summary": "Root filesystem is 91% full.",
  "severity": "warning",
  "facts": [],
  "interpretation": [],
  "safe_next_steps": [],
  "risky_next_steps": [],
  "raw_ref": {
    "command": "bbt disk usage / --json"
  }
}
```

## Stability rules

1. Human output may evolve between minor versions.
2. Structured fields should not be renamed or removed within a schema version.
3. New optional fields may be added.
4. Breaking structured changes require a new schema version.
5. Units must be explicit: prefer bytes, milliseconds, percentages as numbers.
6. Timestamps should be ISO 8601 / RFC 3339.
7. Unknown/unavailable values should be `null`, not missing, when the field is
   part of the schema.
8. Evidence should point to observed facts, not vibes.
