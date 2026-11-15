<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex H: PSK Reporter IPFIX, mapping revision 1

This annex defines `pskreporter-ipfix` for specification 0.2.0: one RFC 7011 message
per UDP datagram. Submission exports station identity, location, and reception data.
The caller chooses the destination and authorizes disclosure (`QGI-GEN-0004`).
The codec opens no sockets, schedules no transmissions, and suppresses no duplicates.
Tests and examples use synthetic Q-prefixed callsigns and local collectors only.

## H.0 Source pins and authority

All sources were retrieved at **2026-11-14T17:28:27Z** and are not redistributed. Each
SHA-256 is of the file as retrieved from the location given; the WSJT-X archive pins are
also in Annex B.
Each source reference below incorporates this retrieval timestamp and its row's SHA-256.

| Source | Location and SHA-256 pin |
|---|---|
| S1 | <https://pskreporter.info/pskdev.html>; `9eab5ae2a01dcb840ac674ac694eb99d723a18dab685d939007d5755971af915`. Developer page, element table lines 106–122, reporting rules 80–96, 128–139, 333–334. |
| S2 | <https://www.rfc-editor.org/rfc/rfc7011.txt>; `59b7ddb1c0350ae711ba41d771d93f4d440ee04755403446f580bb777b928f73`. RFC 7011, September 2013; §§3–4, 7–9. |
| S3 | `Network/PSKReporterIPFIX.cpp` and `Network/PSKReporter.cpp` in the WSJT-X `v3.0.2` source archive; archive SHA-256 `8483a851bcb6455f5787de87b89efdf90e928842debde036e287e93e7f3837bd`. |
| S4 | `Network/PSKReporterIPFIX.cpp` and `Network/PSKReporter.cpp` in the WSJT-X `v3.3.0-beta6` source archive; archive SHA-256 `cbbbfd213de2dabfb0d2bac626a3b2ebb9e7b1836bd2771f90d12800771694be`. |
| S5 | `src/spot/pskrep.cxx`, `src/include/pskrep.h`, and `src/dialogs/fl_digi.cxx` in <https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz>; archive SHA-256 `a1e8d990359ce9c0cce3ceb5116fd0cf72c95528969766898640ca6ca2dba8d4` (Annex E). |

**`QGI-PSI-0010` — Authority.** RFC 7011 supersedes the developer page's RFC 5101
terminology. For the two program profiles, actual S3–S5 bytes govern conflicting
page examples. S1 calls enterprise element 8 `decoderSoftware` in its table and
`decodingSoftware` elsewhere; S3/S4 use the latter spelling, S5 the former.
Both mean PEN 30351, IE 8; neither spelling is transmitted. S1 also misspells
antenna information in prose; IE 9 identifies it. No sender ID selects an encoding.

S5 writes zero Scope Field Count in both receiver Options Templates (582, 593).
RFC 7011 §3.4.2 requires a nonzero scope. This mapping admits exactly S5's two
field definitions as a legacy exception, preserves zero, and diagnoses the departure.
It does not silently turn zero into one or admit arbitrary zero-scope templates.

## H.1 Published submitter rules

These are published expectations (S1), not measured service acceptance. Diagnostic
rule names below are stable. The collector reports violations, recommendations,
and notices separately; every diagnostic names a rule and, when known, Set/record indexes.

