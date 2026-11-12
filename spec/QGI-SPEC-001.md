<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QGI-SPEC-001: the QG Interop specification

**Status: draft, 2026-11-12. Not yet implementable:** the annexes, schema and vectors are unwritten,
and the review items in issue #5 are open. Nothing here is a promise of compatibility until the
first numbered release of this specification.

From KI7MT's direction of 2026-11-11: QG Interop **defines a specification, as ADIF does, and others
implement it**. QSO Graph proves it with a reference implementation for WSJT-X, JS8Call-improved,
N1MM Logger+ and fldigi, and delivers that implementation in a CLI, a simulator and QSO Graph Desktop
(`QGI-RI-001`).

**This document names no programming language, library or operating system.** Conformance is
established by the test vectors (§7), not by how an implementation is built.

## 0.1 How this document works

**One fact, one place.** Every normative statement lives here once, with an ID; other documents
cite it. The words MUST, MUST NOT, SHOULD and MAY are used as RFC 2119 defines them.

| Area | Covers |
|---|---|
| `QGI-GEN` | Purpose, terms, conformance classes, versioning of this specification |
| `QGI-MOD` | The event model: envelope, kinds, fields, value types |
| `QGI-SER` | The JSON representation |
| `QGI-MAP` | Mapping rules common to every family |
| `QGI-WSJ`, `QGI-JS8`, `QGI-N1M`, `QGI-FLD` | The four official family mappings |
| `QGI-EXT` | Families and fields defined by others |
| `QGI-VEC` | Test vectors and conformance |
| `QGI-SEC` | Security considerations |

**Verify methods:** `Test` (a test vector or automated check), `Inspection` (read against this text).

## 1. General

| ID | Requirement | Verify |
|---|---|---|
| `QGI-GEN-0001` | **Purpose: many to many.** Amateur radio station programs report what they do (decodes, radio state, logged QSOs, spots, scores) and accept some commands, each over its own protocol and transport (UDP datagrams; request and response over TCP). Without a common model, every application must learn every program: N programs and M applications need N × M integrations. This specification defines **one event model** and **the mapping from each program's protocol into it**, so each program is mapped once and each application implements the model once: N + M. Anyone may add a program's protocol (§6). | Inspection |
| `QGI-GEN-0002` | **Terms.** A *family* is one program's protocol (WSJT-X's UDP messages, N1MM's XML broadcasts). A *native message* is one message of a family, exactly as sent. An *event* is the specification's representation of a native message: a kind, its common fields, and the native message itself. | Inspection |
| `QGI-GEN-0003` | **Conformance classes.** A **codec** for a family turns native messages into events and, where the family defines a sending direction, events back into native messages. A **consumer** reads events. A **producer** writes events. A claim of conformance names its class, its families and this specification's version. | Inspection |
| `QGI-GEN-0004` | **Protocol, not policy.** This specification defines what a message means, not what an application does with it. Who may send, which transports are enabled, what is displayed, stored or logged, and any content rule stricter than the family's are the application's decisions. | Inspection |
| `QGI-GEN-0005` | **Versioning.** This specification is versioned `MAJOR.MINOR.PATCH`. A MINOR version adds kinds, fields or families and MUST NOT change the meaning of existing ones; a consumer of version `x.y` MUST accept events from any `x.z`, ignoring what it does not know. A MAJOR version may change meaning and says how. Every event carries the version it was written to. | Test |

