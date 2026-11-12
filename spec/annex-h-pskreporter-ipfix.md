<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex H: PSK Reporter IPFIX placeholder

Reserved family: `pskreporter-ipfix`. This is a scope placeholder, not a complete mapping or
conformance claim.

| Item | Scope to be completed |
|---|---|
| Native message | One complete IPFIX message in a UDP datagram. |
| Framing | Datagram plus IPFIX message/Set lengths, not one datagram per reception. The templated class uses `native.format: "ipfix_record"` and `QGI-CLS-0020`–`0032`; per-event `native.data` is one record, with full-message evidence retained separately. |
| Sending direction | Reception-report export toward PSK Reporter is the native sending direction. A possible outbound report encoder is not specified by this placeholder; no generic control operation is assigned. |
| Source to pin | [RFC 7011](https://www.rfc-editor.org/rfc/rfc7011.txt), September 2013, plus a dated, hashed copy of PSK Reporter's [developer/IPFIX definition and examples](https://pskreporter.info/pskdev.html), consulted 2026-11-12. |

The developer page describes separate receiver information and sender information records, enterprise
number 30351, template caching and optional template-defined attributes. Candidate `spot` fields
are Annex A's explicit receiver/sender fields, frequency, mode, SNR, time and reporting software.
Receiver metadata needs its own record event and evidence when used by a sender's reception event;
it cannot be reconstructed from sender bytes alone. [PSK Reporter IPFIX definition](https://pskreporter.info/pskdev.html)

Still to establish: a stable dated/hash pin; all supported information elements/templates; bounds,
expiry and reordering policy; Options Data Record kind/receiver association; unknown information
elements; variable-length fields, padding and signed numeric handling; and native-version behavior.
The page cites RFC 5101 and uses inconsistent `decoderSoftware`/`decodingSoftware` spellings. Annex H
must reconcile the examples with RFC 7011, not assume every example is a complete modern profile.
A receiver-only record is not an invented reception, and a template-only message is not a spot.
No report was sent to PSK Reporter and no server response was probed.