| Rule | Published expectation and observer behavior |
|---|---|
| `H.1-INTERVAL` | At most one datagram every five minutes unless full (S1:91). Timers must not synchronize to the system clock. If timed from last reception, randomize. A single observer can measure intervals, but cannot prove timer synchronization. The collector flags short intervals only for an exact known program layout when another maximum-sized profile record would fit: WSJT-X message at most 872 bytes; fldigi at most 678 bytes. Larger messages might meet the fullness exception; unknown layouts have no inferred fullness policy. |
| `H.1-DUPLICATE` | Each sender callsign at most once per five minutes (S1:82–84). Check the literal callsign per exporter/domain with monotonic observer time, including repeats in one message. Do not invent a mode/band key. |
| `H.1-UNCHANGED` | Prefer once an hour while unchanged (S1:83–84). What constitutes change is explicitly application-defined; the collector does not enforce a threshold. |
| `H.1-REQUIRED` | Sender callsign, mode, informationSource, flowStartSeconds are nonoptional; frequency, iMD, SNR, sender locator are optional (S1:131). Empty, missing, or uninterpretable required content is diagnosed per record. |
| `H.1-RECEIVER` | Include receiver information in every datagram (S1:128–129, 133). The receiver list names callsign, locator, software, and optional antenna, while requesting as much information as possible. Its acceptance minimum is unconfirmed; the collector does not invent required receiver members. |
| `H.1-TEMPLATES` | Repeat definitions at least hourly (S1:94, 139). Check each used definition against observations in this exporter/domain; unknown-template Sets are skipped with evidence. No claim is made about transmissions missed before collection began. |
| `H.1-STARTUP` | The first three datagrams with templates are recommended (S1:94, 163). The collector can report only its first three observed datagrams. Keep the UDP source port stable (S1:95); a single observer cannot infer that two different source ports belong to one exporter. |
| `H.1-SOURCE` | 1 means automatically extracted, 2 QSO, 3 other manual; bit 0x80 marks tests (S1:116, 333–334; S5:130, 776). Preserve the whole byte. Undefined source bits are diagnosed; QSO reports remain reception `spot` events with source 2. |
| `H.1-FIELDS` | Interpret only declared element encodings (H.2). Preserve malformed values and diagnose known content/width errors per record. Unknown elements have no guessed meaning. |
| `H.1-FRAMING` | RFC 7011 header/Set/record lengths and H.3 resource limits apply. Structural failures discard the whole message and commit no state; diagnostics retain known Set/record locations (S2 §9.1). |
| `H.1-PADDING` | Exporters must use zero padding shorter than a minimum record (S2 §3.3.1). Collectors accept nonzero padding and diagnose it, as §9.1 explicitly excludes it from malformed-message errors. |
| `H.1-SCOPE` | Nonzero Options scope required by S2 §3.4.2; the S5 exception above is reported as a departure. |
| `H.1-WITHDRAWAL` | UDP withdrawals must be ignored (S2 §8.4); diagnose, retaining the cached definition. |
| `H.1-ASSOCIATION` | More than one receiver record makes a sender's receiver association unavailable; do not choose an arbitrary receiver. This is an evidence notice, not a service acceptance rule. |

**`QGI-PSI-0002` — Responsible submission.** A producer MUST follow the published
rules above, preserving requirements versus recommendations. Scheduling and
cross-message duplicates belong to the application. Receiver acceptance minimum,
exact duplicate key across modes/bands, and required change threshold remain
unconfirmed and MUST NOT be enforced as invented service policy. No live service
probe is part of this mapping or its verification.

## H.2 Information elements and program templates

All multibyte integers are big endian. Strings preserve exact UTF-8 bytes, including
invalid sequences via `text.utf8_bytes`; empty is distinct from absent. IPFIX has no
implicit null string. Template length 65535 uses RFC 7011 §7: a one-byte length
below 255 or marker 255 plus a two-byte length. Length-prefix octets are excluded
from field content; the extended form can represent 0–65535 (S2:2050–2114).

