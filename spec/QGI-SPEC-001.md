<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QGI-SPEC-001: the QG Interop specification

**Status: draft 0.1.0, 2026-11-12.** The core, WSJT-X mapping, schema and conformance
vectors are specified. Annexes C–E and G–I remain placeholders (Annex I already defines event carriage
and its vectors); all seven families are planned for the first
release. Reference implementation alignment and real-peer interoperability remain separate work.
Nothing here is a promise of compatibility until the first numbered release of this specification.

From KI7MT's direction of 2026-11-11: QG Interop **defines a specification, as ADIF does, and others
implement it**. QSO Graph proves it with a reference implementation for WSJT-X, N1MM Logger+, fldigi
and JS8Call-improved, plus PSK Reporter MQTT and IPFIX and QG Interop's own `qgi` event carriage.
It delivers the implementation in a CLI,
a simulator and QSO Graph Desktop
(`QGI-RI-001`).

**This specification is independent of programming languages, libraries and operating systems.** Conformance means
meeting the normative requirements, demonstrated by the versioned suite (§7).

### Scope boundaries (informative)

| Interface | Scope |
|---|---|
| COM and DDE | Not in scope: Windows-only interprocess communication. |
| Serial CAT radio control | Not in scope: belongs to a separate radio-control project. |
| rigctld | Designed for, not yet mapped: Annex Z.1. |
| DX cluster | Designed for, not yet mapped: Annex Z.2. |
| ADIF over UDP | Designed for, not yet mapped: Annex Z.3. |
| KISS | Designed for, not yet mapped: Annex Z.4. |

## 0.1 How this document works

**One fact, one place.** Every normative statement lives here once, with an ID; other documents
cite it. The words MUST, MUST NOT, SHOULD and MAY are used as RFC 2119 defines them.

| Area | Covers |
|---|---|
| `QGI-GEN` | Purpose, terms, conformance classes, versioning of this specification |
| `QGI-MOD` | The event model: envelope, kinds, fields, value types |
| `QGI-SER` | The JSON representation |
| `QGI-MAP`, `QGI-CLS` | Mapping and protocol-class rules common to every family |
| `QGI-WSJ`, `QGI-JS8`, `QGI-N1M`, `QGI-FLD`, `QGI-PSM`, `QGI-PSI`, `QGI-QGI` | Official family mappings |
| `QGI-EXT` | Families and fields defined by others |
| `QGI-VEC` | Test vectors and conformance |
| `QGI-SEC` | Security considerations |

**Verify methods:** `Test` (a test vector or automated check), `Inspection` (read against this text).

## 1. General

| ID | Requirement | Verify |
|---|---|---|
| `QGI-GEN-0001` | **Purpose: many to many.** This specification defines one event model and a mapping for each native protocol, so N peers and M applications need N + M integrations rather than N × M. Applications construct shared commands from fields without learning native byte layouts. | Inspection |
| `QGI-GEN-0002` | **Terms.** A *peer* is a program or service whose native protocol QG Interop maps, including station software and PSK Reporter. A *family* is a named native protocol profile; a *protocol class* defines framing and envelope rules shared by families. A *native message* is one class-framed input. An *observed event* represents a decoded message or record and carries its native evidence. A *command event* is an application-built control awaiting encoding. | Inspection |
| `QGI-GEN-0003` | **Conformance classes.** A **codec** decodes native messages into zero or more events and, where defined, encodes commands into native messages. A **consumer** reads events; a **producer** writes events. A conformance claim MUST name its role, families and specification version. | Inspection |
| `QGI-GEN-0004` | **Protocol, not policy.** This specification defines what a message means, not what an application does with it. Who may send, which transports are enabled, what is displayed, stored or logged, and any content rule stricter than the family's are the application's decisions. | Inspection |
| `QGI-GEN-0005` | **Versioning.** The specification MUST use `MAJOR.MINOR.PATCH`; the envelope identifies that version. A MINOR release MAY add kinds, optional fields, operation tokens, context enum values and families, but MUST NOT make a field required or change an existing type, unit, default or meaning. | Test |
| `QGI-GEN-0006` | **PATCH scope.** A PATCH release MUST change only wording, examples or vector corrections. | Inspection |
| `QGI-GEN-0007` | **Major boundary.** A consumer MUST reject an event with a different MAJOR version. | Test |

