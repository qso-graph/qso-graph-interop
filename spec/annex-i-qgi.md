<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex I: QG Interop event-carriage placeholder, revision 1

Official family: `qgi`. The native message is one event JSON object. This placeholder defines the
carriage contract and vectors now; transport-helper implementation and interoperability proofs
remain Target. Its published definition is this specification version, not an outside protocol.
The [manifest](vectors/qgi/manifest.json) pins this annex by SHA-256 (`QGI-MAP-0001`).

| ID | Requirement | Verify |
|---|---|---|
| `QGI-QGI-0002` | **Framing.** Over UDP a message MUST be one complete event object per datagram, using `QGI-CLS-0001`, with at most 65,507 payload bytes. Over TCP it MUST be NDJSON (`QGI-SER-0004`, `QGI-CLS-0002`), with LF or CRLF delimiters and at most 1,048,576 UTF-8 bytes per record excluding its delimiter. `native.format` MUST be `qgi_datagram` or `qgi_ndjson`, respectively; `native.data` MUST preserve the exact received object bytes, including whitespace but excluding the TCP delimiter. | Test |
| `QGI-QGI-0003` | **Reading.** A codec MUST parse UTF-8 JSON with duplicate-member rejection at every object depth (`QGI-SER-0007`), require one object with no trailing content except JSON whitespace, and check the carried event against its declared schema and specification version rules (`QGI-GEN-0005`–`0007`, `QGI-SER-0005`). A different MAJOR MUST return `unsupported_version`; invalid JSON/UTF-8 or duplicate members MUST return `malformed`; a schema or semantic violation MUST return `invalid_value`. A newer MINOR of the same MAJOR MUST use compatible reading, including reading an unknown kind as `native`, rather than rejecting it for being newer. | Test |
| `QGI-QGI-0004` | **One observed event.** Successful decoding MUST yield the carried event's recognized members, retaining its `spec`, `family`, `mapping`, `wire_id` and `native_version` when present, subject to compatible reading; it MUST replace `context` with the observer-supplied context and replace `native` with the received `qgi` evidence. If the carried event has its own `native`, the codec MUST keep that object unchanged as `native.carried_native`; otherwise it MUST omit `carried_native`. This member is normative evidence, not an informative `message` view. Nesting MUST be limited to one level: the outer `native` may contain `carried_native`, but that nested object MUST NOT itself contain `carried_native`; a deeper input MUST return `invalid_value`. A codec MUST NOT recursively decode base64 evidence as another `qgi` event. | Test |
| `QGI-QGI-0005` | **Context and command trust.** The carried context MUST remain available in the exact received bytes and MUST be treated as untrusted on arrival (`QGI-SEC-0009`); it MUST NOT supply observer identities. A carried command becomes an observed `control` with received native evidence, not an authorized action (`QGI-MOD-0008`). | Test, Inspection |
| `QGI-QGI-0006` | **Bounds and recovery.** A codec MUST check message size before parsing; it MUST bound JSON container depth at 32 (root object is depth 1), total object members plus array elements at 65,536, and numeric tokens at 128 UTF-8 bytes. Each decoded string MUST be at most the transport's frame-byte limit in UTF-8; XML element/attribute bounds are inapplicable. A bound violation MUST return `oversized` atomically. A TCP reader MUST discard a malformed or oversized record through the next LF before resuming, treating an immediately preceding CR as part of the delimiter; at EOF an unfinished record MUST return `truncated`. | Test |
| `QGI-QGI-0007` | **Writing.** A producer MUST serialize a conforming event as one `qgi` message within `QGI-QGI-0002` and `0006`'s bounds, omitting literal line breaks within a TCP record (escape them in strings). Event emission is distinct from encoding the carried family's command or replaying its native evidence. Re-encoding a `qgi` observation MUST recover its exact `native.data`, adding the caller-selected LF or CRLF delimiter only for TCP. | Test |

The input's family describes the event's meaning; selecting the `qgi` codec describes its carriage.
Decoding does not relabel a carried WSJT-X decode as `family: "qgi"`. An event authored with QG
Interop's own semantics may use `family: "qgi"`, mapping 1, the core kinds and Annex A fields;
there are no additional family-specific operations in this revision. The observation/command
schema distinction still applies to the carried object before wrapping. The outer observation
uses the carriage format to check evidence (`QGI-SEC-0010`); the original native object and its
class-specific members stay together in `carried_native`. Rewrapping an observation that already
contains `carried_native` exceeds the one-level bound. Bytes inside evidence remain opaque.

For check precedence (`QGI-MAP-0007`): message size, JSON framing/duplicate detection (enforcing
parser resource bounds while reading), declared specification version, then schema and semantic
checks. Compatible readers validate the
known requirements of the same-MAJOR schema; producers validate against their own declared
version. Unknown fields are omitted from the recognized view, with the original bytes retained.
Unsupported mapping revisions follow the core's never-guess rule. This profile adds no implicit
replies, routing, acknowledgments, authentication or permission to act.

The vectors cover carried observations and commands, both transport boundaries, native retention,
untrusted context replacement, errors and a hypothetical newer-MINOR kind. Remaining profile work:
capture-based interoperability evidence and future family-specific event/operation additions,
if any. No external source is needed for the present event-carriage contract.
