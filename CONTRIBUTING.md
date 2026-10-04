# Contribute to CadisWave

Read [architecture](docs/ARCHITECTURE.md), [hardware support](docs/hardware-support.md), and [localization](docs/localization.md).

## Boundaries

Keep protocol conversion in `cadiswave-core`.
Keep USB, files, services, and audio workers in `cadiswave-runtime`.
Keep GTK presentation in `cadiswave-desktop`.
Submit typed runtime commands from widgets.
Render confirmed observations separately from pending edits.
Preserve user labels, node names, device serials, and exact errors during translation.

Use Rust 1.98.1 and the checked-in lockfile.
Use Simplified Technical English for technical prose.
Retain original MIT notices.
Keep reference archives and private evidence outside Git.

## Checks

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo build --release --locked --workspace --bins
cargo build --release --locked -p cadiswave-runtime --examples
```

Run ignored GTK tests through `packaging/smoke-install.sh` in its private audio namespace.
Do not run integration fixtures against your daily audio session.
Verify changed hardware controls on the exact USB profile.
Retain and restore the previous hardware state.
Do not change phantom power during demonstrations or automatic tests.

## Project documentation

Keep English and Indonesian product facts consistent.
Read [repository discoverability](docs/discoverability.md) before changing the README or GitHub metadata.
Link hardware claims to verified profiles and test evidence.

## Commits

End every commit message with this trailer after a blank line:

```text
Co-Authored-By: CADIS <agent@cadis.digital>
```

Describe the behavior change and its verification in pull requests.
Report unsupported controls and physical test limits directly.
Do not add generator or model attribution.
