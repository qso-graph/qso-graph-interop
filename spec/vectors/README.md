<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QGI 0.1.0 conformance vectors

The [WSJT-X manifest](wsjtx/manifest.json) lists each WSJT-X vector's base specification version, family,
mapping revision, class and SHA-256 of its complete file including the final newline. The manifest
itself is not self-hashed. Its `source_sha256` identifies the pinned header in Annex B.

Each vector is a JSON object with `spec`, `family`, `mapping`, `class`, `input` and `expected`.
These wrapper objects are test instructions, not events; Annex F validates their positive event members.
For the exactly-one-event WSJT-X, N1MM, fldigi and `qgi` families, `expected.event` abbreviates the outcome `events: [event]` with
no skips; `expected.skip` abbreviates an empty event list and one skip notice. An error returns no
events or state updates. These singleton shorthands do not restrict the core collection outcome.

| Class or input | Meaning |
|---|---|
| `positive` | `input.data` is base64 of one native message (datagram, or HTTP body for fldigi); `input.context` is supplied unchanged to the codec. `expected.event` is the exact expected parsed event, with the optional structured view omitted. |
| `expected_message` | Optional structured-view oracle. If the implementation offers that view, test it separately against this object and validate the event with that view against Annex F. The baseline comparison omits the informative view under `QGI-VEC-0007`. |
| `command` | Encode `input.event`, which has no `native` or `native_version`, using `input.options` (WSJT-X defaults to schema 3; N1MM takes integer Hz arguments, emits kHz, and accepts no encoding options); compare exact bytes with base64 `expected.data` (fldigi uses ordered `expected.requests`). |
| `command_negative` | Attempt command encoding and compare `expected.error.category`. These test mapping checks, including missing arguments, optional-tail holes and native wire ranges; not every failure is expressible in the base event schema. |
| `reencode` | Separate observed-event operation: decode and check `expected.event.native.data`, re-encode, and compare with `reencode.data`. WSJT-X boolean canonicalization can differ from original bytes; N1MM re-encoding preserves native bytes exactly after consistency checking. Original-byte retrieval always equals `input.data`. |
| `negative` with `input.data` | Decode the datagram and compare `expected.error.category`; never return a partial event. |
| `negative` with `input.event` | Read/validate the supplied event; reject with the stated category. Schema shape failures are `invalid_value`; the disagreement case also needs the mapping check. |
| `negative` with `input.event_json` | Parse one serialized NDJSON record excluding its delimiter; apply the byte bound and duplicate-member rule before returning an event. |
| `skip` | Compare the reason and the readable native version/type in `expected.skip`. No event, ID parsing or reply is expected. |
| `compatibility` | Read `input.event` with the declared `reader_spec`, supplied by a hypothetical producer of `writer_spec`. `expected.read` is the reader's recognized `kind`, `fields`, `context` and any preserved `native`, not a rewritten stored event. For a command without native evidence, `expected.preserved_command`, when present, checks the original retained command separately. An unsupported MAJOR yields an error. `expected.verification`, when present, is an out-of-band validation report (`QGI-MAP-0015`), never an event member; unsupported command revisions are also exercised as `command_negative` inputs. |

The [qgi manifest](qgi/manifest.json) uses the same per-vector metadata and hashes; its
`source_sha256` pins the complete Annex I file named by `source`, relative to the manifest.
`qgi` positive and negative inputs declare `transport` (`udp` or `tcp`); `input.data` contains
one datagram or a TCP record including its delimiter. An unterminated TCP input models EOF.
`expected.event` is the unchanged carried event with the receiving context appended in `via`;
the original `context` and `native` remain intact. `reencode.event` checks the semantically
forwarded event and `reencode.delimiter` selects the stream delimiter; JSON byte identity is not
a carriage invariant. A carried command keeps its original command form. For a newer unknown kind
or mapping addition, `expected.read` additionally describes the separate compatible recognized view.

The `qgi` suite has 13 positive cases: carried observations and commands, LF/CRLF framing,
a newer-MINOR unknown kind, a newer mapping at the same specification version, two/eight hops,
maximal receiving context, a valid escaped surrogate pair, exactly 1 MiB of output and exactly
65,536 output elements. Its 22 negative cases plant oversized frames, output/context
budget overflow, a ninth hop, invalid JSON/UTF-8/Unicode strings and keys, duplicates, schema/version
errors, unfinished/empty streams, and depth, element-count and numeric-token limits, including
65,537 output elements and an input at the element limit with no room for the appended hop. These need
only Annex I and the event schema, without an external protocol definition.