| PEN.IE | Name | Encoding and common mapping | Evidence |
|---|---|---|---|
| 30351.1 | senderCallsign | string → `spot.sender_call` | S1:106; S3/S4:86–88, 172; S5:680, 766 |
| 30351.2 | receiverCallsign | string → `receiver_information.receiver_call` | S1:107; S3/S4:120–122, 146; S5:583, 644 |
| 30351.3 | senderLocator | string → `spot.sender_locator` | S1:108; S3/S4:98–100, 180; S5:684, 774 |
| 30351.4 | receiverLocator | string → `receiver_information.receiver_locator` | S1:109; S3/S4:123–125, 147; S5:584, 646 |
| 30351.5 | frequency | unsigned integer Hz, 1–8 bytes; WSJT-X five, fldigi four → `frequency_hz` when common-integer representable | S1:110, 196–198; S3/S4:89–91, 174–176; S5:682, 770 |
| 30351.6 | sNR | signed two's complement, 1–8 bytes; program profile one byte → `snr_db` | S1:111, 279; S3/S4:92–94, 177 |
| 30351.7 | iMD | signed two's complement, 1–8 bytes, normally one → `imd_db` | S1:112, 279 |
| 30351.8 | decoderSoftware / decodingSoftware | string → `reporting_software` | S1:113, 133; S3/S4:126–128, 148; S5:586, 650 |
| 30351.9 | antennaInformation | string → `antenna_information` | S1:114; S3/S4:129–131, 149; S5:587, 652 |
| 30351.10 | mode | string; ADIF MODE or SUBMODE → `mode` | S1:115; S3/S4:95–97, 179; S5:683, 772 |
| 30351.11 | informationSource | one-byte flags → `information_source`, including 1 versus 2 and test bit | S1:116; S3/S4:101–103, 182; S5:685, 776 |
| 30351.12 | persistentIdentifier | string/opaque bytes; fldigi fixed eight → `persistent_identifier`, no security interpretation | S1:117; S5:585, 648 |
| 30351.13 | rigInformation | string → `rig_information` | S1:119; S3/S4:132–134, 150 |
| 30351.14 | messageBits | raw bytes, surplus trailing bits preserved; no common mapping | S1:120 |
| 30351.15 | deltaT | signed int16 in 100 µs; 0x8000 is native unknown; preserve raw, no common mapping | S1:121 |
| 30351.16 | fractionalFrequency | one byte in 1/256 Hz, two in 1/65536 Hz; preserve raw, no guessed addition to frequency | S1:122 |
| 0.150 | flowStartSeconds | unsigned four-byte epoch seconds → UTC `time`, 1970 through u32 maximum; never observer receipt time | S1:118, 280; S3/S4:104–105, 183–189; S5:681, 768 |
| Other | Unknown element | Preserve enterprise, ID, declared length, field content, exact template and record; never interpret | S2 §9 |

Integer values beyond the common integer range remain in evidence without a scalar
common value. Repeated instances of one IE likewise have no guessed scalar value.
Known elements with invalid widths remain in evidence; the observer diagnoses them.

`v` below means 65535 in a template. Lists are in exact wire order.

| Profile | Set kind / Template ID | Ordered IE(length), enterprise 30351 unless prefixed 0. | Source |
|---|---|---|---|
| WSJT-X (both versions) | Template / 0x50e3 | 1(v), 5(5), 6(1), 10(v), 3(v), 11(1), 0.150(4) | S3/S4:73–108 |
| WSJT-X (both versions) | Options Template / 0x50e2, scope count 1 | 2(v), 4(v), 8(v), 9(v), 13(v) | S3/S4:109–137 |
| fldigi | Template / 0x022c | 1(v), 0.150(4), 5(4), 10(v), 3(v), 11(1) | S5:677–686 |
| fldigi long | Options Template / 0x0219, scope count 0 | 2(v), 4(v), 12(8), 8(v), 9(v) | S5:580–589 |
| fldigi short | Options Template / 0x0218, scope count 0 | 2(v), 4(v), 12(8) | S5:591–598 |

Both writers send sender template, receiver template, receiver data, then sender data,
with zero Set padding to four-byte boundaries (S3/S4:59–71, 234–292;
S5:688–780, 817–830). Headers contain version 10, complete length, export seconds,
sequence number, and observation domain. RFC sequence numbers count all data records;
S3/S4's split routine counts sender records only (270), and S5 increments per message
(849). Preserve supplied counters rather than silently correcting source evidence.
The encoder's counters are explicit caller inputs.

## H.3 Decoder state, evidence, and limits

