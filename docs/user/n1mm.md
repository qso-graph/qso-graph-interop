<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Receive logger events

Use `qg-interop` to turn N1MM Logger+ XML datagrams into specification events for monitoring,
captures, and app development.

## Listen

Configure N1MM Logger+ UDP broadcasts to the listener's numeric IP address and port. Then run:

```sh
qg-interop listen --n1mm
```

The default is `127.0.0.1:12060`. N1MM's
[External UDP Broadcasts appendix](https://n1mmwp.hamdocs.com/appendices/external-udp-broadcasts/)
recommends port 12060 for broadcasts and lookup messages. The dated documentation and its hashes
are recorded in [Annex D](../../spec/annex-d-n1mm.md#d1-source-pin-and-evidence-boundaries).

To bind another numeric local address, supply it explicitly:

```sh
qg-interop listen --n1mm 127.0.0.1:12061
qg-interop listen --n1mm '[::1]:12060' --count 10
```

Standard output contains one compact specification JSON event per line. Errors and unknown-root
skip notices go to standard error. A malformed datagram does not end the listener. `--count` counts
received datagrams, including errors and skips; zero listens indefinitely.

As the observer on the broadcast port, the listener supplies `direction: "from_application"`.
The exchange remains `unknown`. The codec preserves that supplied context.

## Decode a capture

A capture file contains exactly one complete native XML UDP payload. Decode a file or standard input:

```sh
qg-interop decode --n1mm capture.xml
qg-interop decode --n1mm - < capture.xml
```

The decoder supplies unknown direction and exchange because the file does not establish either.
UTF-8 and UTF-16 XML are supported. Each observation retains the entire original datagram as
base64 in `native.data`. This evidence includes unknown children and attributes.

## Use the events

Contact additions, replacements, and deletions become `qso_logged`, `qso_changed`, and `qso_deleted`.
RadioInfo becomes `radio_state`, spots become `spot`, and dynamic results become `score`.
`AppInfo` and lookup messages use `native`; lookup timestamps remain available in `fields`.
Frequency and Spectrum inputs become observed `control` events.

Frequencies in common fields are integer hertz. Carried station/radio labels appear in `fields.radio`;
SO2R focus, active radio, running, stereo, and transmitting values stay separate. Cataloged valid
timestamps are accepted exactly as sent and represented as UTC instants. Missing or unconvertible
values leave their common fields absent. Empty text remains empty, and invalid text uses the
specification's raw-code-unit wrapper.

A spot's native `action` determines whether it adds or deletes a spot. A false transmitting value
reports N1MM's indication; external PTT or paddles can still transmit. See
[Annex D's mapping](../../spec/annex-d-n1mm.md#d4-complete-message-catalog) for field meanings.

The Rust library can construct `set_frequency` and `n1mm.spectrum` commands from fields. Both take
integer hertz and emit canonical decimal kHz; Spectrum takes an ordered array of levels and derives
the count. Encoding returns bytes to the caller. See the [library example](../../crates/qso-graph-interop/README.md).