The [N1MM manifest](n1mm/manifest.json) uses the same file metadata and hashes.
Its `source_sha256` is a URL-to-SHA-256 object covering the exact appendix HTML,
examples download-page HTML, downloaded XML response, and three UTC logging
documentation pages pinned in Annex D. Retrieval is 2026-11-12; the example
download page identifies its update as 2019-09-30.
No upstream documentation is bundled. XML data in these vectors is synthetic.

The N1MM suite has **1,105 positive**, **21 command**, **56 command_negative**,
**65 negative**, **4 skip**, and **9 compatibility** vectors (1,260 total), plus
**1,105 embedded re-encoding checks**. It covers all 11 documented root spellings, including historical
`lookup`, inbound `radio_setfrequency` and inbound `Spectrum`. Every catalog leaf
has an individual present and empty case; a minimal case for every root checks
all leaves absent. Nested score leaves and category/selector attributes have
separate presence, absence and empty cases. The suite includes six accepted
boolean spellings, rejected/ambiguous spellings, radio/SO2R distinctions, 10 Hz
conversion, opaque contact IDs, exchange aliases, UTC timestamps, deletion
spots, Unicode and invalid UTF-8/UTF-16 leaf wrappers, declarations/BOMs, duplicate
and nested leaves, namespaces, unknown children/roots, XML isolation attacks,
limits and limit-plus-one failures. Byte-preserving re-encoding is embedded in
every positive observation. Optional whole-document views are checked separately.
The 39 `review-*` XML cases close the independent Annex D review: declaration
grammar and simultaneous faults, lexical namespace names versus URI values,
original invalid-unit byte budgets, mixed content and CDATA, entity/line-ending
processing, default namespaces on children, mapping eligibility before numeric
limits, and UTF-16 expansion to exactly 65,507/65,508 decoded UTF-8 bytes.
Spectrum commands include arrays of length 1 and 4,096. Canonical-byte mutations
plant self-closing empties, leading zeros, changed level order, separator spaces,
trailing commas, formatting newlines, lost supplied LF and rewritten scale text.
An explicit `frequency_unit` option is rejected as `invalid_value`.

Spot, QSY and Spectrum scope frequencies use kHz derived from the pinned examples
in Annex D. Exact decimal observations map to integer Hz only when representable;
sub-Hz and out-of-range values retain native evidence with the mapped field omitted.
QSY and Spectrum commands take integer Hz and encode canonical decimal kHz without
unit options. Vectors cover whole and fractional kHz, integer limits, trailing
zeros, and 475.0/137.5 kHz on 630 m/2200 m without magnitude-based unit selection.
Cataloged timestamps map exact valid `YYYY-MM-DD HH:MM:SS` text to UTC instants:
contacts and deletions use `start`, prior contact keys use `previous_start`, and
spots and score snapshots use `time`. Lookup messages retain kind `native` and map
only their `start`/`previous_start` timestamps. Every timestamped root and leaf has
literal calendar and grammar oracles, including leap days, year end, years 0001
and 9999, invalid dates/clocks, fractional/zoned variants, whitespace, duplicates,
nested leaves and invalid encoding. Bad timestamp scalars are positive messages
with the common member absent and native bytes unchanged. Valid values are
accepted as sent without receiver-clock comparisons, plausibility windows,
corrections or warnings (`QGI-N1M-0037`). Spectrum construction derives DataCount.
Command failures include absent and out-of-range fields, radio selectors, XML text, unsupported
revisions, token/datagram bounds and disallowed arguments. Compatibility covers
both directions, newer program labels, additions, unknown roots/kinds, unavailable
native verification and preserved commands. These vectors establish the draft
contract; they do not claim that the Rust implementation implements Annex D or
that N1MM has accepted a live command.

Generation uses the standard library only. The independent XML isolation/bounds
checker never resolves external resources; the command checker compares bytes to
separately authored expected strings and plants failures. It constructs decimal
digits, separators and paired tags explicitly from Annex D's rules. The XML
checker independently checks processed text wrappers and plants replacement,
entity, CDATA, line-ending and namespace mapping faults. SO2R pair swaps are
checked against independently parsed native leaves. Timestamp oracles have an
independent calendar/grammar check with planted omission, correction, repair,
field-swap and inferred-end failures. This is not a complete N1MM
codec: all vector event oracles remain independent of the reference implementation.
With `jsonschema`, every N1MM positive, command and same-MAJOR compatibility input
is schema-checked, and producer strictness closes the N1MM whole-document view and
family argument catalog recursively. Reader acceptance and strict-producer
rejection of added radio/view/argument members are planted explicitly.