**`QGI-PSI-0011` — Framing.** Check the complete 16-byte header, message length,
Set header and Set length, template specifiers and record fields before allocation.
Reserved Set IDs are malformed. Version 10 is implemented; older versions are
unsupported and later versions are skipped without interpreting their Sets.
Templates (Set 2), Options Templates (Set 3), and Data Sets (ID ≥256) are supported.
Process Sets in wire order; a later template cannot rescue an earlier unknown Set.
Zero-field template records, including all-template withdrawals, are ignored over UDP.
A residual suffix shorter than the minimum record is padding. If enough bytes remain
for a minimum record, they must parse as a record; a supposed padding suffix cannot
hide an overlong variable field. A zero-minimum-length template is rejected.

**`QGI-PSI-0012` — State.** Scope templates by caller session bytes, observation
domain, and template ID. The empty session key explicitly means this codec's one
anonymous exporter. Applications collecting multiple exporters MUST supply distinct
transport-session keys; the CLI uses the full numeric source socket address.
Identical definitions refresh; changed definitions replace. Overflow is `oversized`,
with no eviction. An explicit `DecodeOptions.expiry` discards cached definitions
whose last observed export time is less than that cutoff. There is no implicit
clock, timeout, reordering buffer, or sequence-based reconstruction. Reset one
session or all sessions. All state changes commit only after every event is admitted.
Caller resets/cutoffs resolve exporter restarts and stale UDP definitions; arrival
order alone cannot establish export order (S2 §§8.4, 9.3).

| Resource | Admission policy and evidence |
|---|---|
| Message | 60,000 bytes, below RFC's 65,535-byte header limit (S2 §10.3.3) and ordinary UDP's payload bound; covers the source writers' 952 and 1,454 bytes. This is a QGI resource policy, not a service MTU. |
| Field | At most 59,980 content bytes within one Data Set; lengths must fit actual remaining bytes. |
| Templates | 16 sessions, session key ≤128 bytes, 64 templates per session across domains; 64 fields per template, ≤518 template bytes (6 + 64×8). The programs use two live definitions per profile, three fldigi IDs in total; RFC permits IDs 256–65535, not unlimited cache storage. |
| Cache | At most 1,024 templates, 530,432 raw template bytes; fields and keys have the fixed count bounds above. No cached receiver records. |
| Dependencies | At most one receiver record and its ≤518-byte template per spot; receiver plus sender record bytes are disjoint portions of this message. |
| Events | At most 4,096 records and skips combined per message; a further outcome is an explicit error. Empty templates/messages yield no fabricated events. |

**`QGI-PSI-0003` — Record mapping and evidence.** A template containing IE 30351.1
identifies sender-information records, which become `spot`. A template containing
30351.2 without 30351.1 identifies `receiver_information`, a separate 0.2.0 catalog
kind. Other records become `native` with empty common fields. The IDs do not choose
the layout. Receiver fields on a spot come only from exactly one receiver record
in that same message, even when that receiver follows the spot. Zero or multiple
receiver records leave receiver fields absent. Never retain receiver values across
messages or reconstruct them from sender bytes.

Every record carries `native.format: "ipfix_record"`, exact record `data`, exact
Template Record `template` excluding its Set padding, complete `header`, `set_id`,
zero-based `set_index`/`record_index`, and exact enclosing `message_data`.
A joined spot carries `dependencies` with receiver data, template, header and Set ID
in the class shape (`QGI-CLS-0026`–`0028`, `0034`). Verification requires no session
cache: validate that evidence against the full message and compare mapped fields.
A record re-encodes to its exact record bytes; retrieving `message_data` returns
the original datagram, including unrelated Sets and padding. Unknown-template skips
carry Set index, template ID, header, opaque Set payload and full-message evidence;
no record boundaries are guessed. Structured `native.message` contains `options`,
`scope_fields`, and ordered `elements` with `enterprise`, `id`, `length`, base64 `data`.
There is no radio-part mapping in revision 1.

