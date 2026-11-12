<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Dependencies

Every direct requirement uses a compatible range. Cargo.lock and fuzz/Cargo.lock pin resolution.
Only crates.io sources are allowed. The library uses serde and serde_json, plus optional sha2-const-stable for conformance. The CLI adds
clap. xtask adds TOML and Rust-token parsers without entering the shipped graph.
The isolated fuzz workspace adds libfuzzer-sys and serde_json JSON-path checks and never enters either shipped crate.

## Direct dependencies

| Crate | Range lower bound | License | Why |
| --- | --- | --- | --- |
| serde | 1.0.229 | MIT OR Apache-2.0 | Serialize native and neutral event models. |
| clap | 4.6.7 | MIT OR Apache-2.0 | CLI parsing and help, without its color feature. |
| serde_json | 1.0.145 | MIT OR Apache-2.0 | Event values, bounded JSON, NDJSON, fixtures, and metadata; float_roundtrip, arbitrary_precision and raw_value in the library. |
| toml | 1.1.8+spec-1.1.0 | MIT OR Apache-2.0 | Read manifests as TOML in xtask; no display feature. Requirement is `1.1.8`, which resolves this metadata-qualified release. |
| syn | 3.0.9 | MIT OR Apache-2.0 | Parse actual crate-root attributes in xtask. |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | Scan all Rust tokens, including disabled code, in xtask. |
| libfuzzer-sys | 0.4.13 | (MIT OR Apache-2.0) AND NCSA | Coverage-guided decoder fuzzing, developer tooling only. |
| sha2-const-stable | 0.1.0 | MIT OR Apache-2.0 | SHA-256 manifest hashes in optional conformance tooling; no dependencies or native code. |
| qso-graph-interop | 0.1.0 | GPL-3.0-or-later | Internal compatible-range path dependency of the CLI and fuzz workspace. |

## Workspace resolution

The full resolved list includes Cargo's inactive optional packages. cargo-vet covers this list:
4 fully audited packages (three imported audits and one local automated source review) and 24 exact-version exemptions. Exemptions mean **not audited
yet**, not reviewed or certified. Each exemption records its purpose in supply-chain/config.toml.
The configured audit sources are Bytecode Alliance, Google, Mozilla, and Zcash. The optional hash crate has the local source review recorded below. No trusted-publisher policies
were added. cargo-deny and cargo-audit are separate mechanical checks.

| Crate | Locked version | Declared license | Why | cargo-vet |
| --- | --- | --- | --- | --- |
| anstyle | 1.0.14 | MIT OR Apache-2.0 | Help style types pulled by clap_builder; color output is disabled. | Not audited yet; exact exemption |
| clap | 4.6.7 | MIT OR Apache-2.0 | CLI argument parsing and help; defaults off, no color feature. | Not audited yet; exact exemption |
| clap_builder | 4.6.7 | MIT OR Apache-2.0 | Builds command arguments and help. | Not audited yet; exact exemption |
| clap_derive | 4.6.7 | MIT OR Apache-2.0 | Generates the CLI parser at build time. | Not audited yet; exact exemption |
| clap_lex | 1.1.1 | MIT OR Apache-2.0 | Splits command-line argument tokens. | Not audited yet; exact exemption |
| equivalent | 1.0.3 | Apache-2.0 OR MIT | Optional TOML ordered-map support retained in resolution; not activated. | Not audited yet; exact exemption |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 | Optional TOML ordered-map support retained in resolution; not activated. | Not audited yet; exact exemption |
| heck | 0.5.0 | MIT OR Apache-2.0 | Case conversion used by clap_derive. | Imported audits |
| indexmap | 2.14.2 | Apache-2.0 OR MIT | Optional TOML ordered-map support retained in resolution; not activated. | Not audited yet; exact exemption |
| itoa | 1.0.18 | MIT OR Apache-2.0 | Formats JSON integers. | Not audited yet; exact exemption |
| memchr | 2.8.3 | Unlicense OR MIT | Searches bytes while parsing JSON. | Not audited yet; exact exemption |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | Rust token streams for derive macros and policy scans. | Not audited yet; exact exemption |
| quote | 1.0.47 | MIT OR Apache-2.0 | Builds Rust token streams in derive macros. | Imported audits |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | Formats JSON floating-point values. | Not audited yet; exact exemption |
| serde | 1.0.229 | MIT OR Apache-2.0 | Typed family and common-model serialization. | Not audited yet; exact exemption |
| serde_core | 1.0.229 | MIT OR Apache-2.0 | Shared serialization traits and implementation. | Not audited yet; exact exemption |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 | Generates model serialization at build time. | Not audited yet; exact exemption |
| serde_json | 1.0.145 | MIT OR Apache-2.0 | JSON lines in the CLI, fixtures in tests, metadata in xtask. | Not audited yet; exact exemption |
| serde_spanned | 1.1.3 | MIT OR Apache-2.0 | Manifest values with source spans in xtask. | Not audited yet; exact exemption |
| sha2-const-stable | 0.1.0 | MIT OR Apache-2.0 | Optional conformance manifest hashing. | Local automated source review |
| strsim | 0.11.1 | MIT | CLI typo suggestions. | Imported audits |
| syn | 3.0.9 | MIT OR Apache-2.0 | Parses Rust in derives and validates policy crate-root attributes. | Not audited yet; exact exemption |
| toml | 1.1.8+spec-1.1.0 | MIT OR Apache-2.0 | Parses manifests in xtask; std, serde, parse only. | Not audited yet; exact exemption |
| toml_datetime | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | TOML date/time values in xtask. | Not audited yet; exact exemption |
| toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | TOML syntax in xtask. | Not audited yet; exact exemption |
| toml_writer | 1.2.0+spec-1.1.0 | MIT OR Apache-2.0 | Optional TOML display support retained in resolution; not activated. | Not audited yet; exact exemption |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | Classifies Rust identifiers in parsers and tokenizers. | Not audited yet; exact exemption |
| winnow | 1.0.5 | MIT | Parser combinators for TOML. | Not audited yet; exact exemption |