The hypothetical 0.2.0 values exercise only the additions permitted by `QGI-GEN-0005`; they do not
publish version 0.2.0 or add real WSJT-X message types. An unknown operation token remains a string;
its presence alone supplies no permission to act. Unknown fields are omitted from the reader's
recognized view, while the original event remains available to the application. Mapping-revision
vectors exercise a known kind with a new native type (`unavailable`), a known operation with
a known native type at a supported native version (`verified`), that same known operation/type at
native schema 4 (`unavailable`, with uninterpreted fields), unsupported-revision command encoding,
and a nested QColor addition at the same base specification version. A new reader sees
an old writer's complete required envelope without depending on future fields.

The suite covers all 17 message types in all three supported schemas, historical tails, all three
Clear/Close directions, both Heartbeat directions, target IDs, null/empty/invalid text, named and
custom time zones, schema 1's clock layout, unresolved/invalid times, nonfinite and difficult finite
binary64 values, integer representability, sentinels, trailing bytes, maximum datagrams and IDs, worst-case escaped Status text with its structured view,
distinguishable Status booleans with all three pair swaps planted, radio instance labels
(null/empty/raw/Unicode), empty FreeText with transmission requested, all five error categories, both WSJT-X skip reasons, and same-MAJOR compatibility in both directions.
Commands cover all 12 sending types in schemas 1–3, default schema 3, historical tails, raw/null/empty
text, canonical booleans, missing arguments/IDs, invalid options, numeric bounds, optional-tail holes, a wrong-encoding wrapper and an unnecessary raw wrapper.
`unknown_template` is a core skip reason, not a possible WSJT-X result; Annex H must supply its
stateful message-sequence vectors when completed.
Vectors are synthetic, derived from the released header and Annex B, without invoking the reference
implementation. They are not captures or proof of live application interoperability.

Regenerate with `uv run --no-project python scripts/gen_vectors.py`; check exact deterministic
files and hashes with `uv run --no-project python scripts/gen_vectors.py --check`. The generator uses only the
standard library for generation and always checks Status boolean swaps and command wrapper
negatives. When `jsonschema` is importable, it validates positives, commands and optional views
against the reader schema, same-MAJOR compatibility inputs, core shapes and planted schema errors,
and checks the `qgi` parsing/bounds oracles. It does not implement a complete WSJT-X codec or
claim that schema validation proves native consistency.

Check authored producer output and unknown-member rejections with
`uv run --no-project --with 'jsonschema[format-nongpl]==4.26.0' python scripts/gen_vectors.py --check --producer-strict`.
The `producer_strict=true` flag in `QGI-SER-0009` uses the same schema with the declared object
member sets closed, known kinds/context tokens and the selected mapping revision. The runner
constructs the strict WSJT-X structured-view profile from B.4's wire-type/member catalog and B.2's
composites. Every generated positive observation, command and supplied view is checked; preserved
contents whose specification version or mapping revision is newer than the reader's use the reader
profile while the new hop uses the producer profile on its own. Both compatibility paths plant an
unknown member in the appended hop's context and require its rejection. The strict profile
deliberately requires an explicit extension profile for extension fields; this
runner's authored events use only revision 1 core, WSJT-X, N1MM and fldigi members. Nested color/header/
dependency/radio/structured-view additions are tested for reader acceptance and producer rejection.
For this pass, dependency installation and execution use `--offline` and a writable copy of the
cached packages; no package index or remote schema is accessed. Without `jsonschema`, plain
`--check` reports skipped schema validation. `--producer-strict` requires the module and fails if
it is unavailable. When schema validation is available, a preflight also requires
the optional `date-time` checker before building vectors. A planted checker with
no formats verifies the actionable dependency diagnostic, preventing silent
acceptance of invalid calendar dates.

The [core shape cases](core/shape-cases.json) contain planted valid/invalid event-schema examples
for observed versus command forms, application results, class-specific envelopes, retained/empty
PUBLISH shapes, required template evidence, dependency evidence, header bounds and future-MINOR
native members, one/eight/nine-hop shapes, common radio labels and SO2R state capacity (mapping specified in Annex D).
There are 53 core shapes; these include invalid radio types and open nested additions.
The [core manifest](core/manifest.json) hashes their complete file including its
final newline. These are structural checks only: their empty byte strings are explicit placeholders,
not valid PSK Reporter captures or complete MQTT/IPFIX decode fixtures. No mapping conformance for
Annexes G or H is claimed by passing a shape check.

