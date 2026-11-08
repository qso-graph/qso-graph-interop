<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Security

Report vulnerabilities using [private vulnerability reporting](https://github.com/qso-graph/qso-graph-udp/security/advisories/new).
Do not include private logs or credentials in a public issue.

Our own code forbids unsafe Rust and defines no macros, including code behind inactive
conditions. Dependencies may use unsafe Rust and define macros. Dependencies must come from
crates.io, contain no native code, and pass cargo-vet, cargo-deny, and cargo-audit checks.
Exemptions in cargo-vet are explicit review status, not completed audits.

`cargo xtask policy` checks crate-root forbids, inherited lints, edition 2024, all first-party
source tokens, exact dependency requirements, crates.io sources, and the pinned compiler.
It rejects native source/object files, foreign blocks, assembly, and link attributes in the
shipped dependency graph. Registry Rust unsafe code and macro definitions are allowed.

The library token scan rejects socket/process API names, including disabled code. CLI standard
library access is restricted to io, time, error, str, SocketAddr, and ExitCode paths; root aliases,
grouped standard-library imports, OS APIs, and socket type access are rejected. ExitCode is the
only allowed process path. This prevents process module aliases and alternate OS process paths.
All CLI UDP operations are in `numeric_udp.rs`, whose complete tokens must match the audited
policy template. Its bind and sender methods take SocketAddr, with no generic address traits,
string addresses, DNS, or exposed socket handles. Edits to that boundary require updating the
policy oracle. These are conservative source restrictions, not general Rust name resolution
or a proof about arbitrary third-party behavior; dependencies have separate supply-chain checks.
The direct dependency API set is also fixed: serde for the library, serde_json for its tests,
and clap, serde_json, and the codec for the CLI. Package renames, target-specific additions,
and new direct dependencies fail until this policy is updated after API review, so a wrapper
crate cannot silently introduce process or resolver APIs under an arbitrary name.
Policy self-tests plant aliases, implicit DNS, adapter changes, unreviewed dependency APIs,
and native/first-party violations.
Policy tooling itself may invoke build tools. Fuzzing is an isolated workspace; libFuzzer's C++
runtime and its build dependencies never enter the shipped dependency graph.

The decoder caps datagrams at 65,507 bytes and IDs at 256 bytes. Every array length is checked
against remaining input before allocation. Unknown types and newer schemas produce `Skipped`.
Known messages with incomplete fields return an error. Optional historical fields may end at a
field boundary; extra trailing data is retained without interpretation. Unit tests and fuzzing
exercise both variants. These checks reduce risk; they do not establish that every possible input
has been tested.

The CLI accepts numeric socket addresses only. It listens only on the chosen address, defaults to
`127.0.0.1:2237`, and does not answer incoming messages. The finite simulator sends only to `--to`.
The library implements controls but does not act on them or open sockets. UDP has no authentication;
applications decide which peers and commands they trust.

`cargo deny` checks sources, native build dependencies, and licenses. `cargo audit` checks the
lockfile against RustSec. `cargo vet` uses imported audits where available and records remaining
exact versions as explicit not-yet-audited exemptions in `supply-chain/config.toml`.
See [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) for dependency purposes and review status.

The first milestone's interoperability evidence is independent protocol vectors and reference
encoder agreement. It has not been tested against running WSJT-X or JTDX applications.

Human output escapes C0/C1 controls, bidi marks/embeddings/overrides/isolates (U+061C,
U+200E–U+200F, U+202A–U+202E, U+2066–U+2069), and U+2028/U+2029 separators. It also escapes
soft hyphen (U+00AD), zero-width space/joiners (U+200B–U+200D), word joiner and invisible
operators (U+2060–U+2064), reserved/deprecated format controls (U+2065, U+206A–U+206F),
BOM (U+FEFF), and interlinear annotation controls (U+FFF9–U+FFFB). These additional invisible
format controls can hide boundaries or alter presentation. Other printable Unicode is retained.
The library and JSON retain original text and raw invalid UTF-8 bytes.
