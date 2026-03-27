# Releasing rsgdb

Short checklist for maintainers. **Development** cuts use SemVer **pre-release** identifiers (e.g. `0.2.0-dev.1`) until we are ready for a stable **0.2.0** on crates.io.

## Before tagging

1. **Version** — set `version` in [`Cargo.toml`](Cargo.toml) (library and `rsgdb --version` follow it).
2. **Changelog** — add a dated section under [`CHANGELOG.md`](CHANGELOG.md); keep `[Unreleased]` for the next work.
3. **Lockfile** — [`Cargo.lock`](Cargo.lock) is tracked for **reproducible** builds; run `cargo build` or `cargo test` after changing dependencies so the lockfile updates, and commit it with the release.
4. **Validate like CI** — from the repo root:
   ```bash
   ./scripts/validate_local.sh
   ```
5. **Optional** — [`scripts/deps_check.sh`](scripts/deps_check.sh) before a wider announcement.

## Git tag

Use an annotated tag that matches the version (with `v` prefix):

```bash
git tag -a v0.2.0-dev.1 -m "Development release v0.2.0-dev.1"
git push origin v0.2.0-dev.1
```

GitHub **Releases** can attach notes from [`CHANGELOG.md`](CHANGELOG.md) for that version.

## crates.io (optional)

When publishing a stable (or pre-release) crate version:

```bash
cargo publish --dry-run
cargo publish
```

The README crates.io badge tracks the latest **published** version; it updates after a successful publish, not only on git tags.

## Compatibility

- **MSRV**: Rust **1.70** (see [CONTRIBUTING.md](CONTRIBUTING.md)); bumping MSRV should be called out in the changelog.
