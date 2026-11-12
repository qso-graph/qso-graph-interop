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
| `compatibility` | Read `input.event` with the declared `reader_spec`, supplied by a hypothetical producer of `writer_spec`. `expected.read` is the reader's recognized `kind`, `fields`, `context` and any preserved `native`, not a rewritten stored event. For a command without native evidence, `expected.preserved_command`, when present, checks the original retained command separately. An unsupported MAJOR yields an error. |

The [qgi manifest](qgi/manifest.json) uses the same per-vector metadata and hashes; its
`source_sha256` pins the complete Annex I file named by `source`, relative to the manifest.
`qgi` positive and negative inputs declare `transport` (`udp` or `tcp`); `input.data` contains
one datagram or a TCP record including its LF/CRLF delimiter. An unterminated TCP negative means
EOF. Its `expected.event.native.data` excludes only the TCP delimiter. Its `reencode.data` checks
exact event emission/replay with the input delimiter; it does not encode the carried family's
command. `expected.event.native.carried_native` retains the input event's original native object
when present; the observed context comes from `input.context`, never the serialized identities.

The `qgi` suite has six positive cases: a carried decode, an own-family command, a WSJT-X command,
LF/CRLF stream carriage and a newer-MINOR unknown kind read as `native`. Its 16 negative cases
plant oversized UDP/TCP messages, invalid JSON/UTF-8, top-level and nested duplicate members,
a different MAJOR, a schema violation, excessive native nesting, nonobject/trailing JSON,
unfinished/empty TCP records, and container-depth, element-count and numeric-token bounds.
They need only Annex I and the event schema, without an external protocol definition.

The hypothetical 0.2.0 values exercise only the additions permitted by `QGI-GEN-0005`; they do not
publish version 0.2.0 or add real WSJT-X message types. An unknown operation token remains a string;
its presence alone supplies no permission to act. Unknown fields are omitted from the reader's
recognized view, while the original event remains available to the application. A new reader sees
an old writer's complete required envelope without depending on future fields.

The suite covers all 17 message types in all three supported schemas, historical tails, all three
Clear/Close directions, both Heartbeat directions, target IDs, null/empty/invalid text, named and
custom time zones, schema 1's clock layout, unresolved/invalid times, nonfinite and difficult finite
binary64 values, integer representability, sentinels, trailing bytes, maximum datagrams and IDs,
all five error categories, both WSJT-X skip reasons, and same-MAJOR compatibility in both directions.
Commands cover all 12 sending types in schemas 1–3, default schema 3, historical tails, raw/null/empty
text, canonical booleans, missing arguments/IDs, invalid options, numeric bounds and optional-tail holes.
`unknown_template` is a core skip reason, not a possible WSJT-X result; Annex H must supply its
stateful message-sequence vectors when completed.
Vectors are synthetic, derived from the released header and Annex B, without invoking the reference
implementation. They are not captures or proof of live application interoperability.

Regenerate with `uv run --no-project python scripts/gen_vectors.py`; check with
`uv run --no-project python scripts/gen_vectors.py --check`. The generator uses only the standard
library for generation. If `jsonschema` is already importable, it also checks the local schema,
every positive observation, command and supplied structured view, same-MAJOR compatibility inputs and planted
schema failures and the core shape cases. It also validates the carried and resulting `qgi` events
and checks the planted JSON parsing/bounds failures. It does not fetch schemas or install dependencies. Without that optional module,
it reports the skipped validation and still checks exact deterministic vector files and hashes.


The [core shape cases](core/shape-cases.json) contain planted valid/invalid event-schema examples
for observed versus command forms, application results, class-specific envelopes, retained/empty
PUBLISH shapes, required template evidence, dependency evidence, header bounds and future-MINOR
native members, plus one-level `qgi` native evidence and planted nesting/type failures.
The [core manifest](core/manifest.json) hashes their complete file including its
final newline. These are structural checks only: their empty byte strings are explicit placeholders,
not valid PSK Reporter captures or complete MQTT/IPFIX decode fixtures. No mapping conformance for
Annexes G or H is claimed by passing a shape check.
