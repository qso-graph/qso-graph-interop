<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex I: QG Interop event carriage, revision 1

Official family: `qgi`. Its published definition is this specification version, with no outside
protocol pin. The [manifest](vectors/qgi/manifest.json) pins this annex by SHA-256
(`QGI-MAP-0001`). Transport implementation and live interoperability remain targets.

| ID | Requirement | Verify |
|---|---|---|
| `QGI-QGI-0002` | **Framing.** Over UDP a message MUST be one complete event JSON object per datagram, at most 65,507 payload bytes. Over TCP it MUST be NDJSON with LF or CRLF delimiters and at most 1,048,576 UTF-8 bytes excluding the delimiter. Unlike byte-preserving native classes, carriage MUST use `QGI-QGI-0004`'s semantic evidence boundary; it MUST NOT create a native object for the received JSON. | Test |
| `QGI-QGI-0003` | **Reading.** A codec MUST parse UTF-8 JSON, reject duplicate members at every depth, require one object with no trailing content except JSON whitespace, and use the reader profile (`QGI-SER-0009`) and compatibility rules. A different MAJOR MUST return `unsupported_version`; invalid JSON, invalid UTF-8, strings or keys with unpaired Unicode surrogates, and duplicate members MUST return `malformed`; schema or semantic violations MUST return `invalid_value`. An escaped valid surrogate pair represents its Unicode scalar, not an error. Compatible reading supplies a separate recognized view; it MUST NOT mutate the preserved carried event. | Test |
| `QGI-QGI-0004` | **Composable carriage.** Successful decoding MUST yield exactly the carried event unchanged, with one entry appended to its optional envelope `via` array (create that array when absent). The entry MUST contain only `family: "qgi"` and the receiving observer's supplied `context`. The carried event's own `context`, `native`, command/observation distinction, unknown members and all existing hops MUST remain unchanged. An event already carrying eight hops MUST fail atomically as `invalid_value`. The carried event itself is the evidence; no byte-exact copy of the received JSON is retained. Comparison MUST use `QGI-SER-0008`. | Test |
| `QGI-QGI-0005` | **Trust.** The carried context and every received hop MUST be treated as untrusted on arrival (`QGI-SEC-0009`). Only the appended hop records the current observer's context. A carried command remains a command and supplies no authorization to encode, transmit or act (`QGI-MOD-0008`). | Test, Inspection |
| `QGI-QGI-0006` | **Bounds and recovery.** A codec MUST check frame size before parsing, bound container depth at 32 (root object depth 1), total object members plus array elements at 65,536, and numeric tokens at 128 UTF-8 bytes. Decoded strings MUST be no larger than the transport's frame-byte limit in UTF-8. XML bounds are inapplicable. Admission MUST check the resulting event, after appending the new hop under `QGI-QGI-0004`, against every parser limit above and the compact record bound in `QGI-SER-0004`; any resource violation MUST return `oversized` atomically, with no event or state update. An input within the parser limits is insufficient if its resulting event exceeds them and cannot meet `QGI-QGI-0007`. A TCP reader MUST discard a malformed or oversized record through the next LF (preceding CR belongs to the delimiter); EOF in an unfinished record MUST return `truncated`. | Test |
| `QGI-QGI-0007` | **Writing.** A producer MUST serialize the event as one conforming message within the framing and parser bounds, escaping literal line breaks in TCP strings. Writing or forwarding MUST NOT add a hop; only reception does. Re-encoding the decoded event carries its accumulated hops and original evidence semantically, with the caller-selected TCP delimiter, and MUST NOT promise recovery of JSON whitespace, ordering or escape spelling. It is distinct from encoding a carried family's command or replaying that family's native bytes. | Test |
| `QGI-QGI-0008` | **Own semantics and precedence.** An event authored with `family: "qgi"`, mapping 1, MUST use the core kinds and Annex A fields; this revision defines no extra operations or radio mapping. A carried other-family event MUST retain its family. Check order MUST be frame size, JSON framing and parser bounds, declared version, schema and semantic checks (including hop count), then resulting-event admission under `QGI-QGI-0006`. The carriage step MUST preserve native bytes without interpreting them; original-family consistency checks and their separate verification reports follow `QGI-MAP-0015` when that decoder is supported. An unavailable original-family decoder MUST NOT yield a verified report. | Test |

The output budget is an admission condition on the resulting compact record, rather than a second
base64 copy of the input. For a compact carried event of size S and a new context of compact size C,
appending to a nonempty `via` costs C + 28 bytes; creating `via` costs C + 36 bytes; an existing empty
array costs C + 27 bytes. Thus S + C + 36 is a conservative bound, at most S + 4,132 under
`QGI-SER-0010`. The UDP maximum of 65,507 easily fits. A TCP record can reach the full 1 MiB frame
limit, but admission depends on its compact output: the conservative largest S guaranteed to fit
any allowed new context is 1,044,444 bytes. Larger records can still fit with a smaller context or
compaction of whitespace; the exact measured output decides. The boundary vectors give exactly
1,048,576 output bytes and one additional byte with the same receiving context.

Parser limits also apply after appending the hop. Creating `via` with a context containing only
`direction` and `exchange` adds six object members and array elements in total. The element-boundary
vectors produce exactly 65,536 elements (admitted) and 65,537 (rejected). A separate input with
exactly 65,536 elements is rejected because its resulting event would contain 65,542.

The vectors cover observations and commands, two and eight successful hops, ninth-hop failure,
unchanged original native evidence and context, both delimiters, compatibility, invalid Unicode,
parser bounds and atomic output-budget failure. They are synthetic, not live-peer captures.
