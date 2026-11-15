<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- vale Google.Headings = NO -->
# Decode PSK Reporter MQTT captures
<!-- vale Google.Headings = YES -->

Use `qg-interop` to decode MQTT messages you already have into QGI events.
Each spot becomes a `spot` event. Each receiver publication becomes a
`receiver_information` event. Events include the original topic and payload bytes.

Run the decoder on a local file:

```sh
qg-interop decode --pskreporter-mqtt capture.json > events.ndjson
```

For standard input, use `-` as the filename:

```sh
qg-interop decode --pskreporter-mqtt - < capture.ndjson
```

## Input formats

A capture JSON file contains a `records` array. Each record has a topic and a
base64-encoded payload. For example, `e30=` represents the JSON object `{}`:

```json
{
  "records": [
    {
      "received_at": 1794677323.6476567,
      "topic": "pskr/rxinfo/Q1AAA",
      "retain": true,
      "qos": 0,
      "payload_b64": "e30="
    }
  ]
}
```

Alternatively, supply NDJSON with one complete record per line:

```json
{"topic":"pskr/rxinfo/Q1AAA","payload_b64":"e30=","retain":true,"qos":0}
{"topic":"pskr/rxinfo/Q2BBB","payload_b64":"e30="}
```

In this file format, omitted `retain` means false. Omitted `qos` stays absent.
Supply the captured retain flag when you need its delivered value. QoS accepts
0, 1, or 2. The decoder ignores capture metadata such as `received_at`.

## Output and validation

The decoder writes one JSON event per line. It preserves the exact payload in
`native.data` as base64 and the topic in `native.topic`. Long locators retain their
original characters. A null SNR stays in the native payload; the common integer
`snr_db` field is absent. Receiver fields whose meanings are not defined by the
pinned page stay in native data.

Topic and payload callsigns, band, mode, locators and DXCC must agree. In the
observed captures, each `/` in a payload callsign appears as `.` in its topic
segment. For example, payload `Q1AAA/P` agrees with topic `Q1AAA.P`. The decoder
requires this exact substitution and preserves the payload callsign. For receiver
information, it decodes topic periods to slashes in the common `receiver_call`
field, including when the payload call is absent or null. Literal periods in
payload callsigns are rejected because they would make the substitution ambiguous.
The service page does not document this observed rule.

Locator checks compare the first four characters without regard to case. A
callsign with a missing suffix, dropped slash, or misplaced period still fails
agreement. Mismatches stop decoding with a nonzero exit status. Output written
before the error remains available; check the exit status before processing it
as a complete capture.

JSON capture files can contain up to 1 MiB. NDJSON is streamed, with a maximum
of 32,768 bytes per line. Each payload can contain up to 16,384 bytes. Invalid
JSON, duplicate keys, unexpected field types, and values outside the documented
ranges are errors. See [Annex G](../../spec/annex-g-pskreporter-mqtt.md) for the
source pins, field mappings, and complete limits.

This command reads local files and standard input. It does not connect to a
broker or fetch receiver metadata.
