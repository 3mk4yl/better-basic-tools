# Exit Codes

BBT is designed for humans, scripts, and agents. Structured observations should be parseable even when the observed target is missing or incomplete.

## Policy

| Situation | Exit code | Rationale |
|---|---:|---|
| Successful complete observation | `0` | Command ran and produced a valid observation. |
| Observation completed with warnings | `0` | Warnings are represented in structured output; consumers should inspect `warnings` and agent `severity`. |
| Target missing but represented in output | `0` | Missing paths, PIDs, or services are valid local facts, not CLI failures. |
| Invalid CLI usage | non-zero | The command could not be interpreted; no stable observation exists. |
| Internal/unexpected failure | non-zero | The tool could not produce a valid observation envelope. |

## Examples

Missing path in JSON mode returns success with `exists: false`:

```bash
bbt fs inspect /definitely/missing --json
```

Invalid CLI usage returns non-zero:

```bash
bbt fs inspect
```

## Consumer guidance

- Scripts and agents should treat exit code `0` as “valid BBT output was produced”.
- Scripts and agents should inspect structured fields such as `exists`, `warnings`, `severity`, and `facts` before deciding whether the observed system state is healthy.
- Human output is not a stable parsing contract; use `--json` or `--agent` for automation.
