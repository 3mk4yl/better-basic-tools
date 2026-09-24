#!/usr/bin/env python3
"""Validate live bbt JSON output against every schema in schemas/.

Requires: a built bbt binary (target/debug/bbt by default, override with
BBT_BIN) and the `jsonschema` package.
"""

import json
import os
import pathlib
import subprocess
import sys

import jsonschema

ROOT = pathlib.Path(__file__).resolve().parent.parent
BBT = os.environ.get("BBT_BIN", str(ROOT / "target" / "debug" / "bbt"))

CASES = {
    "fs-inspect.v1.schema.json": [BBT, "fs", "inspect", "README.md", "--json"],
    "fs-list.v1.schema.json": [BBT, "fs", "list", "docs", "--json"],
    "fs-tree.v1.schema.json": [BBT, "fs", "tree", "docs", "--depth", "2", "--json"],
    "disk-usage.v1.schema.json": [BBT, "disk", "usage", ".", "--depth", "1", "--json"],
    "disk-filesystems.v1.schema.json": [BBT, "disk", "filesystems", "--json"],
    "disk-pressure.v1.schema.json": [BBT, "disk", "pressure", "--json"],
    "host-snapshot.v1.schema.json": [BBT, "host", "snapshot", "--json"],
    "proc-inspect.v1.schema.json": [BBT, "proc", "inspect", str(os.getpid()), "--json"],
    "proc-list.v1.schema.json": [BBT, "proc", "list", "--json"],
    "net-listeners.v1.schema.json": [BBT, "net", "listeners", "--json"],
    "service-inspect.v1.schema.json": [
        BBT, "service", "inspect", "bbt-definitely-missing-test-service.service", "--json",
    ],
    "agent-report.v1.schema.json": [BBT, "proc", "list", "--agent"],
}


def main() -> int:
    schema_dir = ROOT / "schemas"
    known_schemas = {p.name for p in schema_dir.glob("*.schema.json")}
    missing_cases = known_schemas - set(CASES)
    if missing_cases:
        print(f"FAIL schemas without a validation case: {sorted(missing_cases)}")
        return 1

    failures = 0
    for schema_name, cmd in CASES.items():
        schema = json.loads((schema_dir / schema_name).read_text())
        try:
            output = subprocess.check_output(cmd, text=True, cwd=ROOT)
            data = json.loads(output)
            jsonschema.validate(data, schema)
        except Exception as error:  # noqa: BLE001 - report every failure kind
            print(f"FAIL {schema_name}: {error}")
            failures += 1
            continue
        print(f"PASS {schema_name}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
