<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Listener JSON lines reference

Run `qg-udp listen --wsjtx --json` to emit one JSON object per decoded datagram.
Every object has `type`, `received` (RFC 3339 UTC), `from` (numeric socket address),
`schema`, and `client_id`. The remaining names match the protocol fields, in snake case.
Frequencies are integer Hz, and flags are booleans. Qt times use `HH:MM:SS.mmm`;
dates with a known UTC offset use RFC 3339. Invalid times and dates with an
unknown local or named time zone are `null`. Schema 1 dates use the UTC clock
stored on the wire. Null strings and absent optional fields are `null`; empty
strings remain empty. Invalid UTF-8 is replaced with U+FFFD and the accompanying
`<field>_bytes` array contains the complete original bytes. Nonfinite floating-point
values use `ieee754:` followed by their 16 hexadecimal bit digits. Colors are
objects with `spec` and `channels`.

For exact finite floating-point round trips, enable `float_roundtrip` in serde_json
readers. Other JSON readers must correctly round binary64 numbers. See the
[JSON float contract](../NORMALIZATION.md#json-float-preservation).

These examples show the type-specific fields; the common fields and other fields
are omitted for brevity. All fields of the decoded message are included in output.

```jsonl
{"type":"heartbeat","version":"2.7.0","max_schema":3}
{"type":"status","dial_frequency_hz":14074000,"mode":"FT8","transmitting":false}
{"type":"decode","time":"21:23:39.000","snr_db":-12,"message":"CQ W1AW FN31"}
{"type":"clear","window":null}
{"type":"reply","time":"21:23:39.000","snr_db":-12,"message":"CQ W1AW FN31"}
{"type":"qso_logged","time_off":"2026-11-08T21:23:39.000Z","dx_call":"W1AW","frequency_hz":14074000}
{"type":"close"}
{"type":"replay"}
{"type":"halt_tx","auto_tx_only":true}
{"type":"free_text","text":"CQ","send":false}
{"type":"wspr_decode","callsign":"W1AW","frequency_hz":14097000,"power_dbm":30}
{"type":"location","location":"FN31"}
{"type":"logged_adif","adif":"<CALL:4>W1AW<EOR>"}
{"type":"highlight_callsign","callsign":"W1AW","background":{"spec":0,"channels":[0,0,0,0,0]}}
{"type":"switch_configuration","configuration_name":"Default"}
{"type":"configure","mode":"FT8","frequency_tolerance_hz":50}
{"type":"annotation_info","dx_call":"W1AW","sort_order":1}
{"type":"jtdx_status","dial_frequency_hz":14074000,"mode":"FT8"}
{"type":"set_tx_delta_freq","tx_delta_frequency_hz":1500}
{"type":"trigger_cq","direction":"","send":true}
{"type":"skipped","message_type":null,"reason":"future_schema"}
```

A skipped datagram has `client_id: null`; `message_type` is null when the schema
is newer than the decoder supports. Its `reason` is `future_schema` or `unknown_type`.
