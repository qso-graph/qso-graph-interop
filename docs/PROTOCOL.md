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
  extensions placed before missing known suffixes.
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

## Normalization and regression oracles

See [identity and direction](NORMALIZATION.md) for neutral IDs, caller context, and future adapter
mappings. Clear's missing window does not distinguish an erase notification from a legacy clear
command in WSJT-X; its Close also has both directions. Reply and Configure identify their target, not source.

Status wire order is Rx DF then Tx DF, per the pinned header lines 150–151. Independent fixtures
use 1201 and 2302 Hz, distinct strings and other same-type scalars, and multiple boolean patterns
whose per-position signatures differ. Normalized-field assertions supplement binary comparisons.
Run `uv run --no-project python scripts/check-field-order-mutation.py` to plant the review's paired
encoder/decoder swap in a temporary copy and verify the independent Status fixture rejects it.

The simulator's WSJT-X QSOLogged frequency is dial plus TX offset, checked for overflow before
sending the script. This follows the released
[mainwindow logging call](https://github.com/WSJTX/wsjtx/blob/v3.0.2/widgets/mainwindow.cpp#L10594),
which adds the transmit spin-box offset to nominal dial frequency. The default USB FT8 script
therefore logs 14,075,500 Hz from a 14,074,000 Hz dial and 1,500 Hz TX offset.
