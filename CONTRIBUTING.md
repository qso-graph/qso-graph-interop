<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Contributing

Follow the [organization defaults](https://github.com/qso-graph/.github/blob/main/CONTRIBUTING.md)
and [repository rules](AGENTS.md). Keep protocol changes tied to a pinned upstream source and add
independent wire fixtures. New checks need planted cases that they reject.

Run every check from the repository root with one command:

```sh
cargo xtask verify
```

It runs the same checks as CI (formatting, Clippy, full and core-only tests, policy, fixtures,
the generated C header, FFI build, linked C test and C++ header compilation,
Miri for all Rust FFI tests, the C caller under ASan/UBSan, planted memory failures,
the specification vectors with `uv`, cargo-vet, cargo-deny, cargo-audit, and a 60-second fuzz
smoke on the pinned nightly), prints one pass/fail table, and exits non-zero if any check fails.
`--fuzz-seconds 0` skips fuzzing, and `--fuzz-seconds N` fuzzes longer. Each check's output is in
`target/verify/`. A missing tool fails its row with the command that installs it.

Fuzzing uses its own workspace and dated nightly. Its libFuzzer runtime is developer tooling,
never a dependency of the library, CLI, or FFI:

```sh
cargo install cargo-fuzz --version 0.13.2 --locked
python3 fuzz/seed.py
cd fuzz
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo fuzz run wsjtx -- -max_total_time=60 -max_len=65508 -timeout=10 -rss_limit_mb=2048 -print_final_stats=1
```

The target tests the decoder, canonical re-encoding, and lossless passage through the common
events, native re-encoding, arbitrary event JSON, and qgi UDP/TCP carriage. Reproduce a failure with `cargo fuzz run wsjtx <artifact>` and add a small regression test.

The FFI memory checks use the same `nightly-2026-10-30` as fuzzing. Install its
`miri` and `rust-src` components with rustup:

```sh
rustup component add --toolchain nightly-2026-10-30 miri rust-src
cargo xtask ffi-miri
cargo xtask ffi-miri-planted
cargo xtask ffi-sanitize
cargo xtask ffi-sanitize-planted
```

Miri runs every Rust FFI test, including threaded ownership, invalid arguments,
and decode/encode round trips through the C entry points. No test is excluded.
Isolation is disabled so the vector test can read repository files; memory,
aliasing, and strict provenance checks remain enabled. The independently compiled
C program runs outside Miri, under the sanitizers. ASan/UBSan requires Linux x86_64 GNU with a C compiler
and its sanitizer runtimes; other hosts skip it with a reason. Rust uses
`-Zsanitizer=address` and the C caller uses `-fsanitize=address,undefined` with
recovery disabled. Each build has a separate directory under `target/`.

The planted checks alone enable `qgi_planted_read`, which injects a one-byte heap
overread behind `qgi_abi_version`. Neither normal builds nor `--all-features`
enable it. Each check requires a nonzero exit and the tool's memory diagnosis;
compiler errors and unrelated crashes fail the check. Diagnostics are retained in
`target/verify/ffi-{miri,asan}-planted.txt`. Foreign input lengths must still name
live readable storage: the ABI can reject null, oversized, and wrapping ranges,
but cannot discover an allocation's actual size. Wrong-length tests use shorter
live slices, rejected oversized inputs, and modified owned-buffer descriptors.

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
