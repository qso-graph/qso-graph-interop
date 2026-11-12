<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex H: PSK Reporter IPFIX placeholder

Reserved family: `pskreporter-ipfix`. This is a scope placeholder, not a complete mapping or
conformance claim.

Submission sends the operator's callsign, location and reception data to a third-party service.
The caller chooses whether to submit (`QGI-GEN-0004`); codec support does not authorize disclosure.

## H.1 Published submitter rules

These facts were read on 2026-11-12 in PSK Reporter's
[developer page](https://pskreporter.info/pskdev.html), specifically Data Gathering, Data Reporting,
Technical IPFIX Information, Record Format Descriptor and Field Notes. They describe the service's
published expectations, not an independently measured acceptance policy.

| Topic | Published guidance |
|---|---|
| Interval | At most one datagram per five minutes unless full; avoid clock-synchronized timers and randomize timers based on last reception. |
| Duplicates | Report each callsign at most once per five minutes; preferably once per hour while unchanged. The developer defines “changed.” |
| Sender fields | Nonoptional: `senderCallsign`, `mode`, `informationSource`, `flowStartSeconds`. Optional: frequency, iMD, SNR, sender locator. |
| Receiver fields | Include receiver information each datagram. The list names callsign, locator, software and optional antenna information, but says to include as much as possible. |
| Templates | Send at least hourly; first three startup datagrams are recommended. Keep the UDP source port stable. |
| Source indicator | Field Notes requests `informationSource=1` for automatic reports; value 2 marks QSO records. |

Source for every row: [PSK Reporter developer information](https://pskreporter.info/pskdev.html).
The linked [FAQ](https://pskreporter.info/faq) rendered only a loading placeholder when read.
Unconfirmed: receiver-field acceptance minimum, exact duplicate key across modes/bands, and a
required change threshold. These are the unconfirmed policies referenced by `QGI-PSI-0002`.

| ID | Requirement | Verify |
|---|---|---|
| `QGI-PSI-0002` | **Responsible submission.** A producer of PSK Reporter IPFIX submissions MUST follow the published submitter rules sourced in H.1, including reporting intervals, duplicate suppression and required fields. It MUST preserve the source's distinctions between requirements and recommendations and MUST NOT invent constraints where H.1 marks the service policy unconfirmed. Scheduling and cross-message duplicate state belong to the submitting application, outside the pure codec. | Test, Inspection |

## H.2 Mapping work remaining

| Item | Scope to be completed |
|---|---|
| Native message | One complete IPFIX message in a UDP datagram. |
| Framing | Datagram plus IPFIX message/Set lengths, not one datagram per reception. The templated class uses `native.format: "ipfix_record"` and `QGI-CLS-0020`–`0032`; per-event `native.data` is one record, with full-message evidence retained separately. |
| Sending direction | Reception-report export toward PSK Reporter is the native sending direction. A possible outbound report encoder is not specified by this placeholder; no generic control operation is assigned. |
| Source to pin | [RFC 7011](https://www.rfc-editor.org/rfc/rfc7011.txt), September 2013, plus a dated, hashed copy of PSK Reporter's [developer/IPFIX definition and examples](https://pskreporter.info/pskdev.html), consulted 2026-11-12. |

**`QGI-PSI-0003` — Evidence and remaining profile gates.** The developer page describes separate receiver information and sender information records, enterprise
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

Radio parts have no established mapping in this placeholder (`QGI-MOD-0014`); their native
source and per-kind availability remain annex work. The family output limit and expansion proof
required by `QGI-MAP-0003` are also unestablished; no conformance is claimed for an assumed limit.
