<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# `qg-interop`

Test amateur radio applications without a radio. `qg-interop` listens to WSJT-X UDP messages,
decodes captured datagrams as QGI-SPEC-001 NDJSON events, and simulates WSJT-X from ALL.TXT logs.
The single binary runs without Python or other runtime packages.

Install from a checkout with `cargo install --path crates/qso-graph-interop-cli --locked`.

```sh
qg-interop listen --wsjtx
qg-interop decode --wsjtx capture.bin
qg-interop sim --wsjtx --to 127.0.0.1:2237
qg-interop sim --wsjtx --to 127.0.0.1:2237 --file ALL.TXT --speed 60
qg-interop sim --capture capture.ndjson --to 127.0.0.1:2237
```

The simulator embeds a synthetic 120-decode sample, preserves cycle timing, and answers Replay.
Choose a numeric IPv4 or IPv6 destination with `--to`. Use `--call`, `--grid`, and `--id` to describe
the simulated station, and `--loop` to repeat. See the [simulator guide](../../docs/user/simulator.md)
for options and capture playback.

The listener defaults to `127.0.0.1:2237`. Standard output contains one specification event per line;
errors and skip notices go to standard error. Each subcommand's `--help` includes examples.

Licensed under GPL-3.0-or-later.
