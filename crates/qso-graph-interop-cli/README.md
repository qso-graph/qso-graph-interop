<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# `qg-interop`

Test amateur radio applications without a radio. `qg-interop` listens to WSJT-X and N1MM Logger+ UDP messages,
polls local fldigi state and received text, decodes captured bodies as QGI-SPEC-001 NDJSON events, simulates WSJT-X from ALL.TXT logs,
generates N1MM contest traffic, and serves a local fldigi XML-RPC stand-in for controller developers.
The single binary runs without Python or other runtime packages.

Install from a checkout with `cargo install --path crates/qso-graph-interop-cli --locked`.

```sh
qg-interop listen --wsjtx
qg-interop listen --n1mm
qg-interop decode --n1mm capture.xml
qg-interop decode --wsjtx capture.bin
qg-interop decode --fldigi response.xml --request-method rig.get_frequency
qg-interop poll --fldigi --count 3
qg-interop sim --fldigi
qg-interop sim --wsjtx --to 127.0.0.1:2237
qg-interop sim --wsjtx --to 127.0.0.1:2237 --file ALL.TXT --speed 60
qg-interop sim --n1mm --to 127.0.0.1:12060 --radios 2 --rate 120 --speed 60
qg-interop sim --capture capture.ndjson --to 127.0.0.1:2237
```

The simulator embeds a synthetic 120-decode sample, preserves cycle timing, and answers Replay.
Choose a numeric IPv4 or IPv6 destination with `--to`. Use `--call`, `--grid`, and `--id` to describe
the simulated station, and `--loop` to repeat. See the [simulator guide](../../docs/user/simulator.md)
for options and capture playback.

The N1MM simulator sends synthetic contacts, edits, deletions, spots, and matching scores.
Use `--radios 2` for SO2R, `--call` and `--station` for station identity, `--rate` for contacts per
simulated hour, and `--speed` for playback speed. `--seed` reproduces traffic, `--count` limits
new contacts, and `--loop` continues until interrupted. A frequency command from the configured
receiver updates the selected radio's next scheduled report.

The listener defaults to `127.0.0.1:2237` for WSJT-X and `127.0.0.1:12060` for N1MM.
See the [N1MM guide](../../docs/user/n1mm.md) for logger setup and capture decoding. Standard output contains one specification event per line;
errors and skip notices go to standard error. Each subcommand's `--help` includes examples.

The fldigi poller defaults to `127.0.0.1:7362` and accepts numeric loopback addresses.
Use `--interval` for the delay between cycles and `--count` for the number of cycles.
See the [fldigi guide](../../docs/user/fldigi.md) for XML-RPC setup and response association.

Run `qg-interop sim --fldigi` to test loggers, contest tools, macros, and the poller without
running fldigi. It serves all 50 mapped Annex E methods at `127.0.0.1:7362`, with settable
frequency, carrier, mode, contact call and locator, queued transmission, and synthetic received
text. Use `--speed`, `--seed`, `--call`, and `--grid` to choose the script settings.
See the [fldigi simulator guide](../../docs/user/simulator.md#test-fldigi-controllers) for first use.

Licensed under GPL-3.0-or-later.
