<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QGI 0.1.0 conformance vectors

The [WSJT-X manifest](wsjtx/manifest.json) lists each WSJT-X vector's base specification version, family,
mapping revision, class and SHA-256 of its complete file including the final newline. The manifest
itself is not self-hashed. Its `source_sha256` identifies the pinned header in Annex B.

Each vector is a JSON object with `spec`, `family`, `mapping`, `class`, `input` and `expected`.
These wrapper objects are test instructions, not events; Annex F validates their positive event members.
For the exactly-one-event WSJT-X, N1MM and `qgi` families, `expected.event` abbreviates the outcome `events: [event]` with
no skips; `expected.skip` abbreviates an empty event list and one skip notice. An error returns no
events or state updates. These singleton shorthands do not restrict the core collection outcome.

| Class or input | Meaning |
|---|---|
| `positive` | `input.data` is base64 of one datagram; `input.context` is supplied unchanged to the codec. `expected.event` is the exact expected parsed event, with the optional structured view omitted. |
| `expected_message` | Optional structured-view oracle. If the implementation offers that view, test it separately against this object and validate the event with that view against Annex F. The baseline comparison omits the informative view under `QGI-VEC-0007`. |
| `command` | Encode `input.event`, which has no `native` or `native_version`, using `input.options` (WSJT-X defaults to schema 3; N1MM QSY requires an explicit frequency unit); compare exact bytes with base64 `expected.data`. |
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

The N1MM suite has **1,080 positive**, **18 command**, **52 command_negative**,
**42 negative**, **4 skip**, and **9 compatibility** vectors (1,205 total), plus
**1,080 embedded re-encoding checks**. It covers all 11 documented root spellings, including historical
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
separately authored expected strings and plants failures. SO2R pair swaps are
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
`uv run --with jsonschema --no-project python scripts/gen_vectors.py --check --producer-strict`.
The `producer_strict=true` flag in `QGI-SER-0009` uses the same schema with the declared object
member sets closed, known kinds/context tokens and the selected mapping revision. The runner
constructs the strict WSJT-X structured-view profile from B.4's wire-type/member catalog and B.2's
composites. Every generated positive observation, command and supplied view is checked; preserved
contents whose specification version or mapping revision is newer than the reader's use the reader
profile while the new hop uses the producer profile on its own. Both compatibility paths plant an
unknown member in the appended hop's context and require its rejection. The strict profile
deliberately requires an explicit extension profile for extension fields; this
runner's authored events use only revision 1 core, WSJT-X and N1MM members. Nested color/header/
dependency/radio/structured-view additions are tested for reader acceptance and producer rejection.
For this pass, dependency installation and execution use `--offline` and a writable copy of the
cached packages; no package index or remote schema is accessed. Without `jsonschema`, plain
`--check` reports skipped schema validation. `--producer-strict` requires the module and fails if
it is unavailable.

The [core shape cases](core/shape-cases.json) contain planted valid/invalid event-schema examples
for observed versus command forms, application results, class-specific envelopes, retained/empty
PUBLISH shapes, required template evidence, dependency evidence, header bounds and future-MINOR
native members, one/eight/nine-hop shapes, common radio labels and SO2R state capacity (mapping specified in Annex D).
There are 53 core shapes; these include invalid radio types and open nested additions.
The [core manifest](core/manifest.json) hashes their complete file including its
final newline. These are structural checks only: their empty byte strings are explicit placeholders,
not valid PSK Reporter captures or complete MQTT/IPFIX decode fixtures. No mapping conformance for
Annexes G or H is claimed by passing a shape check.
