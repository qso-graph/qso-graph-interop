<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Identity, direction, and recognized views

How the library carries the rules of [QGI-SPEC-001](../spec/QGI-SPEC-001.md); the specification is
normative and each rule below cites it.

`Event` preserves a QGI-SPEC-001 envelope. `Codec::decode` takes observer context explicitly;
its default has unknown direction and exchange (`QGI-MOD-0002`). Wire IDs and radio instance labels
remain data and never supply `context.source_id` (`QGI-MAP-0004`, `QGI-WSJ-0020`). An absent wire ID
differs from explicit null, empty text, and invalid native text (`QGI-MOD-0005`, `QGI-SER-0002`).
Neutral `Text` preserves those representations without Qt dependencies (`QGI-RI-0020`).

The core has no default family type. `Codec` and `Profile` use the public event API. Native wire
types stay in family modules and map to neutral common fields at the family boundary. Commands
contain `fields.operation` and its arguments plus `wire_id` when required. Observations carry
native evidence, including observed controls (`QGI-MOD-0007`, `QGI-MOD-0039`). The separate command
encoder, observed re-encoder, and original-byte accessor prevent reading an event from causing an
action (`QGI-MOD-0008`, `QGI-MAP-0010`, `QGI-MAP-0012`).

WSJT-X Clear maps to notification from the application, control to it, and ambiguous with unknown
direction (`QGI-WSJ-0021`). Close maps to closed lifecycle from the application, control to it, and
ambiguous with unknown direction (`QGI-WSJ-0022`). Heartbeat is lifecycle in every direction
(`QGI-WSJ-0023`). No message type supplies observer identity or exchange role (`QGI-MOD-0002`, `QGI-MAP-0004`).
Native version and mapping revision remain independent (`QGI-MOD-0034`, `QGI-MOD-0039`).

`Event::recognized` returns known common fields and context separately from the preserved event.
Unknown kinds or families read as native, and new additive mapping members are ignored recursively
in the recognized view (`QGI-SER-0005`). A supplied family `Profile` verifies supported native
versions and types, returning `Verification::Verified` or `Unavailable` separately from the JSON
envelope (`QGI-MAP-0015`). A recognized operation or native type alone is insufficient for
verification or authorization (`QGI-MAP-0015`, `QGI-SEC-0001`).

`qgi` appends only a receiving hop (`QGI-MOD-0015`). Forwarding preserves the original envelope,
context, command form, native evidence, and unknown additions (`QGI-QGI-0004`). Original-family
verification requires its supported profile; carriage never supplies that proof itself
(`QGI-QGI-0008`, `QGI-SEC-0010`). See [protocol behavior](PROTOCOL.md) and the
[specification outcome table](../spec/QGI-SPEC-001.md#4-mapping-rules-for-every-family).

## JSON float preservation

Finite binary64 fields use JSON numbers. Nonfinite values use `ieee754:` plus 16 lowercase
hexadecimal digits, preserving sign and NaN payload (`QGI-SER-0003`). Every serde_json dependency enables
`float_roundtrip`. Downstream serde_json readers should do the same:

```toml
serde_json = { version = "1.0.145", features = ["float_roundtrip"] }
```

`Binary64` is the neutral value type. The bounded JSON reader retains exact unknown numeric tokens
and rejects duplicate object members at every depth (`QGI-SER-0007`). Semantic event comparison
uses binary64 for floating fields and exact integers elsewhere, independent of object ordering
(`QGI-SER-0008`). Keep native evidence
when a downstream reader cannot round floating values correctly. Supplemental tests retain 10,000
random finite bit patterns and difficult boundary values; fuzzing also accepts arbitrary JSON.
