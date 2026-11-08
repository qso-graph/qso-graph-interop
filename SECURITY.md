<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Security

Report vulnerabilities using [private vulnerability reporting](https://github.com/qso-graph/qso-graph-udp/security/advisories/new).
Do not include private logs or credentials in a public issue.

`cargo xtask policy` checks crate-root forbids, inherited lints, edition 2024, source tokens even
under disabled cfgs, exact dependency requirements, crates.io sources, the pinned compiler, and
native code in the shipped dependency graph. It scans the library for socket/process APIs and the
CLI for TCP, name resolution, and process creation. Its tests plant violations of those rules.
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