## 2. The event model

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MOD-0001` | **Envelope.** Every event has: `spec` (version), `family` (registered name, §6), `family_version` (the native protocol's pinned version or schema), `kind`, `wire_id` (the native message's own client or instance identifier, if it has one), `context`, the kind's common fields, and `native`. | Test |
| `QGI-MOD-0002` | **Context is supplied by whoever observed the message, never inferred from it:** `direction` (`from_application`, `to_application`, `unknown`), `source_id` and `target_id` (the observer's names for the endpoints), `exchange` (`notification`, `request`, `response`, `unknown`), and `correlation_id` for request and response pairs. | Test |
| `QGI-MOD-0003` | **Kinds.** `decode`, `radio_state`, `qso_logged`, `qso_changed`, `qso_deleted`, `spot`, `score`, `control`, `lifecycle`, `ambiguous` (the family allows two directions and context does not say which), and `native` (no common view). A consumer MUST treat an unknown kind as `native`. | Test |
| `QGI-MOD-0004` | **Common fields per kind** are listed in Annex A. A field is present only when the native message carries it; a field the native message lacks is absent, never invented or derived from free text (a callsign is not parsed out of a decode's message text). | Test |
| `QGI-MOD-0005` | **Absent, null and empty are distinct** wherever the family distinguishes them, and the event preserves the distinction. | Test |
| `QGI-MOD-0006` | **Value types.** Frequency: integer hertz. Time: a UTC value only where the native value resolves to one, with the precision it carries; a native time that does not (local, zoned, invalid or null) leaves the common field absent, and the native representation is always kept in `native`. (Review item: separate types for an instant, a time of day and an unresolved time.) Signal report: integer dB where the family gives one. Text: §2.1. Booleans, integers and floating-point values keep the native value exactly (§3). | Test |
| `QGI-MOD-0007` | **No lossy round trip (KI7MT, 2026-11-12).** `native` keeps the native message exactly as received, including fields this specification does not map and trailing data a newer native version added, so the original is always available. The common fields may canonicalize a value (a WSJT-X boolean sent as `2` reads as true), and each family's annex lists every such canonicalization; nothing is ever lost, because the original stays in `native`. | Test |
| `QGI-MOD-0008` | **Reading never acts.** Mapping a native message to an event never changes `native` and never sends anything. Encoding an event into a native message is an explicit operation, and whether to send the result is the application's decision (`QGI-GEN-0004`); the event's kind is not an authorization. | Inspection |

### 2.1 Text

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MOD-0010` | **Text is carried exactly as the family sent it.** A codec MUST NOT strip, replace, normalize or truncate characters. Text that is not valid in its declared encoding is carried as its raw code units. | Test |
| `QGI-MOD-0011` | **All text in an event originates from a station program or another operator and is untrusted** (§8). This specification does not define which characters an application may display; that is application policy (`QGI-GEN-0004`). | Inspection |

## 3. JSON representation

| ID | Requirement | Verify |
|---|---|---|
| `QGI-SER-0001` | **An event's interchange form is one JSON object (RFC 8259, UTF-8)**, valid against the JSON Schema published with each version of this specification. | Test |
| `QGI-SER-0002` | **Text that is valid UTF-8** is a JSON string. **Text that is not** is `{"bytes": [...]}` (UTF-8 code units) or `{"utf16": [...]}` (UTF-16 code units), as the family declared it; null is `{"bytes": null}` where the family has a null distinct from empty. | Test |
| `QGI-SER-0003` | **Binary64 floating point:** a finite value is a JSON number written so that a correctly rounding reader recovers the same bits; a non-finite value is the string `ieee754:` followed by 16 hexadecimal digits of its bits, preserving NaN payloads. A consumer that cannot read numbers with correct rounding MUST use the native message instead. | Test |
| `QGI-SER-0004` | **A sequence of events** is newline-delimited JSON, one event per line. | Test |
| `QGI-SER-0005` | **A consumer MUST ignore members it does not know**, so that MINOR versions are compatible (`QGI-GEN-0005`). | Test |

