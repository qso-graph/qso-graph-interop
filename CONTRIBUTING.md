<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Contributing

Follow the [organization defaults](https://github.com/qso-graph/.github/blob/main/CONTRIBUTING.md)
and [repository rules](AGENTS.md). Keep protocol changes tied to a pinned upstream source and add
independent wire fixtures. New checks need planted cases that they reject.

Run the checks from the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo xtask policy
python3 scripts/generate-fixtures.py --check
cargo deny --locked check bans licenses sources
cargo vet --locked
cargo audit --file Cargo.lock
```

Fuzzing uses its own workspace and dated nightly. Its libFuzzer runtime is developer tooling,
never a dependency of the library or CLI:

```sh
cargo install cargo-fuzz --version 0.13.2 --locked
python3 fuzz/seed.py
cd fuzz
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo fuzz run wsjtx -- -max_total_time=60 -max_len=65508 -timeout=10 -rss_limit_mb=2048 -print_final_stats=1
```

The target tests both variants, canonical re-encoding, and lossless passage through the common
model, plus packet and common-model JSON round trips. Reproduce a failure with `cargo fuzz run wsjtx <artifact>` and add a small regression test.

New source and documentation files carry an SPDX `GPL-3.0-or-later` header in their comment syntax.
Contributions use that license. Keep generated fixture changes reproducible with
`scripts/generate-fixtures.py`. Supply-chain exemptions name exact versions and explicitly mean
not audited yet; do not describe an exemption as an audit.

To verify the independent fixture oracle against a paired encoder/decoder field-order mutation:

```sh
uv run --no-project python scripts/check-field-order-mutation.py
```

This copies the workspace to a temporary directory, applies the adversarial review's Status
Rx/Tx swap, and requires the independent Status fixture to fail. It never edits the working tree.