The [fldigi manifest](fldigi/manifest.json) pins release **4.2.13**, retrieval
**2026-11-12**, the author's source tarball URL/SHA-256, the SourceForge listing's
**2026-08-03 14:16:04 UTC** source date, and all 18 evidence hashes listed in
[Annex E](../annex-e-fldigi.md). Archive hashes use distinct member paths so files
sharing one tarball URL retain separate hashes. No git tag/commit is inferred
from a release tarball. Its profile hash also
identifies the complete Annex E text. The generator entry point remains
`scripts/gen_vectors.py`, which contains the authoritative independent Annex E
policy, mapping, verification and command checks. `scripts/gen_fldigi_vectors.py`
supplies the standard-library fixture builders; its older policy helpers are not
used by the entry point. No upstream source
or implementation is imported and no network access occurs during generation.

The fldigi suite has **453 positive**, **56 command**, **23 command_negative**,
**99 negative**, and **13 compatibility** vectors (**644 total**), with **453
embedded original-byte retrieval checks** and separate observed-consistency checks
that return bytes only when verification is available. Every one of the 50 mapped methods
has a request and response case; all 124 remaining pinned names have native
request and response cases. The suite covers all XML-RPC value types, nil,
untyped/empty values, ordered arrays, duplicate struct names, unknown types and
methods, faults, UTF-8/UTF-16 text and raw wrappers, split RX characters, byte
ranges, HTTP metadata independence, caller correlation and unavailable or wrong
request association. A compatible wrong association cannot be detected from a
response with no method name; that case intentionally maps by the supplied method.

The re-pin adds **26 vectors**: four request/response cases for two new methods,
21 positive cases for changed mode labels/submodes, body-derived type alternatives,
the unchanged queue misspelling and acknowledgment type distinctions, and one
malformed empty-param response derived from the bundled serializer. Existing
native timing requests now use body-required base64 and nine native response
cases use actual returned types rather than the advertised signatures. Nil
fixtures remain synthetic acceptance cases; the bundled backend's default-invalid
value is not silently treated as nil. No live-peer captures are claimed.

For fldigi, `input.data` is exactly one HTTP body, excluding headers. Optional
`input.request` supplies `method` and, for a range-read response, `arguments` in
native positional order. HTTP status and content type inputs are transport
metadata and do not change the expected event. A method name in the optional
`expected_message` view comes from the call or caller association; an unassociated
response does not invent one. The baseline event omits the informative view under
`QGI-VEC-0007`; the view oracle includes original whole-document text separately.
Only a successful associated nonempty valid version result puts the version in
`native_version`; it does not duplicate that text in fields.

fldigi commands use **`expected.requests`**, an ordered array of base64 bodies,
including for single-request operations. `send_text` with transmit true is an
append followed by a separate transmit request; false emits only the append.
An offset-bearing frequency command similarly has two bodies. Annex E defines
caller sequencing after a successful response of the first method's type (double
for the frequency setter, nil for text append), stopping on failure without
atomicity or rollback. The codec never sends requests. Command negatives plant
missing/disallowed fields, invalid XML text, numeric bounds, unsupported mappings,
metadata/options and body overflow. Canonical-byte mutations plant reversed or
missing requests, self-closing empty strings, formatting newlines, leading zeros,
changed integer tags and a numeric reference substituted for literal reference text.
Portable string commands reject CR; the source-derived bundled argument decoder
checks every emitted string independently of a conforming XML parser.

The independent checker applies the bounded XML isolation oracle used by Annex D
with Annex E's 262,144-byte body/text and 8,192-element bounds, then separately checks XML-RPC grammar,
value limits, scalar acceptance, method/association rules and mapped fields. XML
attacks include external/parameter entities without resolving them, DOCTYPE,
unknown entities, forbidden characters, invalid markup and resource overflow.
Simultaneous failures establish declaration, isolation, resource, grammar, numeric
and fault-shape precedence. Boundary cases include the largest body with its view,
UTF-16 expansion, depth/elements/attributes/names, params/arrays/members and numeric
tokens. The generator independently checks Annex E's output-budget arithmetic,
method/signature catalog and numbered requirements, with planted prose omissions,
RF/audio/state swaps, raw-text replacement, entity/line-ending errors and strict
producer unknown-member failures. Compatibility recognized views and native
verification are checked separately from preserved content. Passing these vectors
does not claim a Rust fldigi codec or live-peer interoperability.


