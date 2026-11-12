<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QGI 0.1.0 conformance vectors

The [WSJT-X manifest](wsjtx/manifest.json) lists each WSJT-X vector's base specification version, family,
mapping revision, class and SHA-256 of its complete file including the final newline. The manifest
itself is not self-hashed. Its `source_sha256` identifies the pinned header in Annex B.

Each vector is a JSON object with `spec`, `family`, `mapping`, `class`, `input` and `expected`.
These wrapper objects are test instructions, not events; Annex F validates their positive event members.
For the exactly-one-event WSJT-X and `qgi` families, `expected.event` abbreviates the outcome `events: [event]` with
no skips; `expected.skip` abbreviates an empty event list and one skip notice. An error returns no
events or state updates. These singleton shorthands do not restrict the core collection outcome.

| Class or input | Meaning |
|---|---|
| `positive` | `input.data` is base64 of one datagram; `input.context` is supplied unchanged to the codec. `expected.event` is the exact expected parsed event, with the optional structured view omitted. |
| `expected_message` | Optional structured-view oracle. If the implementation offers that view, test it separately against this object and validate the event with that view against Annex F. The baseline comparison omits the informative view under `QGI-VEC-0007`. |
| `command` | Encode `input.event`, which has no `native` or `native_version`, using `input.options` (or default schema 3); compare exact bytes with base64 `expected.data`. |
| `command_negative` | Attempt command encoding and compare `expected.error.category`. These test mapping checks, including missing arguments, optional-tail holes and native wire ranges; not every failure is expressible in the base event schema. |
| `reencode` | Separate observed-event operation: decode and check `expected.event.native.data`, re-encode, and compare with `reencode.data`. Boolean canonicalization can differ from original bytes. Original-byte retrieval always equals `input.data`. |
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
files and hashes with `python3 scripts/gen_vectors.py --check`. The generator uses only the
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
runner's authored events use only revision 1 core and WSJT-X members. Nested color/header/
dependency/radio/structured-view additions are tested for reader acceptance and producer rejection.
For this pass, dependency installation and execution use `--offline` and a writable copy of the
cached packages; no package index or remote schema is accessed. Without `jsonschema`, plain
`--check` reports skipped schema validation. `--producer-strict` requires the module and fails if
it is unavailable.

The [core shape cases](core/shape-cases.json) contain planted valid/invalid event-schema examples
for observed versus command forms, application results, class-specific envelopes, retained/empty
PUBLISH shapes, required template evidence, dependency evidence, header bounds and future-MINOR
native members, one/eight/nine-hop shapes, common radio labels and provisional SO2R state.
There are 53 core shapes; these include invalid radio types and open nested additions.
The [core manifest](core/manifest.json) hashes their complete file including its
final newline. These are structural checks only: their empty byte strings are explicit placeholders,
not valid PSK Reporter captures or complete MQTT/IPFIX decode fixtures. No mapping conformance for
Annexes G or H is claimed by passing a shape check.
