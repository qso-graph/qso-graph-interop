<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Dependencies

Every direct requirement uses a compatible range. Cargo.lock and fuzz/Cargo.lock pin resolution.
Only crates.io sources are allowed. The library uses serde and serde_json, plus optional sha2-const-stable for conformance. The CLI adds
clap and the Rust-only signal-hook flag API. xtask adds TOML and Rust-token parsers without entering the shipped graph.
The isolated fuzz workspace adds libfuzzer-sys and serde_json JSON-path checks and never enters either shipped crate.

## N1MM XML decision

Annex D requires invalid UTF-8 bytes and unpaired UTF-16 units to survive in leaf text, while
invalid units in markup are errors. It also orders declaration syntax/version/encoding checks,
whole-document DOCTYPE/entity isolation, document-order resources, then mapping eligibility and
numeric conversion. Lexical names use original-encoding byte lengths; decoded text budgets count
valid UTF-8 lengths and original invalid-unit lengths after entity and line-ending processing.

Use the small bounded tokenizer in `src/n1mm/xml.rs`. A conventional XML parser that requires
well-formed decoded character data cannot meet the invalid-unit contract. Repairing or replacing
input before using such a parser would lose the units and change admission/error precedence;
wrapping a parser with a second lexical pass would duplicate the tokenizer. No crates.io XML crate
is added or claimed to meet these requirements without workarounds. The tokenizer has no resolver,
DTD processor, I/O, unsafe code, or macro definitions. Only predefined entities and legal numeric
character references expand. UTF-8 and UTF-16 retain their own units through processing.

The existing dependency graph and both lockfiles remain unchanged. The normal policy, cargo-vet,
cargo-deny, and cargo-audit checks still apply. Specification vectors plant each limit and XML
isolation failure; the isolated N1MM fuzz target exercises decoding, command construction, native
round trips, and strict JSON publication.

## Direct dependencies

| Crate | Range lower bound | License | Why |
| --- | --- | --- | --- |
| serde | 1.0.229 | MIT OR Apache-2.0 | Serialize native and neutral event models. |
| signal-hook | ~0.1.17 | MIT OR Apache-2.0 | Safe SIGINT/SIGTERM flags without native sources; old-line exception explained below. |
| clap | 4.6.7 | MIT OR Apache-2.0 | CLI parsing and help, without its color feature. |
| serde_json | 1.0.145 | MIT OR Apache-2.0 | Event values, bounded JSON, NDJSON, fixtures, and metadata; float_roundtrip, arbitrary_precision and raw_value in the library. |
| toml | 1.1.8+spec-1.1.0 | MIT OR Apache-2.0 | Read manifests as TOML in xtask; no display feature. Requirement is `1.1.8`, which resolves this metadata-qualified release. |
| syn | 3.0.9 | MIT OR Apache-2.0 | Parse actual crate-root attributes in xtask. |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | Scan all Rust tokens, including disabled code, in xtask. |
| libfuzzer-sys | 0.4.13 | (MIT OR Apache-2.0) AND NCSA | Coverage-guided decoder fuzzing, developer tooling only. |
| sha2-const-stable | 0.1.0 | MIT OR Apache-2.0 | SHA-256 manifest hashes in optional conformance tooling; no dependencies or native code. |
| qso-graph-interop | 0.1.0 | GPL-3.0-or-later | Internal compatible-range path dependency of the CLI and fuzz workspace. |

## Signal handler decision

Comparison checked November 12, 2026, against crates.io packages and an isolated Cargo
resolution. Keep `signal-hook = { version = "~0.1.17", default-features = false }` because both
current alternatives violate the repository rule that dependency packages contain no native
code. The rule includes packaged tests and disabled optional code, not just files compiled by
the selected features. This retains an old 2020 API line as a documented exception to the
preference for maintained releases; it does not describe that line as currently maintained.
No dependency, lockfile, policy allowance, or cargo-vet exemption is added by this decision.

Counts include the handler itself and unique normal/build packages, exclude development and
unselected optional dependencies, and include target-specific packages. The target union is
across Linux, macOS, and Windows; per-target columns use x86_64 GNU Linux, Apple Darwin, and
Windows MSVC. These are resolved graph counts, not each manifest's direct requirement count.

| Candidate and features | Target union | Linux | macOS | Windows | Packaged native source and decision |
| --- | ---: | ---: | ---: | ---: | --- |
| signal-hook 0.1.17, defaults off | 6 | 4 | 4 | 6 | 0 C/assembly files in its six-package graph; retain. |
| Current signal-hook 0.4.6, defaults off | 6 | 4 | 4 | 6 | 1 C file in signal-hook itself, `src/low_level/extract.c`; reject. |
| Current ctrlc 3.5.2, `termination` | 12 | 6 | 10 | 3 | ctrlc itself has no C; transitive nix 0.31.3 packages `test/test_kmod/hello_mod/hello.c`; reject. |

