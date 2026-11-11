<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- vale Google.Headings = NO -->
# QG Interop
<!-- vale Google.Headings = YES -->

**Read and write WSJT-X UDP messages from Rust, or watch them with `qg-interop`.**
QG Interop gives radio app developers a bidirectional codec and gives operators a
listener and a simulator for checking their UDP setup.

The library implements all 17 message types in WSJT-X 3.0.2 (and 2.7.0, whose protocol is the same), including logging, decoding,
radio status, and control. Applications get both a common message model and the complete original family payload.
The library leaves sockets and control decisions to its caller.

## Install

From this checkout, with [Rust](https://rustup.rs) installed:

```sh
cargo install --path crates/qso-graph-interop-cli --locked
```

## First use

Listen on the default local address, `127.0.0.1:2237`:

```sh
qg-interop listen --wsjtx
```

In another terminal, send a short simulated exchange to that listener:

```sh
qg-interop sim --wsjtx --to 127.0.0.1:2237
```

The listener displays messages like these:

```text
21:25:33  Heartbeat            WSJT-X  qg-interop  0.1.0  (schema 3)
21:25:33  Status               14.074000 MHz  FT8  RX  DE N0CALL FN31
21:25:33  Decode               -12 dB  0.2 s  1500 Hz  ~  CQ W1AW FN31
21:25:33  Close
```

The simulator sends a heartbeat, radio status, three decodes, a logged QSO, and shutdown messages.
Choose callsigns, mode, and frequency with `--call`, `--dx-call`, `--mode`, and `--frequency` (Hz).
Use `qg-interop listen --wsjtx --json` for one JSON object per decoded datagram. Add `--verbose` to show the sender, schema, and client ID. See the
[JSON Lines reference](docs/user/listen-json.md) for machine-readable output.
Each command's `--help` includes examples.

<!-- vale Google.EmDash = NO -->
<!-- vale Google.Headings = NO -->
<!-- vale Google.We = NO -->
## Security — Our #1 Priority
<!-- vale Google.We = YES -->
<!-- vale Google.Headings = YES -->
<!-- vale Google.EmDash = YES -->

- **Bounded input:** the decoder checks datagram and string lengths before copying them.
  Tests exercise oversized lengths, truncation, null values, and future schemas.
- **Safe Rust:** first-party code forbids unsafe Rust and defines no macros, including code behind
  inactive conditions. Dependencies come from crates.io, contain no native code, and pass
  cargo-vet, cargo-deny, and cargo-audit checks. Dependencies can use unsafe Rust and macros.
- **Explicit networking:** the library opens no sockets. The listener defaults to loopback;
  the simulator requires a numeric destination. Loopback integration tests exercise both commands.
- **Repeatable checks:** exact dependency requirements, lockfiles, dependency checks, and a
  decoder fuzz target are part of the repository's validation.

See [SECURITY.md](SECURITY.md) for the checks behind these properties and private reporting.

## Use the library

The `qso-graph-interop` crate exposes `wsjtx::encode`, `wsjtx::decode`, and `model::Message`.
The [library example](crates/qso-graph-interop/README.md) shows a complete round trip.
Protocol versions and source references are in [docs/PROTOCOL.md](docs/PROTOCOL.md).
Build and contribution instructions: [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[GPL-3.0-or-later](LICENSE).