## Additional fuzz-only resolution

These packages exist only in fuzz/Cargo.lock, besides the shared serialization dependencies above.
They are not covered by the shipped workspace's cargo-vet store or native-code ban. libFuzzer's
C++ runtime is the same isolated developer-tooling exception used by cargo-fuzz; project Rust
sources still forbid unsafe code. This exception does not apply to the library or CLI.

| Crate | Locked version | Declared license | Why |
| --- | --- | --- | --- |
| arbitrary | 1.5.0 | MIT OR Apache-2.0 | Fuzzer input helpers. |
| cc | 1.11.1 | MIT OR Apache-2.0 | Builds libFuzzer in the isolated fuzz workspace only. |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | Platform configuration in fuzz tooling. |
| find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 | Compiler discovery for the fuzz runtime build. |
| getrandom | 0.4.3 | MIT OR Apache-2.0 | Jobserver support on non-Unix platforms; fuzz tooling only. |
| jobserver | 0.1.35 | MIT OR Apache-2.0 | Coordinates fuzz runtime compilation. |
| libc | 0.2.191 | MIT OR Apache-2.0 | OS bindings used by the fuzz runtime build. |
| libfuzzer-sys | 0.4.13 | (MIT OR Apache-2.0) AND NCSA | Isolated libFuzzer C++ runtime for fuzzing only. |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later | EFI bindings in getrandom resolution; fuzz tooling only. |
| shlex | 2.0.1 | MIT OR Apache-2.0 | Compiler argument parsing for the fuzz runtime build. |

CI installs cargo-vet 0.10.3, cargo-deny 0.20.2, cargo-audit 0.22.2, and cargo-fuzz 0.13.2,
with their own `--locked` dependencies. These tools run during development and validation, never
from qg-interop. The Rust compilers are pinned separately for shipped code and fuzzing.

The optional `conformance` feature adds `sha2-const-stable` 0.1 for SHA-256 manifest verification.
It has no dependencies, build script, native code, or I/O. The local cargo-vet entry records an
explicit automated source review of all four source files and independent digest checks; it is
not an independent human audit. The normal codec and CLI dependency graphs omit this feature.
All manifests now use compatible ranges, with both committed lockfiles retaining resolved versions.
`serde_json` is also a library dependency for preserved event values and bounded event JSON;
`float_roundtrip` preserves binary64 and `arbitrary_precision` retains unknown exact numeric values.