## 4. Mapping rules for every family

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MAP-0001` | **Every family's mapping is pinned to released source or a dated published document**, named with its version and a cryptographic hash, and re-pinned only by a new revision of the family's section. | Inspection |
| `QGI-MAP-0002` | **Never guess.** A native message of an unknown version or type is reported as skipped, with its reason, and is not interpreted. A codec MUST NOT infer a field's encoding from a newer version it does not know. | Test |
| `QGI-MAP-0003` | **Limits are declared per family** (largest message, largest field) and checked before a codec allocates; a message beyond them is an error, not a truncation. | Test |
| `QGI-MAP-0004` | **No identity from the wire.** A wire ID, an application or station label, or a network address is data, not proof of who sent the message, and never fills `source_id`. | Inspection |
| `QGI-MAP-0005` | **Malformed input is an error, never a partial event.** | Test |

## 5. The official families

Each family's full mapping (native message → kind, field by field, both directions) is an annex.

| ID | Requirement | Verify |
|---|---|---|
| `QGI-WSJ-0001` | **WSJT-X** (`wsjtx`): every message of `Network/NetworkMessage.hpp` at release `v3.0.2` (identical at 2.7.0), schemas 1 to 3, both directions. Annex B. | Test |
| `QGI-WSJ-0002` | **JTTY** (WSJT-X 3.2) is mapped against the 3.2 release's source once released, not its release candidate. | Test |
| `QGI-JS8-0001` | **JS8Call** (`js8call`): the JSON API of the JS8Call-improved line, over UDP and TCP, pinned to a release. How far the original JS8Call (last release v2.3.1) differs is checked when this family is mapped, and any difference is covered as an older version. Messages JS8Call sends in WSJT-X's format are the `wsjtx` family, already mapped. Annex C. | Test |
| `QGI-N1M-0001` | **N1MM Logger+** (`n1mm`): the XML messages of its External UDP Broadcasts appendix (application, radio, contact add, change and delete, lookup, spot, score) and the inbound `radio_setfrequency`, pinned to a dated copy of the appendix and its examples. Annex D. | Test |
| `QGI-FLD-0001` | **fldigi** (`fldigi`): XML-RPC method calls and replies as fldigi's method reference defines them. A request is a `control` event `to_application` and its reply shares its `correlation_id`. The method list varies by build; a codec accepts the build's own list and never assumes a method exists. Annex E. | Test |

## 6. Families and fields defined by others

| ID | Requirement | Verify |
|---|---|---|
| `QGI-EXT-0001` | **Anyone may define a family** without changing this specification, by publishing its mapping in the form of §5 with its own test vectors (§7). | Inspection |
| `QGI-EXT-0002` | **Family names:** the official names above are reserved. Others use `x-<owner>-<name>`, where `<owner>` is a domain or project name the definer controls, as ADIF reserves `APP_<program>_` for application-defined fields. | Test |
| `QGI-EXT-0003` | **Extra common fields** a definer needs use the same `x-<owner>-` prefix; a consumer ignores those it does not know (`QGI-SER-0005`). | Test |
| `QGI-EXT-0004` | **A family in wide use may be adopted as official** in a MINOR version, under a reserved name, with its earlier `x-` name accepted as an alias. | Inspection |

## 7. Test vectors and conformance

| ID | Requirement | Verify |
|---|---|---|
| `QGI-VEC-0001` | **Each family publishes test vectors with each version of this specification:** native message bytes, the expected event JSON, and for families with a sending direction, the reverse. Negative vectors (truncated, oversized, unknown version, unknown type, malformed) state the expected skip or error. | Test |
| `QGI-VEC-0002` | **Vectors are produced independently of any one implementation** (from the family's published definition) and, where possible, checked against captures from the real program. | Inspection |
| `QGI-VEC-0003` | **A codec conforms for a family when it passes every vector for that family**, both directions where defined; a consumer conforms when it reads every vector's event and ignores unknown members and kinds as §3 and §2 require. | Test |

## 8. Security considerations

| ID | Requirement | Verify |
|---|---|---|
| `QGI-SEC-0001` | **Nothing in an event authenticates its sender.** The family protocols this specification maps have no authentication; an application that acts on events, and above all one that sends `control` events, must decide whom it trusts by its own means. | Inspection |
| `QGI-SEC-0002` | **All text is untrusted** (`QGI-MOD-0011`). An application that displays text in a terminal, a web page or any interpreter must neutralize it for that medium. | Inspection |
| `QGI-SEC-0003` | **A codec must be safe on any input:** bounded memory and time, and no failure beyond an error result, for every byte sequence. | Test |
| `QGI-SEC-0004` | **`control` events can transmit.** WSJT-X's Reply, HaltTx and FreeText, N1MM's `radio_setfrequency` and fldigi's setters change what a station does on the air; an application enables each deliberately. | Inspection |

## Annexes (to be written per family)

- **A.** Common fields per kind, with types.
- **B.** WSJT-X mapping (from the reference implementation's `docs/PROTOCOL.md` and `docs/NORMALIZATION.md`).
- **C.** JS8Call-improved mapping. **D.** N1MM Logger+ mapping. **E.** fldigi mapping.
- **F.** The JSON Schema.
