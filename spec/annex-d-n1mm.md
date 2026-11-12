<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex D: N1MM Logger+, mapping revision 1

This annex is normative for family `n1mm`, specification `0.1.0`. It specifies a
reader and a command writer, not a claim of testing with a running logger.

## D.1 Source pin and evidence boundaries

All three documents were retrieved on **2026-11-12 UTC**. Hashes cover the exact
response bodies, before newline conversion, HTML extraction, or XML parsing.
No upstream document is distributed with these vectors.

| Reference | Published document | SHA-256 |
|---|---|---|
| S1 | [External UDP Broadcasts appendix](https://n1mmwp.hamdocs.com/appendices/external-udp-broadcasts/) | `4bd29b8de620b867d8573d2f0dbff78b7af1f50d64f40804d809b4c281bafab9` |
| S2 | [XML examples download page](https://n1mmwp.hamdocs.com/mmfiles/udp_external_udp_broadcasts-xml/), marked Updated **2019-09-30** | `784c869b8a7d56ca57e6bb8d6c0308e36ebf6caed44ed28fd6b4c8619c6a5d51` |
| S3 | [UDP_External_UDP_Broadcasts.xml](https://n1mmwp.hamdocs.com/mmfile/get/file/UDP_External_UDP_Broadcasts.xml), obtained with S2's download form | `58e0d335c325d1fde94a285b581fdaa4d75965b1aaa564e40969dcbf7eb8400a` |

S1 names no program build or protocol revision. Its example dates are sample
content, not a release pin. S3 is a collection of snippets, not one well-formed XML
document; its first declaration spells `1 .0`. S1's contact sample has an unclosed
`run1run2`, and some sample timestamps contain extra spaces or letters instead of
digits. These are evidence ambiguities, not instructions to repair received bytes.
S1 section names in the tables below identify the evidence for every native name,
layout, direction, description, and documented unit. **Reader policies** and
**writer policies** in this annex are QGI decisions where those sources do not
supply a grammar, requiredness rule, or unit; they are not additional N1MM facts.

The kHz unit for spot, frequency-command and Spectrum scope frequencies is
**derived from the documented examples**, rather than an explicit unit label:
S1 **Spot Data** gives `frequency` 7061.2; S1 **Setting N1MM's
Frequency** gives 14053.284; S1 **Sending Spectrum Data** (also S3 Spectrum)
gives LowScopeFrequency 14000 and HighScopeFrequency 14055. In kHz these are
7.0612 MHz, 14.053284 MHz and 14.000–14.055 MHz, respectively, all amateur-band
frequencies; interpreting the values as Hz or MHz puts none in those bands.
S1 **Contact notes** explicitly states 10 Hz units for contact and radio
frequencies. The spot and command examples occupy the same bands at one
thousandth of the corresponding Hz values. This derivation is to be confirmed by
the live captures required by `QGI-RI-0063` when they exist; no such confirmation
is claimed here. Values below 1,000 kHz use the same unit, including 475.0 kHz
on 630 m and 137.5 kHz on 2200 m; magnitude MUST NOT select MHz instead.

## D.2 Message boundary, version and outcomes

| ID | Requirement | Verify |
|---|---|---|
| `QGI-N1M-0010` | A native message MUST be one complete UDP payload containing one XML document, with no line framing; evidence MUST use `native.format: "n1mm_xml"` and the entire payload in `native.data` (`QGI-CLS-0001`; S1, XML Schema and Message Field Lists). | Test |
| `QGI-N1M-0011` | A reader MUST identify a type by the exact case-sensitive, unqualified root name in D.4, never by `app`, a station label, or a descendant (S1 message samples; S3 historical samples). | Test |
| `QGI-N1M-0012` | An observation MUST omit `native_version`: neither the XML declaration's XML version nor `app` is a declared N1MM protocol version (S1 and S3 samples). | Test |
| `QGI-N1M-0013` | An observation MUST omit `wire_id`: a contact `ID` maps to `record_id`, not a sender/target ID (S1, Contact notes). | Test |
| `QGI-N1M-0014` | A reader MUST apply the outcome table below, after the isolation and syntax checks in D.3, producing exactly one event for a known root or one skip for an unknown root. | Test |
| `QGI-N1M-0015` | Every successful observation MUST use `family: "n1mm"`, `mapping: 1`, and the caller's unchanged context; documented directions in D.4 MUST NOT overwrite that context. | Test |

| Input | Outcome under QGI-N1M-0014 |
|---|---|
| Known root and known children | Map D.4; absent children stay absent. |
| Known root and unknown child/attribute | Map known, unambiguous values; preserve everything in native bytes. No skip. |
| Unknown root, including a namespaced root or a case variation | `unknown_type` skip with `type` equal to the literal qualified root name; omit `native_version`. |
| A newer program emitting a known root | Same mapping, without assuming a schema increment or a build from `app`. |
| A newer program emitting an unknown root | `unknown_type`; there is no N1MM `future_version` discriminator. |
| XML declaration version other than 1.0 | `unsupported_version` (XML syntax support, not a native schema). |
| Unsupported encoding declaration | `unsupported_version`; no guessed decoding. |
| Invalid XML structure, including multiple roots | `malformed`, no partial event. |
| Empty payload, or syntactically unfinished XML at end of payload | `truncated`, no partial event. |
| Resource bound exceeded | `oversized`, no partial event. |

A skip contains the readable type but no event/native envelope. As with Annex B,
applications that archive skipped payloads do so outside this outcome. Mapping
revision compatibility follows `QGI-MAP-0015`: a known root (with no declared native version) and a supported XML
encoding permits verification of recognized common fields; an unknown root makes
verification unavailable even if a stored event claims a known kind or operation.

## D.3 XML reader policies

These policies implement `QGI-SEC-0006` and `QGI-SEC-0007`. S1/S3 show declarations
and ordinary elements; they do not define the limits or error handling below.

| ID | Requirement | Verify |
|---|---|---|
| `QGI-N1M-0016` | A codec MUST reject a payload exceeding **65,507 bytes** before decoding or allocating XML structures. | Test |
| `QGI-N1M-0017` | A reader MUST reject any actual DOCTYPE or entity declaration as `malformed` before entity expansion, including internal, external and parameter entities; it MUST perform no external resolution. Literal declaration-like text inside comments or CDATA is text, not a declaration. | Test |
| `QGI-N1M-0018` | A reader MUST limit element depth to **16** (root depth 1), total elements to **1,024** (including root and unknown descendants), attributes to **16 per element** and **256 per document**, and each XML name to **128 encoded bytes**, reporting `oversized` before exceeding any bound. Namespace declarations count as attributes. | Test |
| `QGI-N1M-0019` | A reader MUST limit each numeric token it interprets to **128 characters** after removing XML whitespace at its ends, and total decoded text plus attribute-value UTF-8 length (or original invalid code-unit byte length) to **65,507 bytes**, reporting `oversized` before conversion/allocation exceeds these bounds. Unknown text is not a numeric token. | Test |
| `QGI-N1M-0020` | A reader MUST detect UTF-8 (default, optional BOM) or UTF-16LE/BE (BOM or initial `3c 00 3f 00` / `00 3c 00 3f` signature); a declaration, when present, MUST agree with that detection, with `UTF-16` allowed only for detected UTF-16; encoding labels are case-insensitive. A disagreement MUST be `malformed`. | Test |
| `QGI-N1M-0021` | A reader MUST accept XML 1.0 with or without a declaration; it MUST reject a structurally invalid declaration or forbidden XML character as `malformed`, without repairing sample typos. | Test |
| `QGI-N1M-0022` | For text content invalid in the detected encoding, a reader MUST preserve the whole affected leaf's decoded content as `utf8_bytes` or `utf16_units` under `QGI-SER-0002`, while still validating markup; malformed code units in markup, names, declarations or attributes MUST be `malformed`. | Test |
| `QGI-N1M-0023` | A reader MUST concatenate a leaf's text and CDATA content, expand only the five predefined XML entities and numeric character references to legal XML characters, and apply XML 1.0 line-ending normalization (CRLF and CR to LF); no trimming or Unicode normalization of text common fields is permitted. | Test |
| `QGI-N1M-0024` | A reader MUST treat a missing leaf as absent and either empty-element spelling as empty text; it MUST NOT infer null, defaults, UTC, radio numbers or boolean false from emptiness or absence. | Test |
| `QGI-N1M-0025` | A reader MUST preserve attributes, comments, processing instructions, unknown children, order, lexical spelling and whitespace in native bytes, without making them common fields except D.4's score selectors; attributes MUST NOT stand in for child elements. Attribute values use XML 1.0 normalization (literal tab/CR/LF become spaces, references are decoded); native bytes keep their lexical spelling. | Test |
| `QGI-N1M-0026` | A reader MUST leave a common member absent when its singleton native leaf is repeated or contains child elements, or its scalar cannot be converted by D.3; it MUST NOT select the first or last duplicate. | Test |
| `QGI-N1M-0027` | A reader MUST parse mapped integers as optional sign plus ASCII decimal digits, with only surrounding XML whitespace ignored, omitting results outside the common integer range; a mapped 10 Hz frequency MUST multiply exactly by 10 and omit a result outside that range. | Test |
| `QGI-N1M-0028` | A reader MUST accept exactly `True`, `true`, `1` as true and `False`, `false`, `0` as false, ignoring only surrounding XML whitespace for that conversion; any other or invalid-text spelling MUST leave the boolean common member absent. | Test |
| `QGI-N1M-0029` | A reader MUST leave unzoned timestamps native-only: S1/S3 do not establish UTC, and receipt time or local timezone MUST NOT create an instant or time of day. | Test |

XML whitespace here means space, tab, CR and LF. For invalid leaf encoding, parse
markup independently of character decoding: XML delimiters still delimit content;
retain the original offending code units and the valid decoded segments encoded in
the same encoding after D.3's entity/line-ending processing. A UTF-16 byte count not
divisible by two is a structurally incomplete code unit (`truncated`). Unpaired
surrogates in leaf text use the UTF-16 wrapper. Valid UTF-8 containing a forbidden
XML character (for example NUL) is an XML error, not invalid UTF-8. Unknown entities,
unbound prefixes, duplicate attributes, mismatched tags and text outside the root
other than XML whitespace are `malformed`. Comments and processing instructions do
not contribute elements; their encoded bytes still count toward the datagram bound.
The text-expansion bound counts all text nodes including interelement whitespace.
Size precedes encoding detection, declaration/isolation, XML structure/resource
checks, root dispatch, then scalar mapping. A reader checks the whole bounded XML
document even for an unknown root. EOF within a tag, attribute, entity reference,
comment, CDATA, or unclosed element is `truncated`; a conflicting closing tag is
`malformed`. A malformed complete scalar is preserved, not a message error.

## D.4 Complete message catalog

**`QGI-N1M-0030` — Catalog mapping.** A reader MUST use the element catalogs below.
Every listed observation leaf is **O**: optional to this reader, zero or one for
mapping. S1/S3 give examples, not a required/optional schema. Duplicate handling is
D.3. Types in these tables describe QGI conversion, not an undocumented N1MM XSD.
`text` includes empty/invalid text; `int` and `bool` use D.3; decimal kHz uses
`QGI-N1M-0036`. “Native only” is retained by `native.data` and the
optional D.6 view. No table implies fields absent from the payload.

| Root | Documented native direction | Kind / operation | Evidence |
|---|---|---|---|
| `AppInfo` | from application | `native` | S1 Application Info; S3 AppInfo |
| `contactinfo` | from application | `qso_logged` | S1 Contact Info; S3 contactinfo |
| `contactreplace` | from application | `qso_changed` | S1 Contact Replace; S3 contactreplace |
| `contactdelete` | from application | `qso_deleted` | S1 Contact Delete; S3 contactdelete |
| `lookupinfo` | from application | `native` | S1 LookupInfo |
| `lookup` | from application (historical example) | `native` | S3 lookup; no assumption it is emitted by current builds |
| `RadioInfo` | from application | `radio_state` | S1 Radio Info; S3 RadioInfo |
| `spot` | from application | `spot` (including deletion) | S1 Spot Data; S3 spot |
| `dynamicresults` | from application | `score` | S1 Score Reporting |
| `Spectrum` | to application | `control`, `n1mm.spectrum` | S1 Sending Spectrum Data; S3 Spectrum |
| `radio_setfrequency` | to application | `control`, `set_frequency` | S1 Setting N1MM's Frequency |

S1's Rotator Control section mentions messages to the separate rotor program on
port 12040 but supplies **no root, fields, or encoding**. There is no invented
rotator type or writer here; any such XML root follows the unknown-root rule.
S1 names spectrum and frequency inputs on port 13064 and recommends 12060 for
broadcasts/lookup. Those are application configuration facts, not codec sockets.
No other accepted input layout is defined by S1/S3.

### AppInfo

Evidence for every row: **S1 Application Info; S3 AppInfo** (S3 omits `app`/`mycall`).
These describe a contest instance, not a logged QSO; all remain native-only.

| Element | Type / unit | Requiredness | Common mapping |
|---|---|---|---|
| `app` | text / — | O | native only |
| `dbname` | text / database label | O | native only |
| `contestnr` | integer text / database-local contest instance number | O | native only |
| `contestname` | text / — | O | native only |
| `StationName` | text / computer label | O | native only |
| `mycall` | text / callsign | O | native only |

### contactinfo, contactreplace, lookupinfo and historical lookup

Evidence for every row unless marked otherwise: **S1 Contact Info and notes**;
S1 Contact Replace and LookupInfo explicitly share that layout. **S3 contactinfo /
lookup** supplies the additional historical spellings. For lookup roots all entries
are native-only: a prospective contact is not a logged or changed QSO. The common
column applies only to the two contact roots.

| Element | Type / unit | Requiredness | Common mapping |
|---|---|---|---|
| `app` | text / — | O | native only |
| `contestname` | text / — | O | `contest_name` |
| `contestnr` | integer text / local contest instance | O | native only |
| `timestamp` | text / unresolved timestamp | O | native only (D.3) |
| `mycall` | text / callsign | O | `station_call` |
| `band` | text / native band label, localized delimiter | O | `band`, unchanged |
| `rxfreq` | int / 10 Hz | O | `dial_frequency_hz` = value × 10 |
| `txfreq` | int / 10 Hz | O | `tx_frequency_hz` = value × 10 |
| `operator` | text / callsign | O | `operator_call` |
| `mode` | text / — | O | `mode` |
| `call` | text / callsign | O | `call` |
| `countryprefix` | text / — | O | native only |
| `wpxprefix` | text / — | O | native only |
| `stationprefix` | text / — | O | native only |
| `continent` | text / — | O | native only |
| `snt` | text / signal report | O | `report_sent` |
| `sntnr` | text / serial exchange | O | `serial_sent`, keep leading zeros |
| `rcv` | text / signal report | O | `report_received` |
| `rcvnr` | text / serial exchange | O | `serial_received`, keep leading zeros |
| `gridsquare` | text / locator | O | `grid` |
| `exchangel` (letter l; S1) | text / received exchange | O | `exchange_received`, fallback only |
| `exchange1` (digit 1; S3) | text / received exchange | O | `exchange_received`, preferred if present |
| `section` | text / contest-defined section | O | `section`, no ADIF reinterpretation |
| `comment` | text / — | O | `comments` |
| `qth` | text / worked station information | O | native only |
| `name` | text / worked station name | O | `name` |
| `power` | text / worked station's exchange, not own TX power | O | native only; never `tx_power` |
| `misctext` | text / — | O | native only |
| `zone` | integer text / contest-defined zone | O | native only |
| `prec` | text / precedence | O | `precedence` |
| `ck` | text / contest check | O | `check` |
| `ismultiplierl` (S1) | integer text / multiplier flag | O | native only |
| `ismultiplier1` (S3) | integer text / multiplier flag | O | native only |
| `ismultiplier2` | integer text / multiplier flag | O | native only |
| `ismultiplier3` | integer text / multiplier flag | O | native only |
| `points` | integer text / QSO points | O | native only |
| `radionr` | int / radio number | O | `radio.number` |
| `run1run2` (S1) | integer text / multi-two run radio | O | native only |
| `Run1Run2` (S3) | integer text / historical spelling | O | native only |
| `ContactType` (S3 only) | text / undocumented meaning | O | native only |
| `RoverLocation` | text / unspecified | O | native only |
| `RadioInterfaced` | boolean text / unspecified spelling | O | native only |
| `NetworkedCompNr` | integer text / unspecified numbering | O | native only |
| `IsOriginal` | boolean text / originally logged here | O | native only |
| `NetBiosName` | text / — | O | native only |
| `IsRunQSO` | boolean text / unspecified spelling | O | native only |
| `StationName` | text / forwarding computer label | O | `radio.instance` |
| `ID` (S1) | text / opaque contact identifier | O | `record_id`, no GUID length repair |
| `IsClaimedQso` (S1) | boolean text / 1 claimed, 0 X-QSO | O | native only |
| `oldtimestamp` (S1) | text / prior unresolved timestamp | O | native only |
| `oldcall` (S1) | text / prior callsign | O | native only |
| `SentExchange` (S1) | text / configured exchange template | O | native only; never actual `exchange_sent` |

**`QGI-N1M-0031` — Exchange aliases.** A reader MUST choose the unique `exchange1`
leaf when present, otherwise the unique `exchangel` leaf; presence includes empty,
invalid or duplicate `exchange1`, so an ambiguous preferred leaf MUST NOT trigger
fallback. This QGI precedence resolves S1/S3's spelling conflict, not a claim that
the logger accepts either spelling as input.

S1 says edits can emit a delete followed by replace, and describes `oldcall` and
`oldtimestamp` as the prior key. Mapping preserves each datagram independently;
it does not synthesize a transaction. S1's ID prose says both “32 byte” and two hex
characters per byte, while its samples disagree about length/hex validity. Keeping
an opaque record ID avoids enforcing an unsupported GUID grammar. `SentExchange`
may contain serial placeholders and is not necessarily the actual sent exchange
(S1 Contact notes). No composite exchange is assembled from other fields.

### contactdelete

Evidence for every row: **S1 Contact Delete; S3 contactdelete** (the latter carries
only timestamp, call, contestnr and StationName). Extra contact children use the
unknown-child rule, without importing the full add/change catalog into deletion.

| Element | Type / unit | Requiredness | Common mapping |
|---|---|---|---|
| `app` | text / — | O | native only |
| `timestamp` | text / unresolved timestamp | O | native only |
| `mycall` | text / callsign | O | `station_call` |
| `band` | text / native band label | O | `band` |
| `call` | text / callsign | O | `call` |
| `contestnr` | integer text / local contest instance | O | native only |
| `StationName` | text / forwarding computer label | O | `radio.instance` |
| `ID` | text / opaque contact identifier | O | `record_id` |

### RadioInfo

Evidence for every row: **S1 Radio Info and notes**; **S3 RadioInfo** corroborates
its older subset. The logging mode is text, not a closed enum: S1's list is not used
to reject newer modes.

| Element | Type / unit | Requiredness | Common mapping |
|---|---|---|---|
| `app` | text / program label | O | native only |
| `StationName` | text / computer label | O | `radio.instance` |
| `RadioNr` | int / radio or SO2V VFO number | O | `radio.number` |
| `Freq` | int / 10 Hz receive frequency | O | `dial_frequency_hz` = value × 10 |
| `TXFreq` | int / 10 Hz reported transmit frequency | O | native only; no common absolute TX slot in `radio_state` |
| `Mode` | text / logging mode | O | `mode` |
| `mycall` | text / station callsign | O | `station_call` |
| `OpCall` | text / operator callsign | O | native only; not station identity |
| `IsRunning` | bool / RUN versus S&P | O | `running` |
| `FocusEntry` | integer text / Windows handle | O | native only |
| `EntryWindowHwnd` | integer text / Windows handle | O | native only |
| `Antenna` | integer text / selection 0–63 per S1 | O | native only |
| `Rotors` | text / selected rotor name | O | native only |
| `FocusRadioNr` | int / receive or keyboard focus, normally 1/2 | O | `focus_radio` |
| `IsStereo` | bool / SO2R audio switching | O | `stereo` |
| `IsSplit` | boolean text / split VFO state | O | native only |
| `ActiveRadioNr` | int / transmit focus, normally 1/2 | O | `active_radio` |
| `IsTransmitting` | bool / program-initiated transmission | O | `transmitting` |
| `FunctionKeyCaption` | text / initiating function-key label | O | native only |
| `RadioName` | text / radio display name | O | native only |
| `AuxAntSelected` | integer text / auxiliary antenna; −1 normally | O | native only |
| `AuxAntSelectedName` | text / auxiliary antenna name | O | native only |
| `IsConnected` | boolean text / connection indication | O | native only |

**`QGI-N1M-0002` — Radio labels.** A reader MUST map only carried `StationName`
and `RadioNr` (or contact `radionr`) to `radio.instance` and `radio.number`,
respectively, omitting the object when neither is mappable; it MUST NOT create
station names to split a two-radio instance, remember missing parts, or fill
`source_id` (S1 Radio Info and Contact notes; `QGI-MOD-0014`). Spot `StationName`
also supplies `radio.instance`, although S1 Spot Data describes it as a station
callsign rather than the computer label described for Radio Info.

**`QGI-N1M-0003` — SO2R mapping.** A reader MUST use `FocusRadioNr` → `focus_radio`,
`ActiveRadioNr` → `active_radio`, `IsTransmitting` → `transmitting`, `IsRunning` →
`running`, and `IsStereo` → `stereo` exactly as D.4, independently of `RadioNr`
(S1 Radio Info notes). This confirms the earlier provisional assignments.

**`QGI-N1M-0032` — Transmit indication.** A receiver MUST NOT treat `transmitting:
false` as proof the radio is receiving: external PTT/paddles can transmit while
N1MM reports false (S1 IsTransmitting warning).

**`QGI-N1M-0034` — SO2V frequency use.** A receiver using the reported native
TXFreq to control equipment in SO2V SHOULD use only packets with RadioNr equal to
ActiveRadioNr (S1 SO2V warning).

**`QGI-N1M-0035` — Topology.** A codec MUST NOT infer physical radio topology
from the numbers: S1 SO2V notes say 1/2 can label VFO A/B in one radio.

### spot

Evidence for every row: **S1 Spot Data; S3 spot** (S3 omits app, mode, statuslist).

| Element | Type / unit | Requiredness | Common mapping |
|---|---|---|---|
| `app` | text / — | O | native only |
| `StationName` | text / station label | O | `radio.instance` |
| `dxcall` | text / spotted callsign | O | `call` |
| `frequency` | decimal / kHz (D.1 derivation) | O | `frequency_hz` = value × 1,000 (`QGI-N1M-0036`) |
| `spottercall` | text / spotter callsign | O | `spotter_call` |
| `timestamp` | text / unresolved spot time | O | native only |
| `action` | text / add or delete per S1 | O | native only |
| `mode` | text / — | O | `mode` |
| `comment` | text / — | O | `text` |
| `status` | text / status label | O | native only |
| `statuslist` | text / unspecified list grammar | O | native only |

S1 names busy, bust, cq, dupe, qtc, single mult, double mult and new qso statuses.
They are not a QGI enum. A `spot` event alone does not mean addition: consumers
inspect native `action` before maintaining a spot list. No source field is inferred
from `app`, `StationName`, or `spottercall`.

**`QGI-N1M-0036` — Exact kHz conversion and native retention.** For spot
`frequency`, `radio_setfrequency/frequency`, and Spectrum LowScopeFrequency and
HighScopeFrequency, a reader MUST parse decimal kHz exactly, without binary
floating point, and multiply by 1,000 to obtain the Hz integer in D.4's mapped
field. The grammar is optional `+` or `-`, one or more ASCII digits, optionally
followed by `.` and one or more ASCII digits; only surrounding XML whitespace is
ignored. Exponents and other spellings are not accepted. If the text is invalid,
empty, not a whole number of Hz after conversion, or outside the common integer
range −9,007,199,254,740,991 through 9,007,199,254,740,991, the reader MUST omit
that mapped field. D.3's duplicate, nested-leaf and numeric-token rules still
apply. The original native text MUST remain unchanged in `native.data`, including
when conversion succeeds or the common field is omitted; conversion MUST NOT
round, truncate or rewrite native evidence. No separate native-text field is added.

### dynamicresults

Evidence for every row: **S1 Score Reporting**. Containers and repeated entries are
optional; each listed attribute is optional. Attribute types are text. Score selectors compare XML-normalized attribute values.

| Element/path | Type / unit | Requiredness | Common mapping |
|---|---|---|---|
| `contest` | text / — | O | `contest_name` |
| `call` | text / contest station call | O | `station_call` |
| `ops` | text / operator list | O | native only |
| `class` | container, attributes below | O | native only |
| `class/@power` | text / category | O | native only |
| `class/@assisted` | text / category | O | native only |
| `class/@transmitter` | text / category | O | native only |
| `class/@ops` | text / category | O | native only |
| `class/@bands` | text / category | O | native only |
| `class/@mode` | text / category | O | native only |
| `class/@overlay` | text / category | O | native only |
| `club` | text / — | O | native only |
| `qth` | container | O | native only |
| `qth/dxcccountry` | text / — | O | native only |
| `qth/cqzone` | integer text / CQ zone | O | native only |
| `qth/iaruzone` | integer text / IARU zone | O | native only |
| `qth/arrlsection` | text / — | O | native only |
| `qth/stprvoth` | text / — | O | native only |
| `qth/grid6` | text / locator | O | native only |
| `breakdown` | container | O | native only |
| `breakdown/qso` | int / count, repeated | O, repeatable | `qso_count` only for unique total/ALL selector |
| `breakdown/point` | int / points, repeated | O, repeatable | `points` only for unique total/ALL selector |
| `breakdown/qso/@band` | text / native band selector | O | selector only |
| `breakdown/qso/@mode` | text / native mode selector | O | selector only |
| `breakdown/point/@band` | text / native band selector | O | selector only |
| `breakdown/point/@mode` | text / native mode selector | O | selector only |
| `score` | int / score | O | `total_score` |
| `timestamp` | text / unresolved timestamp | O | native only |

**`QGI-N1M-0033` — Score totals.** A reader MUST map only the unique `qso` or
`point` leaf with attributes exactly `band="total"` and `mode="ALL"` within the
unique `breakdown`, omitting the corresponding total on duplicates; it MUST NOT
sum band rows or infer multipliers, band, or mode from them. S1 shows these selectors
but does not define whether overlapping rows are additive. Extra attributes do not
alter the selector. A `qth` field is a child element, not an attribute (S1 sample).

### Inbound radio_setfrequency and Spectrum

Evidence for every row: **S1 Setting N1MM's Frequency** or **S1 Sending Spectrum
Data / S3 Spectrum**, respectively. Observation requiredness stays O; writer
requiredness is separately defined in D.5.

| Root / element | Type / unit | Requiredness | Common or family argument on observation |
|---|---|---|---|
| `radio_setfrequency/app` | text / sender program label | O | `app` |
| `radio_setfrequency/radionr` | int / radio selector | O | `radio.number` |
| `radio_setfrequency/frequency` | decimal / kHz (D.1 derivation) | O | `frequency_hz` = value × 1,000 (`QGI-N1M-0036`) |
| `radio_setfrequency/mousebutton` | text / Left selects 1, Right selects 2 in SO2V/R | O | native only |
| `Spectrum/app` | text / sender program label | O | `app` |
| `Spectrum/Name` | text / UI source name | O | `spectrum_name` |
| `Spectrum/LowScopeFrequency` | decimal / kHz (D.1 derivation) | O | `low_scope_frequency`, integer Hz (`QGI-N1M-0036`) |
| `Spectrum/HighScopeFrequency` | decimal / kHz (D.1 derivation) | O | `high_scope_frequency`, integer Hz (`QGI-N1M-0036`) |
| `Spectrum/ScalingFactor` | decimal text / factor to display dB | O | `scaling_factor` |
| `Spectrum/DataCount` | integer text / number of levels | O | native only |
| `Spectrum/SpectrumData` | text / comma-separated integer levels 0–65,535 per S1 | O | native only on observation |

## D.5 Commands from fields

These writers are QGI canonical encodings of S1's examples, using the kHz unit
derived in D.1. The construction ranges below are QGI policies; the sources do
not establish all acceptance ranges.

| ID | Requirement | Verify |
|---|---|---|
| `QGI-N1M-0040` | A command writer MUST support `set_frequency` and `n1mm.spectrum` from the fields below without native evidence; it MUST reject other operations or missing/disallowed arguments as `invalid_value`. | Test |
| `QGI-N1M-0041` | For `set_frequency`, a writer MUST require `frequency_hz` (integer 0–9,007,199,254,740,991), `radio: {"number": 1 or 2}` and `app` (valid XML text string). There is no `frequency_unit` encoding option; all encoding options are disallowed (`QGI-N1M-0046`). | Test |
| `QGI-N1M-0042` | A frequency writer MUST divide supplied integer Hz exactly by 1,000 and write kHz as a plain nonnegative decimal with no leading zeros (except zero itself or the zero before a fractional point), no exponent and no trailing fractional zeros or bare decimal point: 14,053,284 Hz becomes `14053.284`, 7,000,000 Hz becomes `7000`, and 1 Hz becomes `0.001`. A `set_frequency` writer MUST select `mousebutton` Left for radio 1 or Right for radio 2 (S1 selection rule). | Test |
| `QGI-N1M-0043` | For `n1mm.spectrum`, a writer MUST require valid XML text strings `app` and `spectrum_name`, integer Hz arguments `low_scope_frequency` and `high_scope_frequency` in 0–9,007,199,254,740,991 with high greater than low, positive decimal string `scaling_factor`, and `spectrum_data` containing 1–4,096 integers in 0–65,535; it MUST encode both scope frequencies as kHz under `QGI-N1M-0042` and derive DataCount from the array length. | Test |
| `QGI-N1M-0044` | A writer MUST emit UTF-8 with the exact declaration `<?xml version="1.0" encoding="utf-8"?>`, then the root and children in the order below, with no BOM, whitespace between tags, newline, or delimiter. | Test |
| `QGI-N1M-0045` | A writer MUST escape text ampersands, less-than and greater-than signs as `&amp;`, `&lt;`, `&gt;`, and CR as `&#13;`, in one pass; it MUST reject null, raw wrappers, surrogates and forbidden XML characters in command strings. Quotes, tabs and LF remain literal in element text. | Test |
| `QGI-N1M-0046` | A writer MUST reject unsupported family mapping revisions as `unsupported_version`, any `native_version` or `wire_id`, any encoding option or argument outside the operation's catalog, and any radio selector containing `instance` or members other than `number` as `invalid_value`. | Test |
| `QGI-N1M-0047` | A writer MUST apply D.3's datagram/text/token bounds to its result before returning bytes, failing atomically as `oversized`. | Test |

The spectrum `scaling_factor` decimal grammar is `[0-9]+` optionally followed by
`.` and one or more digits, at most 128 characters; signs, exponent notation and whitespace are not
accepted for construction. Its supplied lexical spelling is kept. Scope frequency
arguments are Hz integers, with the same canonical kHz encoding as `set_frequency`.
`spectrum_data` construction is a family argument, not a common measured
spectrum field. No inferred DataCount or levels are added to observed events.

| Operation | Root | Child order |
|---|---|---|
| `set_frequency` | `radio_setfrequency` | app, radionr, frequency, mousebutton |
| `n1mm.spectrum` | `Spectrum` | app, Name, LowScopeFrequency, HighScopeFrequency, ScalingFactor, DataCount, SpectrumData |

For example, fields `{"operation":"set_frequency","app":"synthetic-client",
"radio":{"number":2},"frequency_hz":14053284}` encode to exactly these UTF-8
bytes (no final LF):

```xml
<?xml version="1.0" encoding="utf-8"?><radio_setfrequency><app>synthetic-client</app><radionr>2</radionr><frequency>14053.284</frequency><mousebutton>Right</mousebutton></radio_setfrequency>
```

S1 says the QSY input changes N1MM and its connected radio's frequency; S1 says the
spectrum input supplies levels for display after the user selects that source.
Neither documents an acknowledgment or correlated response. No result, subsequent
RadioInfo, or response timeout is synthesized. Reading never performs either effect.
Live captures under `QGI-RI-0063` will confirm the documented-example derivation
and establish actual peer precision and acceptance ranges.

## D.6 Re-encoding and structured view

| ID | Requirement | Verify |
|---|---|---|
| `QGI-N1M-0048` | Observed re-encoding MUST validate recognized fields against native evidence, then return the exact native bytes, preserving declarations, spacing, boolean spelling, empty-element spelling, unknown members and original encoding; original-byte retrieval remains a separate operation without that consistency check. | Test |
| `QGI-N1M-0049` | If offered, `native.message` MUST contain exactly `root` (literal qualified root name, string) and `xml` (the whole document decoded in the detected encoding, excluding only the BOM, using the matching invalid-text wrapper if necessary); this view MUST NOT apply XML entity or line-ending normalization to `xml`. | Test |

Every known type may be re-encoded for a simulator, including broadcasts. There is
no reconstruction from a DOM that loses attribute order or lexical spellings.
`native.message` is optional and informative; it cannot override `native.data`.
Compatibility vectors preserve the original event and separately expose a recognized
view, including unavailable verification for a hypothetical new root. For a known
kind at a newer mapping revision, recognized common fields remain readable even
when the new root prevents native verification, as in the core outcome table.

## D.7 Output budget and schema scope

**`QGI-N1M-0050` — Observation bound.** An observer MUST use compact UTF-8 JSON
escaping only required characters (or a representation no larger), enforce
`QGI-SER-0004`, and constrain the optional view to D.6. Contexts use
`QGI-SER-0010`. No speculative common fields or per-element structured trees may
inflate this revision's output.

For N = **65,507 bytes**, base64 evidence is 4 × ceil(N / 3) = **87,344** bytes.
Every mapped leaf contributes to at most one common value (including the radio
label); all common text/numeric content fits the conservative **6N** bound for
JSON escapes or invalid-code-unit arrays. XML entity expansion is restricted and
cannot increase this above 6 per source byte. The optional whole-document `xml`
view costs another **6N**; UTF-16 decoding/invalid-unit arrays cost no more per
native byte. Root text is at most 128 bytes; count an extra **768** JSON bytes.
A **32,768** byte reserve covers names, punctuation, wrappers and scalars: there
are fewer than 256 common/view/envelope members, at 128 bytes each. Context and
all hops use the existing **65,536** byte reserve. The total is

87,344 + 12 × 65,507 + 768 + 32,768 + 65,536 = **972,500 bytes**,

leaving **76,076 bytes** below 1 MiB. Thus every admitted datagram fits, including
its largest allowed observation/view. The 65,507 bound is a QGI admission limit,
not an upstream claim about a maximum UDP payload. Vectors include a full-sized
mapped text leaf with its view and an over-limit datagram.

**Schema impact:** adding `radio` to `control` in Annex A and Annex F is a **core
capacity addition** (`QGI-MOD-0016`): inbound frequency commands need a shared radio selector and
must not misuse `wire_id`. It changes no existing meaning or required field. The
`control` arguments `app`, `spectrum_name`, `low_scope_frequency`,
`high_scope_frequency`, `scaling_factor`, and `spectrum_data` are **N1MM profile
members only**, enabled by the schema's `family: n1mm` condition; they do not
change the shared field catalog. No score/QSO capacity is added. In particular,
RadioInfo TXFreq, unresolved times and exchange templates remain native-only
instead of changing or overloading existing common members.

## D.8 Numbered questions for live captures (QGI-RI-0063)

These questions do not authorize changing the policies above without a mapping
revision. The read-only comparison with the local Python adapter is implementation
evidence only, never a protocol source; it motivates Q12–Q15 below.

1. What are the accepted precision/range of spot `frequency`,
   `radio_setfrequency/frequency`, LowScopeFrequency and HighScopeFrequency?
2. Are contact, spot and score timestamps UTC, local, or configurable? What is their
   exact grammar, fractional precision and behavior at DST transitions?
3. Which leaves are required by each current build, and what do omitted, paired
   empty and self-closing leaves mean, especially numeric and boolean leaves?
4. Which boolean spellings occur or are accepted: True/False, true/false, 1/0,
   uppercase, yes/no or surrounding whitespace? Are blank connection flags meaningful?
5. Are non-UTF-8 broadcasts possible? Is a BOM/declaration required or accepted,
   and how are non-ASCII/invalid text, XML entities and line endings serialized?
   The local adapter replaces invalid UTF-8 while referring to Windows-1252;
   do captures establish a legacy encoding? This profile preserves invalid units.
6. What is the actual maximum datagram/text/count/depth size? Can broadcasts fragment,
   combine documents, or contain trailing bytes? Are large spectrum arrays accepted?
7. Do current builds emit `lookupinfo` or `lookup`, `exchange1` or `exchangel`,
   `ismultiplier1` or `ismultiplierl`, and `run1run2` or `Run1Run2`? What does
   ContactType mean? Can both alias spellings coexist?
8. What are the exact contact ID length/encoding, delete fields and edit packet
   sequence? Do forwarded contacts keep StationName, NetBiosName and IsOriginal as described?
9. Is spot StationName a callsign or a computer label in every mode? Can radio
   numbers exceed 1/2, and do different topology modes change their interpretation?
10. For inbound QSY, which of radionr or mousebutton wins on conflict, are both
    required, is zero frequency accepted, and is there any response not documented
    in S1? Which other inbound messages are actually supported?
11. What are spectrum ScalingFactor's range and sign, levels/count mismatch rules,
    endpoint ordering, accepted source names and update-rate limits? What is the
    unspecified rotator wire layout, and does it even use XML?
12. The local adapter trims all text and substitutes defaults for missing station,
    radio, booleans and integers. Do captures justify any of these transformations?
    This profile preserves text and omits unmappable values instead.
13. The local adapter accepts case-insensitive `yes` and treats other boolean text
    as false. Do captures show those spellings? This profile does not guess false.
14. The local adapter reads score qth values as attributes, while S1 shows children,
    and stores radio frequency integers without this profile's ×10 conversion.
    What do live bytes show? Neither behavior is treated as source evidence.
15. The local adapter recognizes lookup bearing/distance not listed in S1/S3, uses
    a 4,096-byte receive buffer and does not cover the inbound spectrum/QSY layouts.
    Which extra fields or larger packets occur? Units and semantics need evidence.
16. Can duplicate/nested leaves, namespaces, attributes on scalar elements or new
    program-version metadata occur? What semantics should a later profile assign?
17. Are score total/ALL selectors stable, can totals repeat, and are multiplier
    rows or extra attributes present? Are breakdown counts/points additive or overlapping?
18. Do malformed snippets in S1/S3 reflect only documentation errors (including
    declaration `1 .0`, broken run1run2, ambiguous ID prose and timestamp typos)?