**`QGI-PSI-0013` — Output expansion proof (`QGI-MAP-0003`).** Let N ≤60,000 be the
message size. Sender and supporting receiver field content occupy disjoint message
bytes. The catalog maps each at most once: JSON text costs at most 6N escaped bytes
(raw byte alternatives cost at most 4N). Record plus dependency base64 costs at most
4N/3; full-message base64 at most 4N/3; the record's structured element data at most
4N/3, plus at most 172 bytes of rounding across 64 elements. Two templates, fixed
headers/indexes, element descriptors, field names, envelope and a maximum 4,096-byte
context fit a conservative 32,768-byte allowance. Thus the largest observation is
at most **10N + 32,768 = 632,768 bytes**, below 1,048,576. Element count is at most
N raw-byte entries + 2,048 context elements + 1,536 envelope/field/element nodes =
63,584, below 65,536. Both estimates overcount simultaneous expansions. Exact compact
output admission remains mandatory. The boundary test admits a 60,000-byte message
with a maximum-size raw-byte sender string, structured view and large context;
one byte beyond the message cap is rejected. This bound includes supporting evidence,
rather than treating only the IPFIX length as an observation limit.

## H.4 Encoding and local tools

**`QGI-PSI-0014` — Submission encoding.** `encode_submission` accepts one receiver
information event, zero or more spot events, and explicit profile/header/template
options. It validates all supplied fields, integer ranges, representable whole-second
UTC times, nonempty required text, callsign/locator syntax and mode token syntax;
unknown or unrepresentable fields are errors, never silently dropped. Mode tokens
are preserved; the caller chooses a registered ADIF MODE/SUBMODE. Strings are never
truncated. Every string emitted by the selected fixed layout must be explicitly present:
use an empty string for an unknown optional value. Missing and null are errors, rather
than silently becoming empty. Receiver text may be explicitly empty; this writer does
not invent the unconfirmed receiver acceptance minimum. WSJT-X limits in bytes are callsign 32, locator/mode 16, software 80,
antenna/rig 128 (S3/S4:16–21). fldigi uses maximum 254 text bytes and fixed eight-byte
persistent identifier (S5:239, 555–565). The short profile cannot carry software or
antenna. Receiver values asserted by a spot must agree with its supplied receiver.

The two layouts are H.2's exact templates, field orders, lengths, and padding.
WSJT-X payloads are at most 952 bytes (S3/S4:9–12); fldigi at most 1,454
(S5:121–122). An oversized submission fails; caller splitting must retain receiver
information and schedule descriptor repetition. Templates may be omitted explicitly
when the receiving session already has them. The fldigi writer rejects receiver-only
submissions because S5:809–815 does not send them; WSJT-X permits them (S3/S4:250–253).
There is no generic control operation.
This API does not grant permission to submit to an outside service.

`qg-interop collect --pskreporter-ipfix [ADDRESS]` defaults to `127.0.0.1:4739`,
accepts numeric unicast only, emits record NDJSON, and sends H.1 diagnostics to stderr.
Its state is bounded to 64 exporter/domain histories and 4,096 recent literal sender
calls per history, using monotonic caller time; overflow is explicit. It does not
synthesize policy about unconfirmed acceptance. Malformed content does not become
an invented reception. A malformed message discards all its records atomically.

`qg-interop sim --pskreporter-ipfix --to ADDR --profile wsjtx|fldigi` requires an
explicit numeric unicast destination. One submission is the default; `--loop`
uses a stable socket and startup-relative intervals of 301–317 seconds, repeats
templates every message, and cycles sixteen Q-prefixed callsigns so unchanged calls
are more than an hour apart. No service hostname or implicit remote destination
is supplied. The simulator exposes no rate acceleration for this family.

## H.5 Live WSJT-X confirmation (2026-11-15)

