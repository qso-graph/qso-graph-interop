<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- vale Google.Headings = NO -->
# QG Interop
<!-- vale Google.Headings = YES -->

QG Interop gives radio app developers Rust and C libraries for specification events and commands,
and gives operators the `qg-interop` listener, capture decoder, and simulator.

The library implements QGI-SPEC-001 0.1.0 for all 17 WSJT-X message types, schemas 1–3,
N1MM Logger+ XML, fldigi XML-RPC, and QGI event carriage over UDP and TCP.
Events preserve native evidence and distinguish observations from
commands. Applications supply observer context and own transport and control policy.

## Install

From this checkout with Rust 1.99.1 installed:

```sh
cargo install --path crates/qso-graph-interop-cli --locked
```

## First use

Listen on the default local address, `127.0.0.1:2237`:

```sh
qg-interop listen --wsjtx
```

In another terminal, send a short synthetic exchange to the listener:

```sh
qg-interop sim --wsjtx --to 127.0.0.1:2237
```

The listener emits one specification JSON event per line. Its events describe heartbeats, radio
state, decodes, logged QSOs, and shutdown messages. Each observation includes the complete native
datagram. Decode one captured native message from a file or standard input:

```sh
qg-interop decode --wsjtx capture.bin
qg-interop decode --wsjtx - < capture.bin
```

Listen to N1MM Logger+ broadcasts on `127.0.0.1:12060` with
`qg-interop listen --n1mm`, or decode a captured XML datagram with
`qg-interop decode --n1mm capture.xml`. See the [N1MM guide](docs/user/n1mm.md).

Poll local fldigi state and received text as specification NDJSON:

```sh
qg-interop poll --fldigi --interval 1000 --count 3
qg-interop decode --fldigi response.xml --request-method rig.get_frequency
```

The poller uses `127.0.0.1:7362` and accepts numeric loopback addresses.
See the [fldigi guide](docs/user/fldigi.md) for setup and library examples.

See the [event JSON guide](docs/user/listen-json.md). Each command's `--help` includes examples.

## Use the library

Install with `cargo add qso-graph-interop`. Default features select `wsjtx`, `n1mm`, `fldigi`, `qgi`, and `serde`.
Use `default-features = false` for the core without protocol families or enable `conformance` to test a
third-party codec. The public `Codec` trait decodes observations, encodes commands from common
fields into ordered requests with response metadata, and separately re-encodes native evidence.
Typed views expose common fields while preserving the complete event JSON. Stream input uses
caller-owned buffers with explicit progress; QGI event carriage has a separate encoding capability.

C and C++ applications can use the [C ABI](docs/C-ABI.md) as shared or static libraries.
Consumers need no Rust toolchain. The ABI also supports Python, .NET, and Node bindings.

See the [library example](crates/qso-graph-interop/README.md),
[protocol documentation](docs/PROTOCOL.md), and [contribution guide](CONTRIBUTING.md).

## Security

The library forbids unsafe Rust and opens no sockets. The command-line tool accepts numeric addresses and defaults
to loopback. Parser, field, context, and event limits are checked before output is published.
Shipped dependencies use compatible ranges and committed lockfiles, with cargo-vet, cargo-deny, and
cargo-audit checks. See [SECURITY.md](SECURITY.md) for private vulnerability reporting.

## License

[GPL-3.0-or-later](LICENSE).
