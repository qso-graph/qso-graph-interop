<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Event JSON guide

Run `qg-interop listen --wsjtx` to emit one QGI-SPEC-001 0.2.0 JSON event per line. The optional
`--json` flag selects the same output. Read one captured native WSJT-X message with
`qg-interop decode --wsjtx capture.bin`, or use `-` to read standard input. Each capture contains
one complete datagram payload, without transport headers.

Every event has `spec`, `family`, `mapping`, `kind`, `context`, and `fields`. WSJT-X observations
also have `native_version`, `wire_id`, and `native`. `listen` receives on a WSJT-X server port, where only WSJT-X
instances send, so it supplies direction `from_application`; `decode` reads a file whose
direction it cannot know, so it supplies `unknown`. Exchange is `unknown` in both. Neither infers identity from a sender's network address or wire ID. Skip notices and malformed
input diagnostics go to standard error and produce no event.

For example, `listen` reports WSJT-X closing like this:

```json
{"spec":"0.2.0","family":"wsjtx","mapping":1,"native_version":"3","kind":"lifecycle","wire_id":"test-instance","context":{"direction":"from_application","exchange":"unknown"},"fields":{"state":"closed"},"native":{"format":"wsjtx_datagram","data":"rbzL2gAAAAMAAAAGAAAADXRlc3QtaW5zdGFuY2U="}}
```

The same datagram through `decode` has unknown direction, so it is `ambiguous` with
`"fields":{"operation":"close"}`.

```text
qg-interop decode --wsjtx close.bin
```

Common frequencies use integer hertz. WSJT-X clock values use `HH:MM:SS.mmm`; resolved UTC dates
use RFC 3339 with three fractional digits and a final `Z`. Unresolved or invalid native times leave
the corresponding common field absent. Their original bytes remain in `native.data`.

A missing text field is omitted, native null is JSON `null`, and empty text is `""`. Invalid text
uses `{"utf8_bytes":[255]}` or `{"utf16_units":[55296]}` without replacement characters. Finite
binary64 values are numbers; nonfinite values use `ieee754:` and 16 lowercase hexadecimal digits.
Consumers must round floating-point numbers correctly. Object member order does not affect equality.

`fields.radio.instance` labels the radio instance carried by the family. It is never an authenticated
sender identity. Common fields do not extract callsigns or grids from decode text or parse ADIF.
Commands have `kind: "control"` and omit native evidence. Reading a command never sends it.

The library also carries events through `qgi`: reception appends one `via` hop with the receiving
observer's context while preserving the original event and evidence. Received context and hops
remain untrusted. See the [protocol reference](../PROTOCOL.md) for validation and limits.

## Receive text

Proposed WSJT-X type 19 uses `kind: "decode"`, `mapping: 3`. The sender Id is
`wire_id` and `fields.radio.instance`. `message_id` is signed decimal text to
preserve all 64 bits; `start` is the UTC transmission start, `audio_offset_hz` is
the reported receive offset, and `snr_db`, `text`, `complete`, and `off_air` carry
the remaining wire fields. No mode or callsign is inferred from the text.

Associate updates by `(wire_id, fields.message_id)` within your receiving session.
Each `text` replaces the previous whole text. `complete` marks a terminal update;
`off_air` identifies WAV decoding. Start follows the existing invalid-time rules.
Missing datagrams need no text concatenation. Null, empty, and raw text follow the
JSON rules in this guide; line breaks remain escaped within one NDJSON line.

Use `qg-interop listen --wsjtx --human` for readable summaries. For example:

```text
JTTYText id="test-instance" message="1" start="1970-01-01T01:00:00.123Z" df=1501Hz snr=-12dB complete=true off_air=false text="CQ Q1ABC"
```

Other events show their kind, Id, and common fields. `--human` and `--json` are
mutually exclusive; JSON remains the default and retains exact native evidence.
