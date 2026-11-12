<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Protocol evidence and behavior

## Released source pins

| Application | Released tag | Commit | Definition |
| --- | --- | --- | --- |
| WSJT-X | `v3.0.2` | `ccdfaf3c1c109010d15399674ce278167cfde848` | [Network/NetworkMessage.hpp](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp) |

WSJT-X was fetched from its official repository, `https://github.com/WSJTX/wsjtx` (where WSJT-X 3 is
released; the older SourceForge repository ends at 2.7.0), using the released tag `v3.0.2`. Its
`Network/NetworkMessage.hpp` and `Network/NetworkMessage.cpp` are byte for byte the same as at
`wsjtx-2.7.0` (SourceForge, commit `b4f9a431bcf6449df8f37b56de79d48b665b044c`), so this library
implements WSJT-X 2.7.0 and 3.0.2 alike. The annotated tag object is
`d2d9c46a1e5061a8ab489ebf473e71fa070dc4e7`.

SHA-256 of the definitions:

```text
fd73b7f92fc0f66ea8f12a147b59a6072408c24656735d59df772dc55a5eeb62  WSJT-X NetworkMessage.hpp
```

Qt's stream operators establish types beyond the headers' abbreviated description. These are
pinned to Qt `v5.4.2`: [QDate/QTime/QDateTime](https://github.com/qt/qtbase/blob/v5.4.2/src/corelib/tools/qdatetime.cpp),
[QTimeZone](https://github.com/qt/qtbase/blob/v5.4.2/src/corelib/tools/qtimezone.cpp),
and [QColor](https://github.com/qt/qtbase/blob/v5.4.2/src/gui/painting/qcolor.cpp).
The implementation is Rust; upstream C++ is not compiled or shipped here.

## Specification and public API

The implementation follows [QGI-SPEC-001 0.1.0](../spec/QGI-SPEC-001.md), Annex A's common
fields, Annex B revision 1 for WSJT-X, and Annex I revision 1 for event carriage. The source pins
above remain the native-layout evidence. No upstream source was fetched during this alignment.

The family-independent core exports `Event`, `Context`, neutral text/integer/time/binary64 values,
radio labels, hops, class-native evidence, `Codec`, `Profile`, outcomes, skips, limits, and errors.
It imports no family. Default features select `wsjtx`, `qgi`, and `serde`; disabling default features
builds and tests the core alone. `conformance` exports the vector runner and hash, round-trip,
truncation, limit and fuzz helpers. The `serde` feature enables serde traits on neutral values and events. The core retains serde_json
for the preserved JSON value model and bounded JSON format, including with serde integration off.

`Codec::decode` returns an ordered collection of events and skip notices or one atomic error.
The caller supplies a session key, observer context and optional expiry input. `Limits` declares
message, field, event, session, template and dependency budgets. WSJT-X and qgi have no semantic
session state. `reset` resets one session or all sessions. Stream decoding consumes a caller-owned
buffer without an async runtime. The external implementation in `tests/core.rs` proves the open
trait with bounded session state and planted atomic failures.

`encode_command` constructs bytes from common fields, operation, target `wire_id` and encoding
options. `reencode` separately validates observed native evidence and canonicalizes booleans.
`Event::native_bytes` retrieves exact original bytes. Decoding and encoding never perform I/O.
`Profile` supplies family interpretation and producer validation through public core APIs.
Family errors remain typed sources of the non-exhaustive core error. Display messages are prose.

## WSJT-X mapping

All 17 types and schemas 1–3 use Annex B's wire order, optional contiguous tails, radio identifier,
text wrappers, time resolution, integer range and sentinels. Unknown versions stop after schema;
unknown types stop before ID. Admission order is size, magic, schema, type, then body. QString
length checks distinguish oversized bounds, malformed odd counts and truncated payloads.

Observed events preserve exact datagrams, including noncanonical booleans and extensions.
Optional `native.message` uses the Annex B catalog, with separate structured-view comparisons.
The producer profile checks that complete view against the decoded native representation.
`Profile::verify` returns `verified` or `unavailable` separately from the event and applies the
mapping-revision outcome table. Unsupported command mappings are never encoded.

The sending types are 0, 3, 4, 6–9, 11 and 13–16. Schema defaults to 3 and accepts integers 1–3.
Missing arguments or wire IDs, optional-tail holes, incorrect text wrappers, unsupported argument
combinations and wire-range violations return `invalid_value`. The common view never derives
callsigns, locators, observer identity or a WSPR mode from text or IDs.

## Event JSON and carriage

`json::parse` checks the frame limit before allocating. Its bounded container parser delegates
string/Unicode and numeric scalar syntax to serde_json. Building maps explicitly rejects duplicate
members at every depth and preserves ordinary event keys that coincide with serde_json's private
arbitrary-precision-number marker. This small framing layer is needed for those two guarantees and
for resource admission before map/array growth; it does not replace serde's scalar encoding.
`json::parse` is the byte-bounded reader entry point. Serde deserialization of `Event` also delegates
to it after acquiring the raw JSON record from the deserializer.

Carriage containers have depth at most 32, with at most 65,536 total object members and array
elements. The general event reader uses the same depth and number bounds and a 1,048,576-element
bound, implied by its record-byte bound. This permits Annex B's large raw-text structured views
without applying Annex I's separate carriage limit to native observations.
Numeric tokens have at most 128 bytes. Strings are bounded by the transport frame limit. Each
context has a 4,096-byte compact budget. Compact records are admitted at 1,048,576 UTF-8 bytes. Serialization uses a bounded in-memory
serde sink, including the separate context budget, so oversize output never grows an unbounded
output buffer.
Finite binary64 values use correctly rounded numbers; nonfinite values preserve all bits.
Common integers are exact within ±(2^53−1), and unknown numeric additions retain their exact tokens.
Absent, null, empty and invalid text remain distinct. Neutral time values validate UTC spelling
and Gregorian dates. Qt values stay in the WSJT-X module.

Reader validation accepts additions and preserves the complete event; `recognized` projects known
members recursively without rewriting the stored content. Producer validation rejects unknown
members recursively, including contexts, hops, fields, radio, colors, native headers, dependencies
and family-supplied structured views. Native consistency requires an explicit supported family
profile; carriage itself never verifies the original native protocol.

`qgi::Qgi` receives one UDP event or LF/CRLF-delimited TCP record. Reception appends exactly one
observer hop and preserves all prior content. Eight existing hops reject a ninth atomically.
Parser and compact-output limits are reapplied after appending. Forwarding adds no hop and uses
the reader profile for newer preserved content. TCP errors consume through the next LF. If an
oversized incomplete frame has no LF, the codec enters delimiter-discard recovery; reset clears
that framing state. EOF with an unfinished frame returns `truncated`.

## Acceptance and supplemental evidence

`tests/conformance.rs` hashes every complete vector file, checks manifest metadata, executes every
instruction, and checks that the manifest and directory have identical file sets. Unknown classes
fail. The qgi manifest's annex hash is also checked. WSJT-X's source hash is compared with the pin
above. Informative views and original-byte retrieval are tested separately from normative events.

| Family | Positive | Command | Command negative | Negative | Skip | Compatibility | Embedded reencode |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| wsjtx | 165 | 67 | 69 | 80 | 5 | 14 | 81 |
| qgi | 13 | 0 | 0 | 22 | 0 | 0 | 13 |

The manifests contain no standalone `reencode` class today; the exported runner handles it, and
all embedded re-encoding instructions execute. qgi compatibility checks are embedded in positives.
All 435 manifest vectors and 94 embedded re-encoding operations pass.

The old layout/schema/string/date/color/boolean/sentinel/direction fixtures and corresponding
`tests/protocol.rs` tests were retired because the independently reviewed specification vectors
supersede them. Their generator no longer maintains a second protocol oracle. The 120-message
variation corpus remains in `tests/fixtures`, with synthetic callsigns, generated by the independent
standard-library `struct` script. Its sequence, exhaustive-prefix, mode/negotiation helpers and
10,000 randomized finite-binary64 checks use the new event API. Qt primitive validation and
inconsistent-date-time/optional-tail encoder guards remain covered inside the family.

Additional planted cases cover unknown classes and changed manifest bytes, duplicate and reserved
JSON keys, precise unknown integers, null/empty/absent context, neutral value bounds, recursive
producer rejection, partial and multiple TCP frames, EOF/recovery, and typed family errors. A maximum-size invalid UTF-8 observation with its structured view
checks Annex B admission separately from Annex I rejection. The
fuzz harness accepts arbitrary binary, arbitrary event JSON and qgi datagram/stream input through
the public API on nightly-2026-10-30. Live-peer interoperability remains separate evidence.

## Simulator timing and reactions

The simulator's timing evidence comes from released WSJT-X `v3.0.2`, commit
`ccdfaf3c1c109010d15399674ce278167cfde848`, the Annex B pin above. The source was fetched
read-only from the WSJTX GitHub repository; playback itself needs no network fetch. The Python
0.3.1 ALL.TXT reader supplies parsing compatibility evidence, and Desktop's independently
captured Python 0.2.0 datagrams supply the output oracle after callsign substitution.
See [simulator validation](simulator-validation.md) for normalization and planted failures.

All periods below are T/R periods, not the duration of the encoded waveform. Source links
refer to `widgets/mainwindow.cpp` at that exact released commit.

| Mode or recorded label | Established period | ALL.TXT playback and released source |
| --- | --- | --- |
| FT8, FT8_SH, FT8_SF | 15 s | Play ordinary received decodes; SuperFox Tx lines remain skipped. [Period, line 11007](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11007). |
| FT4 | 7.5 s | Play. [Line 10914](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L10914). |
| JT4 | 60 s | Play. [Line 11156](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11156). |
| JT65 | 60 s | Play. [Line 11301](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11301). |
| JT9 | 60 s; fast mode: 5, 10, 15, or 30 s | Skip: the recorded `JT9` name does not distinguish fast mode or its setting. [Lines 11245–11260](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11245). |
| MSK144 | 5, 10, 15, or 30 s | Skip: the selected setting is absent from the log. [Lines 11441–11478](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11441). |
| Q65 | 15, 30, 60, 120, or 300 s | Play only a period-bearing label such as `Q65-60C`; skip bare `Q65`. [Period choices, line 11381](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11381), [label construction, lines 13135–13147](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L13135), and [log writer, lines 15221–15239](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L15221). |
| FST4 | 15, 30, 60, 120, 300, 900, or 1800 s | Skip: the selected setting is absent from the log. [Lines 10854–10856](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L10854). |
| FST4W | 120, 300, 900, or 1800 s | Skip: the selected setting is absent from the log. [Choices, line 566](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L566), [setting handler, lines 13121–13124](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L13121). |
| WSPR | 120 s | Known period, but a separate log and WSPRDecode layout; skip as not an ordinary Decode. [Period, line 11541](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11541), [WSPRDecode, lines 13419–13429](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L13419), [log destination, line 15272](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L15272). |
| Echo | 3 s internal period | Known period, but measurements use a separate layout and `all_echo.txt`; skip as not an ordinary Decode. Automatic measurements are aligned to 6 s. [Period, line 11585](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11585), [measurement formatting, lines 2440–2457](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L2440), [log destination, lines 15273–15275](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L15273). |
| FreqCal | 5, 10, 15, or 30 s | Skip: the selected setting is absent from the log. [Lines 11628–11639](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L11628). |

The [released log writer](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L15211)
records timestamp, dial frequency, Rx/Tx, mode label, and decode text. Only Q65's label embeds
the selected period; timestamp gaps, DT, and defaults do not establish a missing setting.
Q65 label validation follows the released [submode bounds](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L13076):
15 s allows A–C, 30 s A–D, 60 s A–E, 120 s A–F, and 300 s A–E. The label supplies Status
mode `Q65`, its submode letter, and integer period; Decode uses the native `:` marker.
The writer also records [FT8_SH/FT8_SF labels](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/widgets/mainwindow.cpp#L15242)
while the active mode remains FT8. These labels retain the 15 s period, Status mode `FT8`,
and Decode marker `~`; FT8_SF is written only for Tx, which remains skipped.
FT4 uses the header's not-applicable integer T/R period sentinel because 7.5 seconds is not
representable as u32 seconds. Other playable modes retain concrete integer periods.

Unknown names and modes without a recorded selectable period are counted and skipped before
cycle grouping. The summary lists their distinct names in sorted order, escaping non-ASCII and
control characters for terminal output. Received lines before `--from` do not enter the skip
count. If nothing playable remains, the error retains `no received decodes` and appends the
skip summary when modes were skipped. WSPR captures can be played using native NDJSON playback
without converting WSPRDecode into Decode.

All header links below use the exact pin in Annex B:

| Reaction | Header evidence |
| --- | --- |
| Replay: stored Decodes in order, New false, then Status | [lines 329–341](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L329) |
| Status on supported state changes, including decoding start/end and changed Rx offset, DX fields, mode, or transmitted message | [lines 166–189](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L166) |
| Halt Tx: disable Auto Tx, or halt immediately; changed state uses Status | [lines 344–350](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L344) |
| Free Text: stage/clear text, or select staged text unchanged; no standalone acknowledgment | [lines 353–381](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L353) |
| Configure: supported settings, empty/no-change sentinels, silently ignored invalid values; changed state uses Status | [lines 488–508](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L488) |
| Clear: erase selected windows; no acknowledgment | [lines 235–253](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L235) |
| Close: request graceful shutdown, send Close before exit | [lines 321–326](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L321) |
| Location: change session locator | [lines 411–425](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L411) |
| Heartbeat: exchange supported schema | [lines 109–138](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp#L109) |

The simulator's receive-only state doesn't claim transmission or rig-frequency-table effects.
It implements the state it can observe and emits no invented success response. Reply, highlighting,
annotation and unavailable named configurations have no synthetic acknowledgment. Shutdown Clear
then Close matches the Python retirement contract. UDP capture playback emits the captured
messages unchanged; qgi playback forwards complete events and adds no hop at send time.
