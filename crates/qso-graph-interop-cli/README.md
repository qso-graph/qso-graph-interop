<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# qg-interop

Listen to WSJT-X UDP messages as QGI-SPEC-001 NDJSON events, decode a captured native message,
and send a synthetic exchange to a numeric destination.

Install from a checkout with `cargo install --path crates/qso-graph-interop-cli --locked`.

```sh
qg-interop listen --wsjtx
qg-interop decode --wsjtx capture.bin
qg-interop decode --wsjtx - < capture.bin
qg-interop sim --wsjtx --to 127.0.0.1:2237
```

The listener defaults to `127.0.0.1:2237`. Standard output contains one specification event per line;
errors and skip notices go to standard error. Each subcommand's `--help` includes examples.

Licensed under GPL-3.0-or-later.
