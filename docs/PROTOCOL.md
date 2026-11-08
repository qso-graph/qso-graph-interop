<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Protocol evidence and behavior

## Released source pins

| Application | Released tag | Commit | Definition |
| --- | --- | --- | --- |
| WSJT-X | `v3.0.2` | `ccdfaf3c1c109010d15399674ce278167cfde848` | [Network/NetworkMessage.hpp](https://github.com/WSJTX/wsjtx/blob/ccdfaf3c1c109010d15399674ce278167cfde848/Network/NetworkMessage.hpp) |
| JTDX | `159` | `2a0e2bea8c66c9ca94d2ea8034cf83a68cfa40eb` | [NetworkMessage.hpp](https://github.com/jtdx-project/jtdx/blob/2a0e2bea8c66c9ca94d2ea8034cf83a68cfa40eb/NetworkMessage.hpp) |

WSJT-X was fetched from its official repository, `https://github.com/WSJTX/wsjtx` (where WSJT-X 3 is
released; the older SourceForge repository ends at 2.7.0), using the released tag `v3.0.2`. Its
`Network/NetworkMessage.hpp` and `Network/NetworkMessage.cpp` are byte for byte the same as at
`wsjtx-2.7.0` (SourceForge, commit `b4f9a431bcf6449df8f37b56de79d48b665b044c`), so this library
implements WSJT-X 2.7.0 and 3.0.2 alike. JTDX was fetched from `https://github.com/jtdx-project/jtdx.git` at its tag.
The annotated tag objects are `d2d9c46a1e5061a8ab489ebf473e71fa070dc4e7` and
`2e26ce38908934eb5bde18880bcbd681e3eab199`, respectively.

SHA-256 of the definitions:

```text
fd73b7f92fc0f66ea8f12a147b59a6072408c24656735d59df772dc55a5eeb62  WSJT-X NetworkMessage.hpp
5024babc52e18e8f6a9dda14284e54501366d045560475495d7b4b206ee0a2dc  JTDX NetworkMessage.hpp
```

Qt's stream operators establish types beyond the headers' abbreviated description. These are
pinned to Qt `v5.4.2`: [QDate/QTime/QDateTime](https://github.com/qt/qtbase/blob/v5.4.2/src/corelib/tools/qdatetime.cpp),
[QTimeZone](https://github.com/qt/qtbase/blob/v5.4.2/src/corelib/tools/qtimezone.cpp),
and [QColor](https://github.com/qt/qtbase/blob/v5.4.2/src/gui/painting/qcolor.cpp).
The implementation is Rust; upstream C++ is not compiled or shipped here.

## Complete message inventory

All fields listed by the pinned WSJT-X header are encoded and decoded in their specified order.
The named fixture for each message is an independent Python `struct` vector used for both directions.

| Value | WSJT-X message | Fixture |
| --- | --- | --- |
| 0 | Heartbeat | `wsjtx_Heartbeat` |
| 1 | Status | `wsjtx_Status` |
| 2 | Decode | `wsjtx_Decode` |
| 3 | Clear | `wsjtx_Clear`, `prototype_Clear` |
| 4 | Reply | `wsjtx_Reply` |
| 5 | QSOLogged | `wsjtx_QsoLogged` |
| 6 | Close | `wsjtx_Close` |
| 7 | Replay | `wsjtx_Replay` |
| 8 | HaltTx | `wsjtx_HaltTx` |
| 9 | FreeText | `wsjtx_FreeText` |
| 10 | WSPRDecode | `wsjtx_WsprDecode` |
| 11 | Location | `wsjtx_Location` |
| 12 | LoggedADIF | `wsjtx_LoggedAdif` |
| 13 | HighlightCallsign | `wsjtx_HighlightCallsign` |
| 14 | SwitchConfiguration | `wsjtx_SwitchConfiguration` |
| 15 | Configure | `wsjtx_Configure` |
| 16 | AnnotationInfo | `wsjtx_AnnotationInfo` |

JTDX defines 0–10 and 12, reserves 11 and 13–49, and adds SetTxDeltaFreq (50) and TriggerCQ (51).
Reserved types are skipped in JTDX mode. The mode is explicit because JTDX Status cannot be
reliably distinguished by bytes or the configurable sender ID.

## JTDX differences

The pinned header documents signed `qint32` Rx/Tx offsets and a final Tx-first boolean in Status,
in place of WSJT-X's special-operation and subsequent fields. Heartbeat ends after version;
Clear has no window; QSOLogged ends after My grid and calls its frequency a dial frequency.
WSJT-X's header calls that QSO field a Tx frequency. The normalized model preserves this distinction.

The [pinned MessageClient.cpp](https://github.com/jtdx-project/jtdx/blob/2a0e2bea8c66c9ca94d2ea8034cf83a68cfa40eb/MessageClient.cpp)
(SHA-256 `b2ea862b140f8d58a05cf6e34e86c447fbfda057a76a17f156176b1794e7ba58`)
differs from its own header in three places:

- Reply's header stops after message; its reader also accepts low-confidence and modifiers.
- WSPRDecode's header stops after power; its sender also writes off-air.
- TriggerCQ's header lists direction, Tx period, send. Its reader reads direction and send only,
  initializing Tx period to false. The codec accepts and emits either form:
  `tx_period: None` selects the source implementation, `Some(bool)` selects the header form.

Both forms have independent fixtures. These are source-derived vectors, not packet captures from
running applications. The codec does not claim that JTDX's receiver honors its header's Tx-period field.

## Wire and compatibility rules

- Header: big-endian magic `0xadbccbda`, schema, type, and UTF-8 QByteArray ID.
- All floating-point wire fields are IEEE 754 binary64, including fields called float upstream.
- Schema 1 uses Qt 5.0's raw date/time layout, including its UTC-clock/original-spec bug; schema 2
  uses Qt 5.2, schema 3 uses Qt 5.4. The Rust type keeps the raw Julian day, clock, and spec.
  It does not silently convert local times or repair schema 1's meaning. Schema 0 is refused.
- Schemas greater than 3 are skipped immediately after the stable eight-byte prefix. Their type
  and ID encodings cannot be assumed. Unknown types in supported schemas are skipped without
  interpreting their payload. Neither case generates a network reply.
- `negotiate_schema` limits responses to the local maximum, peer datagram schema, and peer maximum;
  a heartbeat missing its maximum implies 2. Callers manage per-peer sessions and heartbeat timers.
- A null byte array or QString length is `0xffffffff`; empty is zero. UTF-8 is retained as bytes;
  QString is retained as UTF-16 units, even for invalid Unicode. Date/time nulls use Qt's sentinels.
- QString length is in bytes. Named and custom Qt zones are carried without consulting host data.
  QColor carries its spec and all five 16-bit channels, including padding and invalid colors.
- Historical suffix fields are optional and preserve absent versus transmitted null. Status requires
  its first ten fields; Heartbeat may have no fields; Decode requires through message; Reply
  requires through message; QSOLogged requires through name. Configure's generate-messages and
  HighlightCallsign's highlight-last may be absent. A partial field is malformed.
- Additional trailing fields are kept in `Packet::extensions` and ignored by normalization. They
  are written back unchanged. Optional suffixes must be contiguous; encoders refuse holes and
  extensions placed before missing known suffixes. TriggerCQ source-form payloads cannot carry
  extensions because they would be ambiguous with the header form.
- Nonzero booleans decode as true, as Qt does; encoders canonicalize them to 1. Other values remain
  raw. JSON uses numeric finite doubles and `ieee754:<16 hex digits>` for nonfinite values, preserving
  NaN payloads. Text uses `{ "bytes": null }` versus `{ "bytes": [] }`; an absent suffix is JSON null.
- Limits are 65,507 bytes per datagram and 256 bytes per ID. Arrays must fit in remaining input
  before allocating. Encoders enforce the same limits. These are local resource limits, not claims
  about every possible upstream instance name or network MTU.

The common model exposes logged QSO, radio state, decode, control, and lifecycle observations.
It always carries the full family packet. `into_family` returns that packet; editing normalized
observations does not implicitly send commands or rewrite family data. ADIF is retained verbatim;
this milestone does not parse ADIF tags. Callsigns are not inferred from decode text.

## Fixture provenance

`scripts/generate-fixtures.py` uses Python's standard-library `struct`, independent of the Rust
codec. The original Python prototype was verified against the six packet shapes it emits and all
120 decodes in `ALL-sample.TXT`, byte for byte. Reference encoder SHA-256:
`d83f197e89daa372beff55d18145b06da47377716d4bf303f282c4bd5d41428c`.
The script ports the fixtures for repeatability without an external source checkout.

Tests also cover every datagram prefix, malicious lengths, maximum datagrams and IDs, wrong magic,
future schemas, null/empty/raw Unicode, date/time specs, colors, and model/JSON round trips.
Actual application interoperability remains a separate validation step.
