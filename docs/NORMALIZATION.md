<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Identity, direction, and recognized views

`Event` preserves a QGI-SPEC-001 envelope. `Codec::decode` takes observer context explicitly;
its default has unknown direction and exchange. Wire IDs and radio instance labels remain data
and never supply `context.source_id`. An absent wire ID differs from explicit null, empty text,
and invalid native text. Neutral `Text` preserves those representations without Qt dependencies.

The core has no default family type. `Codec` and `Profile` use the public event API. Native wire
types stay in family modules and map to neutral common fields at the family boundary. Commands
contain `fields.operation` and its arguments plus `wire_id` when required. Observations carry
native evidence, including observed controls. The separate command encoder, observed re-encoder,
and original-byte accessor prevent reading an event from causing an action.

WSJT-X Clear maps to notification from the application, control to it, and ambiguous with unknown
direction. Close maps to closed lifecycle from the application, control to it, and ambiguous with
unknown direction. Heartbeat is lifecycle in every direction. No message type supplies observer
identity or exchange role. Native version and mapping revision remain independent.

`Event::recognized` returns known common fields and context separately from the preserved event.
Unknown kinds or families read as native. New additive mapping members are ignored recursively in
the recognized view. A supplied family `Profile` verifies supported native versions and types,
returning `Verification::Verified` or `Unavailable` separately from the JSON envelope. A recognized
operation or native type alone is insufficient for verification or authorization.

`qgi` appends only a receiving hop. Forwarding preserves the original envelope, context, command
form, native evidence, and unknown additions. Original-family verification requires its supported
profile; carriage never supplies that proof itself. See [protocol behavior](PROTOCOL.md) and the
[specification outcome table](../spec/QGI-SPEC-001.md#4-mapping-rules-for-every-family).

## JSON float preservation

Finite binary64 fields use JSON numbers. Nonfinite values use `ieee754:` plus 16 lowercase
hexadecimal digits, preserving sign and NaN payload. Every serde_json dependency enables
`float_roundtrip`. Downstream serde_json readers should do the same:

```toml
serde_json = { version = "1.0.145", features = ["float_roundtrip"] }
```

`Binary64` is the neutral value type. The bounded JSON reader retains exact unknown numeric tokens
and rejects duplicate object members at every depth. Semantic event comparison uses binary64 for
floating fields and exact integers elsewhere, independent of object ordering. Keep native evidence
when a downstream reader cannot round floating values correctly. Supplemental tests retain 10,000
random finite bit patterns and difficult boundary values; fuzzing also accepts arbitrary JSON.