The retained graph is signal-hook 0.1.17, signal-hook-registry 1.4.8, libc 0.2.191, errno 0.3.15,
windows-sys 0.61.2, and windows-link 0.2.1. Current signal-hook has the same five transitive
packages: upgrading would add **zero** package identities, but its packaged C source fails
policy even with `extended-siginfo-raw` disabled. Its
[manifest](https://docs.rs/crate/signal-hook/0.4.6/source/Cargo.toml) includes C sources explicitly;
its [build script](https://docs.rs/crate/signal-hook/0.4.6/source/build.rs) only compiles
[extract.c](https://docs.rs/crate/signal-hook/0.4.6/source/src/low_level/extract.c) with that optional
feature on non-Windows targets. No C compilation is activated in this comparison.

The ctrlc graph is ctrlc 3.5.2, nix 0.31.3, bitflags 2.13.2, cfg-if 1.0.5, cfg_aliases 0.2.2,
libc 0.2.191, dispatch2 0.3.1, block2 0.6.2, objc2 0.6.5, objc2-encode 4.1.0,
windows-sys 0.61.2, and windows-link 0.2.1. Replacing the retained graph introduces **9** package
identities and removes **3**, a net increase of **6**. Its
[target requirements](https://docs.rs/crate/ctrlc/3.5.2/source/Cargo.toml) select nix on Unix,
dispatch2 on Apple targets, and windows-sys on Windows. The nix
[packaged C test](https://docs.rs/crate/nix/0.31.3/source/test/test_kmod/hello_mod/hello.c) is not
compiled as a dependency, but still violates the package-wide source rule. Apple dependencies
also add OS FFI/link declarations outside the existing exact binding allowances, including
[dispatch2's System/libdispatch link](https://docs.rs/crate/dispatch2/0.3.1/source/src/lib.rs).
Fewer packages favor retaining the current graph, but the native-source rule is decisive.

| Candidate | Windows behavior | Maintenance evidence |
| --- | --- | --- |
| signal-hook 0.1.17 | Limited CRT `signal`/`raise` support; handlers must be re-registered and have races. Not every console Ctrl+C becomes SIGINT; SIGTERM is not a Windows console event. | Historical 2020 line. Its [changelog](https://docs.rs/crate/signal-hook/0.1.17/source/CHANGELOG.md) permits newer registry versions, so the locked backend is 1.4.8; this does not bring all newer frontend fixes into 0.1.17. |
| signal-hook 0.4.6 | Still limited CRT support with documented re-registration races and Ctrl+C/SIGTERM limitations. | Current maintained line. Its [changelog](https://docs.rs/crate/signal-hook/0.4.6/source/CHANGELOG.md) includes recent fixes and static flag support. Prefer it if the package-wide C-source rule changes. |
| ctrlc 3.5.2 with `termination` | Uses Windows `SetConsoleCtrlHandler` for console events, a better match for Ctrl+C/Ctrl+Break. On Unix, `termination` adds SIGTERM and SIGHUP to SIGINT; it does not create Unix SIGTERM semantics on Windows. | Current release, dated February 10, 2026; the package labels maintenance passive. [Release and dependencies](https://docs.rs/crate/ctrlc/3.5.2), [handler documentation](https://docs.rs/ctrlc/3.5.2/ctrlc/), and [Windows implementation](https://docs.rs/crate/ctrlc/3.5.2/source/src/platform/windows/mod.rs). |

Windows limitations are documented by both the
[retained frontend](https://docs.rs/signal-hook/0.1.17/signal_hook/#portability) and the
[current frontend](https://docs.rs/signal-hook/0.4.6/signal_hook/#portability).
Unix SIGINT/SIGTERM cleanup has process tests; local verification does not establish Windows
console delivery behavior. Each of the retained graph's six exact-version cargo-vet exemptions
remains labeled **not audited yet**, with no audit claim based on this comparison.

## Workspace resolution

The full resolved list includes Cargo's inactive optional packages. cargo-vet covers this list:
4 fully audited packages (three imported audits and one local automated source review) and 30 exact-version exemptions. Exemptions mean **not audited
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

Signal registration adds the following Rust-only packages. OS FFI declarations in the exact
libc, errno, windows-sys, and windows-link versions are explicitly admitted by policy; native
sources and assembly remain forbidden. See [simulator validation](simulator-validation.md#dependency-boundary).

| Crate | Locked version | License | cargo-vet |
| --- | --- | --- | --- |
| signal-hook | 0.1.17 | MIT OR Apache-2.0 | Not audited yet; exact exemption |
| signal-hook-registry | 1.4.8 | MIT OR Apache-2.0 | Not audited yet; exact exemption |
| libc | 0.2.191 | MIT OR Apache-2.0 | Not audited yet; exact exemption |
| errno | 0.3.15 | MIT OR Apache-2.0 | Not audited yet; exact exemption |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 | Not audited yet; exact exemption |
| windows-link | 0.2.1 | MIT OR Apache-2.0 | Not audited yet; exact exemption |

The fuzz harness also links the CLI parsing library to exercise ALL.TXT, so its lockfile includes
the CLI's Rust dependencies. No developer tooling enters a shipped crate.

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