## 2. The event model

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MOD-0001` | **Envelope.** An event MUST contain `spec` (version string; `0.1.0` here), `family` (registered name), `mapping` (positive annex revision), `kind` (string), `context` (object) and `fields` (object). | Test |
| `QGI-MOD-0002` | **Context.** `context` MUST contain observer-supplied `direction` (`from_application`, `to_application`, `unknown`) and `exchange` (`notification`, `request`, `response`, `unknown`); observer-supplied `source_id`, `target_id` and `correlation_id` are optional text. An observer without that knowledge MUST supply `unknown` for each enum; a codec MUST preserve the supplied context without inferring identities or exchange roles from the message. | Test |
| `QGI-MOD-0003` | **Kinds.** The registered kinds are `decode`, `radio_state`, `qso_logged`, `qso_changed`, `qso_deleted`, `spot`, `score`, `control`, `result`, `notification`, `lifecycle`, `ambiguous` and `native`. | Test |
| `QGI-MOD-0004` | **Common fields.** A codec MUST map fields according to Annex A and its family annex, without inventing values or deriving them from free text; a native field that is absent MUST leave its common member absent. The representability exceptions are `QGI-MOD-0006` and `QGI-MOD-0013`. | Test |
| `QGI-MOD-0005` | **Absent, null and empty are distinct** wherever the family distinguishes them, and the event preserves the distinction. | Test |
| `QGI-MOD-0006` | **Time types.** An `instant` MUST be an RFC 3339 UTC string ending in `Z`; a `time_of_day` MUST be `HH:MM:SS` in UTC without a date. Both carry fractional seconds only to the native precision. A local, unresolved-zone, invalid or null native time MUST leave its common member absent; the original remains in `native`. Annex A assigns a type to each time field. | Test |
| `QGI-MOD-0007` | **Lossless evidence.** An observed event MUST preserve exact native bytes, including unmapped fields and extensions, according to its protocol class; its family annex MUST list common-field canonicalizations. | Test |
| `QGI-MOD-0008` | **Reading never acts.** Mapping a native message to an event never changes `native` and never sends anything. Encoding an event into a native message is an explicit operation, and whether to send the result is the application's decision (`QGI-GEN-0004`); the event's kind is not an authorization. | Inspection |
| `QGI-MOD-0009` | **Native object.** When present, `native` MUST contain `format` (class/profile format token) and `data` (canonical padded RFC 4648 base64), and MAY contain an informative family-defined `message` object. | Test |

### 2.1 Text

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MOD-0010` | **Text is carried exactly as the family sent it.** A codec MUST NOT strip, replace, normalize or truncate characters. Text that is not valid in its declared encoding is carried as its raw code units. | Test |
| `QGI-MOD-0011` | **All text in an event is untrusted**, including text supplied by the observer (§8). This specification does not define which characters an application may display; that is application policy (`QGI-GEN-0004`). | Inspection |
| `QGI-MOD-0012` | **Native boundaries.** A family annex MUST select a protocol class from §4.1, define its native message and identify framing bytes separately from content bytes. | Test |
| `QGI-MOD-0013` | **Common integers.** Every integer common field MUST lie in −9,007,199,254,740,991 through 9,007,199,254,740,991; a native integer outside that range MUST leave the common member absent. Frequencies MUST use integer hertz. | Test |

