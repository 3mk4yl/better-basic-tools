# Release Policy

## Versioning

BBT follows semantic versioning with a pre-1.0 caveat: minor versions may change output schemas until v1.0.0.

- `v0.x.y-alpha.N` / `-beta.N` — prereleases; schemas may still change between them.
- `v0.x.y` — stable within the minor line; schema changes bump the minor version.
- `v1.0.0` — schema freeze: `bbt.*.v1` schemas become append-only (new optional fields allowed, no removals or type changes). Breaking output changes require `v2` schema identifiers.

## Quality gates

Every commit to `main` runs CI (`.github/workflows/ci.yml`):

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test --workspace --all-targets`
4. `cargo deny check` (advisories, licenses, bans, sources)
5. `scripts/validate_schemas.py` — live command output validated against every schema in `schemas/`

## Cutting a release

Releases are tag-driven (`.github/workflows/release.yml`):

```bash
git tag v0.2.0-alpha.2
git push origin v0.2.0-alpha.2
```

The workflow then:

1. re-runs the full quality gate (fmt, clippy, tests, cargo-deny);
2. builds release binaries for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`;
3. packages each as `bbt-<tag>-<target>.tar.gz` containing the binary, README, LICENSE, SECURITY.md, and the `schemas/` directory;
4. generates SHA-256 checksums;
5. publishes a GitHub release with auto-generated notes. Tags containing `-` (e.g. `-alpha.1`) are marked prerelease automatically.

## Verifying a download

```bash
sha256sum -c bbt-<tag>-<target>.tar.gz.sha256
```

## Support

Only the latest release and `main` receive fixes. See [`SECURITY.md`](../SECURITY.md) for vulnerability reporting.