The independent Annex E review adds **35 vectors** without changing mapping
revision 1. Existing requirement IDs are retained. Each row has a regression
that fails against the reviewed text/checks and passes after correction; the
missing-list-coverage and footer cases also plant prose omissions.

| Finding | Requirement IDs | Change and representative planted case |
|---|---|---|
| 1 | QGI-FLD-0060, 0061, 0068 | Pin S17; reject CR in all string commands. `command-bad-cr` previously encoded a reference delivered literally by the bundled decoder; now returns `invalid_value`. `command-escaping` also preserves literal entity-looking text in one pass. |
| 2 | QGI-FLD-0005, 0021, 0027, 0028, 0046 | Fault shape precedes extension dispatch. `fault-optional-future-missing-code` changes from accepted native to `malformed`; `fault-optional-future` changes from native to error result. Known bad optional/required scalars still fail. |
| 3 | QGI-MAP-0002, 0015; QGI-FLD-0013, 0021 | Core explicitly permits the family's isolated opaque fallback. `value-future-type` remains native; `compatibility-opaque-type` and `compatibility-opaque-root` require verification `unavailable`. Removing the core exception fails the prose contract. |
| 4 | QGI-SER-0010; QGI-FLD-0027, 0066 | Context validation has its own ordered stage. ASCII, Unicode and escaped contexts pass at 4,096 compact bytes and fail at 4,097; `context-hop-overflow` checks a hop. Competing XML, scalar and association errors precede context overflow. |
| 5 | QGI-FLD-0011, 0039, 0065 | Compare recognized envelope presence/values, kind and fields before verified re-encoding. `reencode-bad-native-version`, `reencode-bad-wire-id`, `reencode-bad-kind` and `reencode-bad-mapped-value` require `invalid_value`. Adding native_version to `compatibility-new-mapping` is rejected. Range and fault cases distinguish available and unavailable association-dependent checks. |
| 6 | QGI-FLD-0029–0032 | Literal operation/field expectations are separate from builders and checked against every mapping-table column. In-memory builder operation, request-field, response-field and kind mutations are regenerated and rejected. The reported `modem.get_name` → `fldigi.broken` mutation previously passed regeneration and validation. |
| 7 | QGI-FLD-0014, 0016, 0017, 0066 | Accept the complete list: 176 registrations including duplicates, 1,942 elements and three members per row. `fldigi-list-full-source-shape` uses the released order/envelope and 99 synthetic help bytes per row, at the longest released help length: 53,325 body bytes, expected native success with exact re-encoding and XML view. The fixed-source audit sets fourfold headroom at 8,192 elements, 1,024 array values and 262,144 body/text bytes; params remain 32 and members remain 256. Each ceiling has an exact-limit and limit-plus-one case. |
| 8 | QGI-FLD-0007 | Supplied null differs from absence. `association-null` changes from accepted to `invalid_value`. |
| 9 | Core annex footer | List B, D, E and I as defined normative profiles; reserve C and G. Record D’s implemented N1MM reader/writer separately from E’s pending implementation. Reintroducing E as reserved, dropping D from defined profiles or losing D’s implementation status fails the prose contract. |

The response-size audit also adds full 169-value modem and 101-value IO name
lists, the 177-value bundled system.listMethods, a native multicall with success
and fault rows, the three-member released version struct, and full 4,096-byte RX/TX/RXTX/
flmsg buffers with the bundled serializer's 72-column base64 line breaks.
Processed UTF-16 text has exact 262,144/262,145-byte cases. Actual compact output
has exact 1,048,576/1,048,577-byte cases whose input bodies satisfy every XML
resource bound. The old sufficient universal proof remains valid through 71,155
input bytes; at the new ceiling it gives 3,595,096 bytes, so actual output size is
checked atomically. The real 42,903-byte list has a conservative bound of 671,880
bytes. Caller-populated rig lists, multicall batches, growing RX widget reads and whole logbooks have
no finite application maximum; the annex records that source limitation explicitly.

These are synthetic contract tests, not captures. The bundled backend's empty-param
acknowledgments retain their specified malformed outcome; the review does not claim
live fldigi interoperability.