### 2.2 Observations and commands

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MOD-0030` | An observed event MUST carry `native`. | Test |
| `QGI-MOD-0031` | A command event MUST have `kind: "control"` and omit `native`. | Test |
| `QGI-MOD-0032` | `control`, `notification` and `ambiguous` MUST carry `fields.operation` from Annex A or the family annex. | Test |
| `QGI-MOD-0033` | A command MUST place its arguments in `fields` and its target in `wire_id` when the family uses a target ID. | Test |
| `QGI-MOD-0034` | An observed event MUST include `native_version` only when its native input declares a version, and `wire_id` only when the family carries an ID. | Test |
| `QGI-MOD-0035` | A command MUST omit `native_version`; the caller supplies the encoding version as an encoding option. | Test |
| `QGI-MOD-0036` | A family MUST use the shared operation token and argument names in Annex A for the same action. | Test |
| `QGI-MOD-0037` | An operation specific to one family MUST use `<family>.<token>` (for example, `wsjtx.highlight_callsign`). | Test |
| `QGI-MOD-0038` | A `result` MUST carry `fields.status` (`ok` or `error`); a native application error is a successfully decoded result, not a codec error. | Test |

The narrow discriminator is presence of `native`, with no new envelope flag: even an observed
`control` carries it. Annex F removes `native` from the unconditional `required` list and uses
`if` (absence of `native`) / `then` (`kind` fixed to `control`, required `fields.operation`, and
no `native_version`) / `else` (required `native`). Schema cannot establish that a producer
truthfully distinguishes a wire observation from an application-built command; the codec API does.
After encoding, decoding the produced bytes yields an observed event with native evidence; the
command input is not mutated. Native and mapping versions are independent.

## 3. JSON representation

| ID | Requirement | Verify |
|---|---|---|
| `QGI-SER-0001` | **An event's interchange form is one JSON object (RFC 8259, UTF-8)**, valid against the JSON Schema published with each version of this specification. | Test |
| `QGI-SER-0002` | **Text.** Valid text MUST be a JSON string; a family’s explicit null MUST be JSON `null`, empty text `""`, and a missing field an omitted member. Text invalid in its declared encoding MUST be `{"utf8_bytes": [...]}` (integers 0–255) or `{"utf16_units": [...]}` (integers 0–65535, including unpaired surrogates). These forms also apply to text in `wire_id`, context and the optional structured view. | Test |
| `QGI-SER-0003` | **Binary64 floating point:** a finite value is a JSON number written so that a correctly rounding reader recovers the same bits; a non-finite value is the string `ieee754:` followed by 16 lowercase hexadecimal digits of its bits, preserving NaN payloads. A consumer that cannot read numbers with correct rounding MUST use the native message instead. | Test |
| `QGI-SER-0004` | **Sequences.** An event sequence MUST use newline-delimited JSON (NDJSON), one object per record, with each record at most 1,048,576 UTF-8 bytes excluding its line delimiter; a larger record MUST be an `oversized` error. | Test |
| `QGI-SER-0005` | **Compatible reading.** A consumer reading a newer MINOR of the same MAJOR MUST ignore unknown members, treat unknown kinds as `native`, unknown context enums as `unknown`, and unknown families as kind `native` with uninterpreted fields; it MUST preserve any native evidence and the original command when evidence is absent. Such a command is not encodable without a supported family/operation mapping. | Test |
| `QGI-SER-0006` | **Compatible writing.** A producer writing for an older reader of the same MAJOR MUST retain everything that version requires. | Test |
| `QGI-SER-0007` | **Duplicate members.** An event with duplicate member names at any object depth MUST be rejected as `malformed`; this is a parsing rule that JSON Schema cannot express. | Test |
| `QGI-SER-0008` | **Equality.** Events MUST be compared as parsed values, independent of object member order, using binary64 values for floating fields and exact integers otherwise; base64 data remains exact. | Test |

## 4. Mapping rules for every family

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MAP-0001` | **Every family's mapping is pinned to released source or a dated published document**, named with its version and a cryptographic hash, and re-pinned only by a new revision of the family's section. | Inspection |
| `QGI-MAP-0002` | **Never guess.** A codec MUST follow its family’s version and type rules, returning a skip for an unknown future version or unknown type rather than interpreting an encoding it does not know. | Test |
| `QGI-MAP-0003` | **Limits are declared per family** (largest message, largest field) and checked before a codec allocates; a message beyond them is an error, not a truncation. | Test |
| `QGI-MAP-0004` | **No identity from the wire.** A wire ID, an application or station label, or a network address is data, not proof of who sent the message, and never fills `source_id`. | Inspection |
| `QGI-MAP-0005` | **Atomic errors.** On malformed input or any other message error, a decoder MUST return no events and commit no state updates for that message. | Test |
| `QGI-MAP-0006` | **Outcomes.** A decoder MUST return either a successful ordered collection of zero or more events and zero or more skip notices, or one error categorized as `truncated`, `oversized`, `malformed`, `unsupported_version` or `invalid_value`. | Test |
| `QGI-MAP-0007` | **Check precedence.** A decoder MUST check size, framing or magic, version, type and body in that order where those concepts apply; a family annex MUST specify record/template ordering within the body. | Test |
| `QGI-MAP-0008` | **Error categories.** A codec MUST use `truncated` for missing bytes of a required or begun field, `oversized` for a declared resource bound exceeded, `malformed` for invalid framing or structural encoding, `unsupported_version` for an explicitly unsupported version (a future native version is skipped on decoding) and `invalid_value` for a structurally complete but disallowed value. | Test |

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MAP-0009` | A skip MUST use `future_version`, `unknown_type` or `unknown_template`, including readable native version/type and a record or Set location where applicable. | Test |
| `QGI-MAP-0010` | Command encoding MUST use only `fields`, `fields.operation`, `wire_id`, the selected family/mapping and explicit encoding options, without requiring prior native bytes. | Test |
| `QGI-MAP-0011` | Missing required command arguments or disallowed argument values MUST return `invalid_value`. | Test |
| `QGI-MAP-0012` | Re-encoding an observed event from its native evidence MUST be a separate explicit operation from command encoding. | Test |
| `QGI-MAP-0013` | A family MUST publish command-to-byte reverse vectors, including missing and out-of-range arguments, for each defined sending type. | Test |
| `QGI-MAP-0014` | Exceeding a declared state or event bound MUST return `oversized` atomically; codecs MUST NOT silently evict a live template to accept the message. | Test |

### 4.1 Protocol classes

The class rules below select the native evidence boundary. A new family using these rules needs
only an annex; it does not add envelope members. A genuinely new class requires a core/schema
revision before introducing native envelope members.

| ID | Requirement | Verify |
|---|---|---|
| `QGI-CLS-0001` | **Datagrams.** `native.data` MUST contain the complete datagram payload. | Test |
| `QGI-CLS-0002` | **Delimited streams.** `native.data` MUST contain one complete framed message excluding only its outer delimiter; embedded newlines and escape bytes remain exact. | Test |
| `QGI-CLS-0003` | **Request/response.** The family MUST define request and response boundaries (HTTP XML-RPC: one body excluding HTTP headers; text streams: one complete reply, potentially multiple lines). | Test |
| `QGI-CLS-0004` | A response whose interpretation depends on its request MUST receive that request's method/operation from the caller; absent that association it MUST remain `native` rather than infer the method. | Test |
| `QGI-CLS-0005` | Request/response association MUST preserve caller-supplied `correlation_id` without inventing one from message order. | Test |
| `QGI-CLS-0006` | For HTTP XML-RPC, `native.data` MUST contain exactly one complete request or response body, excluding HTTP headers. | Test |
| `QGI-CLS-0010` | **Publish/subscribe.** A decoded PUBLISH MUST carry `native.topic` (text), `native.retained` (boolean retain flag) and `native.data` (exact payload bytes). | Test |
| `QGI-CLS-0011` | A retained PUBLISH MUST remain marked retained and MUST NOT be represented as a new observation solely because it was delivered now. | Test |
| `QGI-CLS-0012` | Broker connections, subscriptions, credentials and delivery acknowledgments MUST remain outside the codec, which decodes one caller-supplied PUBLISH. | Inspection |
| `QGI-CLS-0013` | Families MUST NOT add native envelope members beyond `format`, `data`, `message` and those listed by their selected class, except `qgi`'s `carried_native` (`QGI-QGI-0004`); family-specific structured members belong inside `native.message`. | Test |
| `QGI-CLS-0020` | **Templated records.** A valid templated message MUST produce one event per decoded data record in wire order; a template-only message MUST produce no events. | Test |
| `QGI-CLS-0021` | A stateful decoder MUST scope cached templates by a caller-supplied session key plus the native observation domain; IPFIX session keys identify the exporter's transport session, not merely its domain or Template ID. | Test |
| `QGI-CLS-0022` | A stateful codec MUST declare finite maxima for sessions, templates per session, bytes per template, cached dependency bytes and events per message before allocation. | Test |
| `QGI-CLS-0023` | A stateful codec MUST provide caller reset of one session or all sessions. | Test |
| `QGI-CLS-0024` | State expiration MUST use caller-supplied time/expiry input, never an internal clock. | Test |
| `QGI-CLS-0025` | Unknown-template data MUST be skipped as `unknown_template` without guessing record boundaries; for IPFIX this skips the remaining Data Set and identifies its Set index and Template ID. | Test |
| `QGI-CLS-0026` | A templated event MUST carry `native.data` (exact record bytes), `native.template` (exact Template or Options Template Record bytes used), `native.header` (all message header fields), `native.set_id`, `native.set_index`, `native.record_index` (zero-based indexes), and `native.message_data` (exact enclosing message bytes, including padding and unrelated Sets). | Test |
| `QGI-CLS-0027` | A templated event using other records to supply common fields MUST carry those records and their templates, headers and Set IDs in `native.dependencies`, in dependency wire order, sufficient to check the event without session state. | Test |
| `QGI-CLS-0028` | IPFIX `native.header` MUST contain unsigned integers `version`, `length`, `export_time`, `sequence_number` and `observation_domain_id` in their RFC 7011 header ranges. | Test |
| `QGI-CLS-0029` | IPFIX template management MUST follow RFC 7011 §§8–8.4: process received Sets in order, treat identical UDP Template IDs/definitions as refreshes, replace changed definitions for the same scoped ID, and ignore UDP withdrawals. | Test |
| `QGI-CLS-0030` | An IPFIX codec MUST NOT infer UDP export order from arrival order or a sequence number; it MUST declare its template lifetime/reordering policy and avoid retroactively reinterpreting emitted records. | Test |
| `QGI-CLS-0031` | A publish/subscribe event MUST use `native.format: "mqtt_publish"`. | Test |
| `QGI-CLS-0032` | An IPFIX record event MUST use `native.format: "ipfix_record"`. | Test |

For IPFIX, [RFC 7011 §§8.2, 8.4 and 9.3](https://www.rfc-editor.org/rfc/rfc7011.txt)
distinguish export-time validity from arrival order. This profile skips unknown-template Sets
immediately instead of buffering them; a later template does not recover a previous skip. Identical
IDs in different sessions/domains are unrelated. Sequence numbers count data records, not messages.
The UDP replacement rule is not permission to infer the template of an earlier datagram. An annex
must resolve reordering and lifetime policy explicitly; neither broker nor exporter networking
belongs inside this pure codec. Each `native.dependencies` entry is an object containing base64
`data`, base64 `template`, `header` in the same class shape, and `set_id`. An absent dependency
array means there were no supporting records; it cannot conceal evidence used to populate fields.

## 5. The official families

Each family's full mapping (native message → kind, field by field, both directions) is an annex.

The first release covers these seven families. The first six retain their delivery order; `qgi`
adds event carriage using the existing datagram and stream classes. Annex letters are stable.

| Order | Class | Family | Annex |
|---|---|---|---|
| 1 | Binary datagrams over UDP | WSJT-X (`wsjtx`) | B |
| 2 | XML datagrams over UDP, External UDP Broadcasts | N1MM Logger+ (`n1mm`) | D |
| 3 | Request and response, XML-RPC over HTTP | fldigi (`fldigi`) | E |
| 4 | Newline-delimited JSON over a TCP stream | JS8Call-improved (`js8call`) | C |
| 5 | Publish/subscribe over MQTT | PSK Reporter live feed (`pskreporter-mqtt`) | G |
| 6 | Templated binary IPFIX over UDP | PSK Reporter reception reports (`pskreporter-ipfix`) | H |
| 7 | Event JSON over UDP or NDJSON over TCP | QG Interop (`qgi`) | I |


| ID | Requirement | Verify |
|---|---|---|
| `QGI-WSJ-0001` | **WSJT-X** (`wsjtx`) MUST follow [Annex B](annex-b-wsjtx.md), mapping revision 1, for message types 0–16 and schemas 1–3 at released source `v3.0.2`. | Test |
| `QGI-WSJ-0002` | **JTTY** (WSJT-X 3.2) is mapped against the 3.2 release's source once released, not its release candidate. | Test |
| `QGI-JS8-0001` | **JS8Call** (`js8call`) is reserved for the JS8Call-improved newline-delimited JSON API over a TCP stream, pinned when Annex C is written; the mapping MUST compare the original v2.3.1 then. Messages in WSJT-X format use `wsjtx`. | Test |
| `QGI-N1M-0001` | **N1MM Logger+** (`n1mm`): the XML messages of its External UDP Broadcasts appendix (application, radio, contact add, change and delete, lookup, spot, score) and the inbound `radio_setfrequency`, pinned to a dated copy of the appendix and its examples. Annex D. | Test |
| `QGI-FLD-0001` | **fldigi** (`fldigi`) MUST follow Annex E when completed; a codec MUST NOT assume a method exists without the caller-supplied build capability list. | Test |

| ID | Requirement | Verify |
|---|---|---|
| `QGI-PSM-0001` | **PSK Reporter MQTT** (`pskreporter-mqtt`) is reserved for the live spot feed in Annex G; no broker transport is part of the codec. | Inspection |
| `QGI-PSI-0001` | **PSK Reporter IPFIX** (`pskreporter-ipfix`) is reserved for reception reports in Annex H, using RFC 7011 over UDP and the published PSK Reporter information elements. | Inspection |
| `QGI-QGI-0001` | **QG Interop events** (`qgi`) MUST follow the event-carriage profile in Annex I, mapping revision 1. | Test |

## 6. Families and fields defined by others

| ID | Requirement | Verify |
|---|---|---|
| `QGI-EXT-0001` | **Anyone may define a family** without changing this specification, by publishing its mapping in the form of §5 with its own test vectors (§7). | Inspection |
| `QGI-EXT-0002` | **Extension family names** MUST use `x-` followed by a reversed domain name controlled by the definer, a dot and a name (for example, `x-org.example.myfamily`), using lowercase ASCII letters, digits, dots and hyphens with nonempty dot-separated components. | Test |
| `QGI-EXT-0003` | **Extension fields** MUST use the same controlled reversed-domain prefix as `QGI-EXT-0002`; unknown extension fields follow `QGI-SER-0005`. | Test |
| `QGI-EXT-0004` | **Adoption.** A MINOR release MAY adopt a family under an official name; the former extension name MUST remain an accepted alias. | Inspection |
| `QGI-EXT-0005` | **Stable names.** A published family name MUST NOT be reused for another meaning. | Inspection |

## 7. Test vectors and conformance

| ID | Requirement | Verify |
|---|---|---|
| `QGI-VEC-0001` | **Versioned suite.** A specification release MUST publish its schema and positive, negative and compatibility vectors identifying base version, family revision, supplied input/context and expected outcome. | Test |
| `QGI-VEC-0002` | **Vectors are produced independently of any one implementation** (from the family's published definition) and, where possible, checked against captures from the real peer. | Inspection |
| `QGI-VEC-0003` | **Codec conformance.** A codec MUST meet every normative requirement applicable to its declared family and pass its versioned vectors in both defined directions; passing vectors alone does not excuse a requirement violation. | Test |
| `QGI-VEC-0004` | **Producer conformance.** A producer MUST emit `qgi` native messages carrying events that validate against their declared version's schema and meet all applicable specification requirements, including version compatibility and Annex I's framing and bounds; the suite's positive and negative carriage vectors establish the sending contract. A producer need not decode another family's wire protocol to conform. | Test |
| `QGI-VEC-0005` | **Consumer conformance.** A consumer MUST meet applicable normative requirements, read the suite’s positive events, reject invalid events, and implement both directions of version compatibility as demonstrated by the compatibility vectors. | Test |
| `QGI-VEC-0006` | **Independent families.** A family defined by others MUST publish its own vectors against the base version it names; it need not republish for each later base version. | Inspection |
| `QGI-VEC-0007` | **Informative view comparison.** A vector runner MUST compare the normative event with `native.message` omitted and, when a view is emitted and an `expected_message` oracle is supplied, compare that view separately as parsed values. | Test |

## 8. Security considerations

| ID | Requirement | Verify |
|---|---|---|
| `QGI-SEC-0001` | **Identity.** An event MUST NOT be treated as authentication of its sender; authenticated transport, when present, is an application boundary outside the codec. | Inspection |
| `QGI-SEC-0002` | **All text is untrusted** (`QGI-MOD-0011`). An application that displays text in a terminal, a web page or any interpreter must neutralize it for that medium. | Inspection |
| `QGI-SEC-0003` | **A codec must be safe on any input:** bounded memory and time, and no failure beyond an error result, for every byte sequence. | Test |
| `QGI-SEC-0004` | **`control` events can transmit.** WSJT-X's Reply, HaltTx and FreeText, N1MM's `radio_setfrequency` and fldigi's setters change what a station does on the air; an application enables each deliberately. | Inspection |
| `QGI-SEC-0005` | **Pure codecs.** A codec MUST perform no I/O: no socket, file, process, clock, environment or logging access. | Inspection |
| `QGI-SEC-0006` | **XML isolation.** An XML codec MUST NOT resolve external entities, DTDs, schemas, files or network resources, and MUST reject a document containing a DOCTYPE as `malformed`. | Test |
| `QGI-SEC-0007` | **Parser bounds.** Every family MUST declare bounds on nesting depth, element count, attribute count, numeric token length and decoded text expansion, or mark each inapplicable for a fixed binary format. | Inspection |
| `QGI-SEC-0008` | **Stream recovery.** A stream family MUST declare a maximum frame size and a resynchronization boundary after malformed or oversized input, including delimiter/escape handling or caller reset when no safe boundary exists. | Test |
| `QGI-SEC-0009` | **Context trust.** An application MUST treat serialized context from storage or another peer, including a carried `qgi` event's context on arrival, as untrusted unless it trusts that boundary. | Inspection |
| `QGI-SEC-0010` | **Consistency.** An observed event whose mapped fields or envelope disagree with its native evidence MUST be rejected as `invalid_value`; an informative `native.message` MUST NOT override that evidence. For `qgi` evidence, comparison uses Annex I's context replacement, native wrapping and compatible-reading rules, rather than the carried family's wire decoder. | Test |
| `QGI-SEC-0011` | **Delivery semantics.** This specification promises no ordering, deduplication or replay protection; applications acting on controls or QSO changes MUST handle those properties themselves. | Inspection |


## 9. Open profile questions (informative)

These are narrow readings to resolve when the placeholder annexes are pinned; they do not block
the completed WSJT-X profile or authorize guessing:

1. How should fldigi `send_text` with `transmit: true` be represented when text insertion and
   transmission require separate XML-RPC requests: caller orchestration or a rejected combination?
2. Should MQTT receiver software remain a separate metadata event, or should a later class revision
   allow a bounded join carrying both PUBLISH envelopes as evidence? The documented spot example
   does not supply a software member, and receiver metadata is a separate retained publication.
3. Which receiver Options Data Record kind and association rule should Annex H use without turning
   receiver-only metadata into a fictitious reception? How do PSK Reporter's older examples differ
   from RFC 7011 for field naming, Options scope and padding?
4. What explicit template lifetime and reordering policy should Annex H pin, with caller-provided
   time and session resets? No network-order inference or silent template eviction is allowed.

## Annexes

The requirements in Annexes A, B, H and I are normative; their tables are incorporated by reference.
H and I remain placeholders for the profile work they explicitly leave open.
Annex F is the machine-readable event shape; parsing and mapping rules also apply. Its open string
registries accept additions from newer MINOR versions. A schema check alone cannot establish that
a family name is registered, a native version or mapping is supported, a JSON string contains only
valid Unicode text, a time resolves at native precision, or the native/common views agree. The
applicable codec and consumer checks establish those properties. Common-field schemas apply to
recognized official families; a third-party family publishes its profile constraints with its vectors.

- **A.** [Common fields](annex-a-fields.md).
- **B.** [WSJT-X mapping, revision 1](annex-b-wsjtx.md).
- **C.** [JS8Call-improved placeholder](annex-c-js8call.md).
- **D.** [N1MM Logger+ placeholder](annex-d-n1mm.md).
- **E.** [fldigi placeholder](annex-e-fldigi.md).
- **F.** [JSON Schema 2020-12](schema/qgi-event-0.1.0.schema.json).

- **G.** [PSK Reporter MQTT placeholder](annex-g-pskreporter-mqtt.md).
- **H.** [PSK Reporter IPFIX placeholder](annex-h-pskreporter-ipfix.md).
- **I.** [QG Interop event-carriage placeholder](annex-i-qgi.md).
- **Z.** [Informative extension design check](annex-z-design-check.md).

The [vector format and coverage](vectors/README.md) describe the versioned suite. Annexes C–E and G
reserve names and scope only and make no claim that those family codecs conform yet.
Annex H additionally records submitter obligations; Annex I defines carriage and vectors now,
without claiming reference-implementation or live-peer conformance.