The byte-exact `wsjtx-3.0.2-ft8-ipfix-udp.ndjson` and
`wsjtx-3.3.0-beta6-ft8-ipfix-udp.ndjson` captures confirm S3 and S4 respectively.
[Provenance and SHA-256 pins](../docs/PROTOCOL.md#live-peer-captures-2026-11-1415)
describe the Windows 11 VM, normal operation with live ft8sim audio through a
virtual audio cable, and local redirection of PSK Reporter submissions. Neither
capture used WSJT-X's audio test mode. Receiver and sender identities are confined
to the authorized live fixtures; `ipfix-oracles.json` contains their literal values.
Receiver locator is DN13; antenna is explicitly empty and rig information is
`No CAT control` in both versions. Decoder software is exactly `WSJT-X v3.0.2`
and `WSJT-X v3.3.0-beta6 725545`, respectively.

Record numbers below are one-based NDJSON lines. All four submissions decode with
zero errors or skips, with the existing WSJT-X profile unchanged. Each first
submission contains Set 2 (60 bytes), Set 3 (52 bytes), receiver Set 0x50e2
(48 bytes for S3, 60 for S4), and sender Set 0x50e3 (84 bytes). The template
fields and widths match H.2 exactly, including receiver scope count 1 and five-byte
frequencies. Each first submission produces one receiver-information event and
three spots; the second produces one receiver-information event.

| Version | Submission records / bytes | Export UTC / sequence | Observation domain | Matching first grid-bearing Decode records |
|---|---|---|---|---|
| 3.0.2 (S3) | 82 / 260; 158 / 176 | 01:57:14 / 0; 01:59:15 / 3 | 3493175687 | 21, 23, 24 (Status 20) |
| 3.3.0-beta6 (S4) | 81 / 272; 154 / 188 | 02:03:21 / 0; 02:05:22 / 3 | 2439138539 | 29, 31, 32 (Status 28) |

All export dates are 2026-11-15. Each second submission repeats both template
Sets and the receiver Data Set byte-for-byte, with no sender Data Set. This is a
template refresh, not cache-only reuse; it decodes even in a fresh collector.
The source port and domain stay constant within each capture. Export times differ
by 121 seconds; recorder receipt intervals are 121.010 seconds (S3) and
121.011 seconds (S4). These match S3/S4 `PSKReporter.cpp`'s 121-second report timer
(lines 35, 167–169). Descriptor startup sends can contain receiver information
without spots (lines 283–302). This observed interval is shorter than H.1's published
five-minute rule; successful parsing does not establish service-policy compliance.
Sequence 3 after three sender records confirms the source counter behavior in H.2.

The spots use FT8 and informationSource 1 (automatic, test bit clear). In both
captures, the sender at FN42 is at 14,074,800 Hz; EM86 at 14,075,200 Hz; EM73 at
14,076,300 Hz. S3 orders them FN42, EM73, EM86 with SNR -13, -16, -15 dB; S4 orders
FN42, EM86, EM73 with SNR -12, -16, -16 dB. Each frequency is the matching
Status dial frequency (14,074,000 Hz) plus the Decode audio offset. The Decode mode
marker is `~`; Status names FT8. All three S3 spots have flowStartSeconds
1794707730 (`2026-11-15T01:55:30Z`); S4 has 1794708105 (`2026-11-15T02:01:45Z`).
These match Decode reception times, not export or recorder receipt times. Later
Decodes include changed SNR/audio offsets, but neither second submission contains
sender records. The captures establish this observation, not a general duplicate
suppression policy.

The surrounding UDP traffic also decodes without errors: S3 has 40 Decode,
142 Status, 24 Heartbeat, and one Close; S4 has 40 Decode, 149 Status,
24 Heartbeat, and no Close. S3 ends with Close at 02:00:46.697Z; its preceding
Heartbeat was at 02:00:44.976Z. S4 ends with Status at 02:06:59.391Z, without an
observed orderly shutdown. The local S4 `Network/MessageClient.cpp` sends Close
from `impl::closedown()` (lines 745–754), called by its destructor (98–104).
Close is a shutdown message, not a periodic message. The capture does not prove
whether Close was sent after recording ended, lost, or never sent; its absence
alone cannot establish a beta6 protocol difference or a decoding failure.

Offline tests check literal independent field oracles, same-capture correlations,
raw timestamps, both template definitions, second-message refresh, evidence
verification, and byte-exact WSJT-X submission encoding. Planted field drift,
truncation, and missing-template cases must fail verification or skip unknown Sets.
Live WSJT-X IPFIX evidence is complete for both versions under QGI-RI-0063.

## H.6 Live fldigi confirmation (2026-11-15)

The byte-exact `fldigi-4.2.13-ipfix.ndjson` capture confirms S5's long receiver
profile. The lab's Windows ham station ran receiving and transmitting fldigi
4.2.13 instances through a virtual audio cable. Both used BPSK31, dial
14,070,000 Hz, and audio carrier 1,500 Hz. Spot was on at the receiver. The
LF-normalized `fldigi-4.2.13-ipfix-tx.txt` preserves the six transmitted CQ lines,
their times, and the receiver's verbatim decoded text. [Provenance and hashes](../docs/PROTOCOL.md#live-peer-captures-2026-11-1415)
pin both artifacts. Literal authorized calls and independent fields are confined
to the live fixtures, including `fldigi-ipfix-oracle.json`.

The single 256-byte datagram arrived at UDP 127.0.0.1:4739 from
127.0.0.1:50422 at 02:55:15.884Z. Its IPFIX header has version 10, export time
1794711315 (`2026-11-15T02:55:15Z`), sequence 0, and observation domain 1210219340
(0x48227b4c). Sets in order are 2 (52 bytes), 3 (52), 0x0219 (64), and 0x022c
(72). The templates exactly match H.2: sender 0x022c with four-byte frequency,
long receiver 0x0219 with fixed eight-byte persistentIdentifier and zero scope.
The latter is H.0's diagnosed legacy exception, not a decode error. Short receiver
0x0218 does not occur in this datagram; its exact ID/layout coverage remains the
independent source-derived short-profile fixture. Options Template padding is two
zero bytes, receiver Data Set padding three, and sender Data Set padding one.

The receiver record contains the station's six-character locator, `Dipole`,
decoder software exactly `fldigi-4.2.13/windows32-ix86`, and persistentIdentifier
bytes `48 22 7b 4c 69 74 69 68`. The three sender records, in wire order, have
flowStartSeconds 1794711166, 1794711106, and 1794711046: respectively
02:52:46Z, 02:51:46Z, and 02:50:46Z on 2026-11-15. Each matches a different
transmitted call seven seconds after its first CQ line. Each call appears twice
in the six-line decoded text, but only once in the submission. The last transmitted
line is at 02:55:39Z, after this datagram; no later submission was captured.
All spots carry mode `PSK31`, empty sender locator, informationSource 1 (automatic,
test bit clear), and **frequency 0 Hz**. The codec preserves these zero bytes;
it does not substitute dial plus offset (14,071,500 Hz). S5 `pskrep::append`
(lines 412–413) sets frequency to zero when `pskrep_qrg` is disabled. The capture
does not independently establish that configuration flag's value.

The following are S5 sender behavior and setup observations, not additional
IPFIX or service requirements:

- `pskrep_check` requires nonempty call, locator, and antenna information;
  its locator expression requires six characters (`pskrep.cxx`:236, 245–266).
- Received text reaches `spot_recv` only with Spot on (`progStatus.spot_recv`;
  controlled by XML-RPC `spot.set_auto`). The receive path excludes control
  characters below space (`fl_digi.cxx`:9425–9426). Thus a newline alone cannot
  satisfy `PSKREP_RE`'s trailing `[^[:alnum:]]+$` (`pskrep.h`:33); a printable
  non-alphanumeric delimiter after the repeated call, such as the space before
  `PSE K`, lets the spotter match.
- `SEND_INTERVAL` is 300 seconds (`pskrep.cxx`:107–108, 334); one observed
  datagram cannot measure the interval. `DUP_INTERVAL` is 1800 seconds
  (110–113, 418–419), keyed by call and band, consistent with the single report
  per repeated call here. This is separate from H.1's five-minute published rule.

Offline conformance decodes all four records with zero errors or skips, verifies
their evidence, and re-encodes the complete submission byte for byte with the fldigi
profile. Independent fields, raw timestamps, templates, and transmitter text are
checked; planted frequency/mode/time/receiver drift, truncation, and missing-template
cases fail verification or skip unknown Sets. The existing codec handles the live
traffic unchanged. Live fldigi IPFIX evidence is done under QGI-RI-0063.
