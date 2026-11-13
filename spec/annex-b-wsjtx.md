<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex B: WSJT-X, mapping revision 1

This annex is normative for family `wsjtx`, specification `0.1.0`. Its pin is
[WSJT-X v3.0.2, NetworkMessage.hpp](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp),
commit `ccdfaf3c1c109010d15399674ce278167cfde848`, SHA-256
`fd73b7f92fc0f66ea8f12a147b59a6072408c24656735d59df772dc55a5eeb62`.
[Protocol evidence](../docs/PROTOCOL.md) records the matching 2.7.0 definition and the pinned
stream-format evidence for date/time, time zones and colors. Header line references below use
this exact file. The source defines layouts; the requirements here define their common mapping.

## B.1 Framing, limits and outcomes

| ID | Requirement | Verify |
|---|---|---|
| `QGI-WSJ-0010` | A codec MUST read the header in order: magic `u32` `0xadbccbda`, schema `u32`, type `u32`, ID `utf8`; all multibyte values are big endian (header lines 8–51). | Test |
| `QGI-WSJ-0011` | A codec MUST reject a datagram longer than 65,507 bytes as `oversized` before reading it. | Test |
| `QGI-WSJ-0012` | A codec MUST reject a non-null ID length greater than 256 bytes, or any other non-null declared text length greater than 65,507 bytes, as `oversized` before allocation. | Test |
| `QGI-WSJ-0013` | Schema 0 MUST return `unsupported_version`; schemas 1–3 MUST use the layouts below; a schema greater than 3 MUST return `future_version` immediately after the eight-byte magic/schema prefix, without reading type or ID. | Test |
| `QGI-WSJ-0014` | An unknown type in a supported schema MUST return `unknown_type` immediately after its four-byte type, without reading ID or body. | Test |
| `QGI-WSJ-0015` | A successful event MUST use `family: "wsjtx"`, `mapping: 1`, `native_version` equal to the schema's decimal string, `wire_id` equal to the decoded ID (including null), and `native.format: "wsjtx_datagram"`. | Test |
| `QGI-WSJ-0016` | A codec MUST parse each known optional suffix only if bytes remain, preserve its absence by omission, and report `truncated` if a begun field lacks bytes; bytes after all known fields MUST remain in `native.data` as extensions. | Test |
| `QGI-WSJ-0017` | Within a field, a decoder MUST check its declared resource bound, structural encoding, byte availability and semantic discriminator in that order; wrong magic is `malformed`, an odd non-null QString byte count is `malformed`, and a time-spec byte outside 0–3 is `invalid_value`. | Test |
| `QGI-WSJ-0018` | Every non-null length MUST fit the remaining input before allocation; absence of those bytes is `truncated`. | Test |
| `QGI-WSJ-0019` | A valid WSJT-X datagram MUST produce exactly one observed event; an unknown version/type MUST produce only its skip notice. | Test |

**`QGI-WSJ-0050` — Binary parser bounds.** These are profile resource bounds, not claims about all upstream instance names or network MTUs.
Fixed binary scalars have no numeric tokens. Nesting is fixed at the header/body and the bounded
QDateTime/time-zone structure below; recursive nesting, elements and attributes are inapplicable.
Text decoding expands no entities, performs no normalization and is bounded by its byte count
(up to one Unicode scalar per UTF-8 byte, or one per UTF-16 unit). The structured view's base64
expansion is at most four characters per three native bytes, rounded up to a multiple of four.
There is no stream delimiter or recovery state for this datagram family. Zero bytes is a truncated
header. A bad complete magic is malformed even when a later schema would be unsupported.

## B.2 Scalar and composite wire types

**`QGI-WSJ-0051` — Wire values.** A codec MUST use the following wire types and composite layouts.

| Type | Wire representation | Informative structured value |
|---|---|---|
| `u8`, `u32`, `u64` | unsigned 8, 32, 64 bits | integer, except `u64` is a decimal string |
| `i32`, `i64` | signed two's complement 32, 64 bits | integer, except `i64` is a decimal string |
| `bool` | one byte, zero false, every nonzero value true | boolean |
| `f64` | IEEE 754 binary64, even when the header says float (lines 8–11) | `QGI-SER-0003` number or nonfinite-bit string |
| `utf8` | `u32` byte count then UTF-8 bytes; `0xffffffff` is null, zero is empty | `QGI-SER-0002` text |
| `QString` | `u32` byte count then big-endian UTF-16 units; null and empty as above | `QGI-SER-0002` text |
| `QTime` | `u32` milliseconds since midnight | raw integer |
| `QDateTime` | `i64` Julian day, `QTime`, `u8` time spec, then schema-dependent suffix below | object described below |
| `QColor` | `u8` spec, then five `u16` channels including padding | `{"spec": integer, "channels": [five integers]}` |

`QDateTime` (header lines 31–38, 69–74): schema 1 stores a UTC clock with the original time-spec
byte and has no offset/zone suffix. Schemas 2 and 3 store the clock for the declared spec: 0 local,
1 UTC, 2 offset (followed by `i32` offset seconds), 3 time zone (followed by the zone layout).
A zone starts with a `QString` key. A key other than `OffsetFromUtc` is the complete named-zone
representation. That exact key introduces the custom-zone suffix in order: ID `QString`, offset
seconds `i32`, name `QString`, abbreviation `QString`, country `i32`, comment `QString`.
No host time-zone lookup is permitted. Schema 2 and schema 3 have identical layouts for these
fields; their different native schema declarations remain distinct.

The date/time structured object has `julian_day` (signed decimal string), `milliseconds` (raw u32),
`time_spec` (integer), and only the suffix members carried: `offset_seconds` or `zone`. A named zone
is `{"key": <text>}`; a custom zone has `key: "OffsetFromUtc"` and `id`, `offset_seconds`, `name`,
`abbreviation`, `country`, `comment` in the types above. Null Julian day is −9223372036854775808;
null/invalid QTime is `0xffffffff`. Other invalid calendar/clock values remain raw.
Colors preserve all spec/channel values, including invalid colors and uninterpreted spec values;
no conversion to another color space occurs.

## B.3 Mapping and canonicalization

| ID | Requirement | Verify |
|---|---|---|
| `QGI-WSJ-0020` | A codec MUST map types and fields by the tables in B.4, independent of the sender ID; ID MUST NOT fill `context.source_id`. Reply and Configure IDs name targets only. | Test |
| `QGI-WSJ-0021` | Clear MUST map to `notification` from the application, `control` to it and `ambiguous` with unknown direction, each with `operation: "clear"` and `window` only when present. | Test |
| `QGI-WSJ-0022` | Close MUST map to `lifecycle` with `state: "closed"` from the application, `control` with `operation: "close"` to it, and `ambiguous` with `operation: "close"` with unknown direction. | Test |
| `QGI-WSJ-0023` | Heartbeat MUST map to `lifecycle` with `state: "heartbeat"` in either direction, including unknown direction. | Test |
| `QGI-WSJ-0024` | A codec MUST canonicalize a native boolean to false for zero and true for nonzero, and write false as 0 and true as 1 on encoding. | Test |
| `QGI-WSJ-0025` | A valid QTime MUST map to `time_of_day` using exactly three fractional digits; a QDateTime resolving without host data MUST map to `instant` using exactly three fractional digits, otherwise the common time MUST be absent. | Test |
| `QGI-WSJ-0026` | Status frequency tolerance and T/R period equal to `0xffffffff` MUST leave their common members absent (not applicable); Configure MUST retain those integers because its sentinels are command arguments. | Test |
| `QGI-WSJ-0027` | LoggedADIF MUST map only its verbatim ADIF text, without parsing tags into other common members; WSPRDecode MUST NOT synthesize a mode, and Decode MUST NOT extract call or grid from its text. | Test |
| `QGI-WSJ-0028` | **Radio.** For types 1, 2, 5, 10 and 12, `fields.radio.instance` MUST equal the header client ID using the same text representation as `wire_id`, including null or empty; `radio.number` MUST be absent. The client ID labels the one-radio WSJT-X instance (including the program's `--rig-name` instance label), never sender identity. It MUST NOT fill `source_id`. Types with no common radio-bearing kind MUST NOT invent a radio field. | Test |
| `QGI-WSJ-0029` | **Side effects.** The WSJT-X effects listed below are the family-defined meanings for `QGI-MOD-0022`; callers MUST NOT equate `fields.text` with guaranteed on-air content. Mapping and encoding MUST NOT execute these effects. | Inspection |

| Operation | Possible WSJT-X application effects (pinned header) |
|---|---|
| `send_text` | Nonempty Text replaces the free-text and Tx5 fields; Send true selects the send action. Empty Text with Send true sends the previously staged text unchanged; that text is not carried by the command and this API cannot query it. Empty Text with Send false clears the text. The application may encode or truncate the on-air message (lines 353–381). |
| `reply` | Selects a matching decode and can enable transmission according to the application's settings (lines 256–294). |
| `halt_tx` | Stops transmission immediately or at the end of the current period according to Auto Tx Only (lines 344–350). |
| `clear` | Clears the selected activity window (lines 235–253). |
| `close` | Requests graceful application shutdown (lines 321–326). |
| `replay` | Emits stored decodes marked not new, followed by Status (lines 329–341). |
| `set_frequency` | No WSJT-X shared-operation mapping. |
| `wsjtx.configure` | Changes settings; a mode/submode change can also retune to the band's default frequency for that mode if the current frequency is not in its frequency table (lines 488–508). |
| Other family operations | Heartbeat participates in schema negotiation; Location changes the location used for WSPR/messages; HighlightCallsign changes highlighting; SwitchConfiguration changes the selected configuration; AnnotationInfo changes the application's annotation/sort value (B.4). |

**`QGI-WSJ-0052` — Time conversion.** For B.3 time conversion, a valid clock is less than 86,400,000 milliseconds. Schema 1 uses its
UTC clock regardless of time spec. For schemas 2–3, spec 1 resolves directly; spec 2 and custom
zones resolve by subtracting the supplied offset; spec 0 and named zones remain unresolved.
Julian day 2,440,588 is 1970-01-01. Conversion uses the proleptic Gregorian calendar and leaves
instants outside years 0001–9999 absent, as well as null or invalid dates/clocks. It neither attaches
an observer's date to QTime nor consults a clock. A family-carried UTC value is not receipt time.

**`QGI-WSJ-0053` — Other canonicalizations.** The other common canonicalizations are text presentation (`QGI-SER-0002`), binary64 presentation
(`QGI-SER-0003`), integer omission outside the common range (`QGI-MOD-0013`), and field renaming in
the tables. There is no callsign case conversion, whitespace trimming, mode-symbol expansion,
frequency addition, or loss of null versus empty text. UTF-8 invalidity itself is not an error.
A nonfinite float retains its bits. A negative zero is emitted as a JSON negative-zero number;
comparison of finite floating fields uses binary64 values as in `QGI-SER-0008`.

**`QGI-WSJ-0054` — Native sentinels and defaults.** Clear's native Window enum is 0 band activity (default when not supplied to the application),
1 Rx frequency, 2 both (header lines 235–253); absence remains absent in the event. An out-of-range
Window is preserved, without claiming an application action. The presence or absence of Window
never supplies direction. Status special-operation mode remains native only. Configure empty text
means no change; its Rx DF and frequency tolerance `0xffffffff` mean no change (lines 500–508).
AnnotationInfo always carries its sort order even if sort-order-provided is false; `0xffffffff`
removes the value in the application's table (lines 510–529). No such command semantics are
executed by mapping. Native application defaults never create common fields.

## B.4 Message layouts

**`QGI-WSJ-0055` — Layout catalog.** These tables list body fields in wire order, after the common header and ID. `?` is an optional
contiguous historical suffix, not a null marker; all fields through the last unmarked row are
required on the wire. This optionality comes from the historical compatibility rules recorded in
`docs/PROTOCOL.md`, not from a schema-number test. Header lines 89–102 allow appended fields
without a schema increment. The left column is also the member name in `native.message.body`.
“Native only” means it has no common field; it is still decoded for validation and encoding.
The direction column is the native protocol's defined direction; it never changes caller context.

| Type | Name | Native direction | Common kind | Operation or state |
|---|---|---|---|---|
| 0 | Heartbeat | both | lifecycle | `heartbeat` |
| 1 | Status | from application | radio_state | — |
| 2 | Decode | from application | decode | — |
| 3 | Clear | both | directional | `clear` |
| 4 | Reply | to application | control | `reply` |
| 5 | QSOLogged | from application | qso_logged | — |
| 6 | Close | both | directional | `close` |
| 7 | Replay | to application | control | `replay` |
| 8 | HaltTx | to application | control | `halt_tx` |
| 9 | FreeText | to application | control | `send_text` |
| 10 | WSPRDecode | from application | decode | — |
| 11 | Location | to application | control | `wsjtx.location` |
| 12 | LoggedADIF | from application | qso_logged | — |
| 13 | HighlightCallsign | to application | control | `wsjtx.highlight_callsign` |
| 14 | SwitchConfiguration | to application | control | `wsjtx.switch_configuration` |
| 15 | Configure | to application | control | `wsjtx.configure` |
| 16 | AnnotationInfo | to application | control | `wsjtx.annotation_info` |

### 0: Heartbeat

[Header line 110](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L110).

| Native field | Wire type | Common field |
|---|---|---|
| `max_schema` | `u32?` | `max_schema` |
| `version` | `utf8?` | `version` |
| `revision` | `utf8?` | `revision` |

### 1: Status

[Header line 141](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L141).

| Native field | Wire type | Common field |
|---|---|---|
| `dial_frequency_hz` | `u64` | `dial_frequency_hz` |
| `mode` | `utf8` | `mode` |
| `dx_call` | `utf8` | `dx_call` |
| `report` | `utf8` | `report` |
| `tx_mode` | `utf8` | `tx_mode` |
| `tx_enabled` | `bool` | `tx_enabled` |
| `transmitting` | `bool` | `transmitting` |
| `decoding` | `bool` | `decoding` |
| `rx_df_hz` | `u32` | `rx_offset_hz` |
| `tx_df_hz` | `u32` | `tx_offset_hz` |
| `de_call` | `utf8?` | `station_call` |
| `de_grid` | `utf8?` | `station_grid` |
| `dx_grid` | `utf8?` | `dx_grid` |
| `tx_watchdog` | `bool?` | `tx_watchdog` |
| `sub_mode` | `utf8?` | `sub_mode` |
| `fast_mode` | `bool?` | `fast_mode` |
| `special_operation_mode` | `u8?` | native only |
| `frequency_tolerance_hz` | `u32?` | `frequency_tolerance_hz` |
| `tr_period_seconds` | `u32?` | `tr_period_seconds` |
| `configuration_name` | `utf8?` | `configuration_name` |
| `tx_message` | `utf8?` | `tx_message` |

### 2: Decode

[Header line 209](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L209).

| Native field | Wire type | Common field |
|---|---|---|
| `new` | `bool` | `is_new` |
| `time` | `QTime` | `time_of_day` |
| `snr_db` | `i32` | `snr_db` |
| `delta_time_seconds` | `f64` | `dt_seconds` |
| `delta_frequency_hz` | `u32` | `audio_offset_hz` |
| `mode` | `utf8` | `mode` |
| `message` | `utf8` | `text` |
| `low_confidence` | `bool?` | `low_confidence` |
| `off_air` | `bool?` | `off_air` |

### 3: Clear

[Header line 235](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L235).

| Native field | Wire type | Common field |
|---|---|---|
| `window` | `u8?` | `window` |

### 4: Reply

[Header line 256](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L256).

| Native field | Wire type | Common field |
|---|---|---|
| `time` | `QTime` | `time_of_day` |
| `snr_db` | `i32` | `snr_db` |
| `delta_time_seconds` | `f64` | `dt_seconds` |
| `delta_frequency_hz` | `u32` | `audio_offset_hz` |
| `mode` | `utf8` | `mode` |
| `message` | `utf8` | `text` |
| `low_confidence` | `bool?` | `low_confidence` |
| `modifiers` | `u8?` | `modifiers` |

### 5: QSOLogged

[Header line 296](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L296).

| Native field | Wire type | Common field |
|---|---|---|
| `time_off` | `QDateTime` | `end` |
| `dx_call` | `utf8` | `call` |
| `dx_grid` | `utf8` | `grid` |
| `frequency_hz` | `u64` | `tx_frequency_hz` |
| `mode` | `utf8` | `mode` |
| `report_sent` | `utf8` | `report_sent` |
| `report_received` | `utf8` | `report_received` |
| `tx_power` | `utf8` | `tx_power` |
| `comments` | `utf8` | `comments` |
| `name` | `utf8` | `name` |
| `time_on` | `QDateTime?` | `start` |
| `operator_call` | `utf8?` | `operator_call` |
| `my_call` | `utf8?` | `station_call` |
| `my_grid` | `utf8?` | `station_grid` |
| `exchange_sent` | `utf8?` | `exchange_sent` |
| `exchange_received` | `utf8?` | `exchange_received` |
| `propagation_mode` | `utf8?` | `propagation_mode` |

### 6: Close

[Header line 321](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L321).

| Native field | Wire type | Common field |
|---|---|---|
| — | no body | — |

### 7: Replay

[Header line 329](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L329).

| Native field | Wire type | Common field |
|---|---|---|
| — | no body | — |

### 8: HaltTx

[Header line 344](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L344).

| Native field | Wire type | Common field |
|---|---|---|
| `auto_tx_only` | `bool` | `auto_tx_only` |

### 9: FreeText

[Header line 353](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L353).

| Native field | Wire type | Common field |
|---|---|---|
| `text` | `utf8` | `text` |
| `send` | `bool` | `transmit` |

### 10: WSPRDecode

[Header line 384](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L384).

| Native field | Wire type | Common field |
|---|---|---|
| `new` | `bool` | `is_new` |
| `time` | `QTime` | `time_of_day` |
| `snr_db` | `i32` | `snr_db` |
| `delta_time_seconds` | `f64` | `dt_seconds` |
| `frequency_hz` | `u64` | `frequency_hz` |
| `drift_hz` | `i32` | `drift_hz` |
| `callsign` | `utf8` | `call` |
| `grid` | `utf8` | `grid` |
| `power_dbm` | `i32` | `power_dbm` |
| `off_air` | `bool?` | `off_air` |

### 11: Location

[Header line 407](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L407).

| Native field | Wire type | Common field |
|---|---|---|
| `location` | `utf8` | `location` |

### 12: LoggedADIF

[Header line 424](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L424).

| Native field | Wire type | Common field |
|---|---|---|
| `adif` | `utf8` | `adif` |

### 13: HighlightCallsign

[Header line 445](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L445).

| Native field | Wire type | Common field |
|---|---|---|
| `callsign` | `utf8` | `call` |
| `background` | `QColor` | `background_color` |
| `foreground` | `QColor` | `foreground_color` |
| `highlight_last` | `bool?` | `highlight_last` |

### 14: SwitchConfiguration

[Header line 479](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L479).

| Native field | Wire type | Common field |
|---|---|---|
| `configuration_name` | `utf8` | `configuration_name` |

### 15: Configure

[Header line 488](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L488).

| Native field | Wire type | Common field |
|---|---|---|
| `mode` | `utf8` | `mode` |
| `frequency_tolerance_hz` | `u32` | `frequency_tolerance_hz` |
| `sub_mode` | `utf8` | `sub_mode` |
| `fast_mode` | `bool` | `fast_mode` |
| `tr_period_seconds` | `u32` | `tr_period_seconds` |
| `rx_df_hz` | `u32` | `rx_offset_hz` |
| `dx_call` | `utf8` | `dx_call` |
| `dx_grid` | `utf8` | `dx_grid` |
| `generate_messages` | `bool?` | `generate_messages` |

### 16: AnnotationInfo

[Header line 510](https://github.com/WSJTX/wsjtx/blob/v3.0.2/Network/NetworkMessage.hpp#L510).

| Native field | Wire type | Common field |
|---|---|---|
| `dx_call` | `utf8` | `dx_call` |
| `sort_order_provided` | `bool` | `sort_order_provided` |
| `sort_order` | `u32` | `sort_order` |

## B.5 Structured view

| ID | Requirement | Verify |
|---|---|---|
| `QGI-WSJ-0030` | If emitted, `native.message` MUST use `schema` (integer), `type` (integer), `id` (text), `body` (object using B.4 member names and B.2 values), and `extensions` (base64 of trailing bytes, including `""` when empty); absent suffix members MUST be omitted. | Test |

**`QGI-WSJ-0056` — Structured value spelling.** This view is informative: its presence is optional, its values do not replace `native.data`, and
the suite tests the permitted omitted view as its baseline, with separate structured-view oracles.
Event comparison otherwise follows `QGI-SER-0008`. The `u64`/`i64` decimal strings prevent accidental
precision loss even where the common integer cannot be represented. Their spelling is a minus
sign only for negative values, followed by decimal digits without leading zeros, except `0`.

## B.6 Commands and observed re-encoding

Command side effects are defined once in B.3 (`QGI-WSJ-0029`), including staged FreeText and indirect Configure retuning.

| ID | Requirement | Verify |
|---|---|---|
| `QGI-WSJ-0040` | Command encoding MUST select types 0, 3, 4, 6–9, 11 and 13–16 from the operation table below and encode only the command fields and `wire_id`, using B.2 primitives and B.4 wire order. | Test |
| `QGI-WSJ-0041` | Observed re-encoding MUST decode `native.data`, check its mapping consistency, and write every field, preserving raw values and extensions except boolean-byte canonicalization under `QGI-WSJ-0024`. | Test |
| `QGI-WSJ-0042` | Command encoding MUST reject a value outside B.2 wire ranges, a missing required argument, an unrecognized operation, or a hole in an optional tail as `invalid_value`. | Test |
| `QGI-WSJ-0043` | Encoding MUST enforce the decoding size limits before producing a datagram. | Test |
| `QGI-WSJ-0044` | The encoding option `schema` MUST default to 3 and accept only integers 1–3; any other schema MUST return `unsupported_version`. | Test |
| `QGI-WSJ-0045` | Every command MUST include `wire_id`, including explicit null when intended, as the native target/instance ID; a missing ID MUST return `invalid_value`. | Test |
| `QGI-WSJ-0046` | A command `clear.window`, when present, MUST be 0, 1 or 2. | Test |
| `QGI-WSJ-0047` | A command `reply.time_of_day` MUST be representable as integer milliseconds since UTC midnight in 0–86,399,999. | Test |
| `QGI-WSJ-0048` | Command encoding MUST emit no trailing extensions or inferred optional arguments. | Test |

| Operation | Type | Required arguments in `fields` (besides `operation`) | Optional contiguous tail |
|---|---|---|---|
| `wsjtx.heartbeat` | 0 | none | `max_schema`, `version`, `revision` |
| `clear` | 3 | none | `window` |
| `reply` | 4 | `time_of_day`, `snr_db`, `dt_seconds`, `audio_offset_hz`, `mode`, `text` | `low_confidence`, `modifiers` |
| `close` | 6 | none | none |
| `replay` | 7 | none | none |
| `halt_tx` | 8 | `auto_tx_only` | none |
| `send_text` | 9 | `text`, `transmit` | none |
| `wsjtx.location` | 11 | `location` | none |
| `wsjtx.highlight_callsign` | 13 | `call`, `background_color`, `foreground_color` | `highlight_last` |
| `wsjtx.switch_configuration` | 14 | `configuration_name` | none |
| `wsjtx.configure` | 15 | `mode`, `frequency_tolerance_hz`, `sub_mode`, `fast_mode`, `tr_period_seconds`, `rx_offset_hz`, `dx_call`, `dx_grid` | `generate_messages` |
| `wsjtx.annotation_info` | 16 | `dx_call`, `sort_order_provided`, `sort_order` | none |

**`QGI-WSJ-0057` — Argument representations.** `operation` is `fields.operation`. A text argument MUST use a string when valid in its native encoding, and only the matching raw wrapper when invalid; WSJT-X UTF-8 arguments MUST reject UTF-16 wrappers and raw UTF-8 wrappers containing valid UTF-8 as `invalid_value`. Required text arguments accept explicit null and empty under
B.2, but omission is not null. QColor arguments use the B.2 object (`spec` u8, exactly five u16
`channels`); every channel, including padding, is supplied explicitly. Integer fields use the
corresponding B.4 scalar bounds: `snr_db` i32, `modifiers` u8, `max_schema`, offsets, tolerances,
periods and sort order u32. Boolean arguments require booleans. `reply.dt_seconds` uses the exact
binary64 representation of §3, including nonfinite bits. Optional tails end at the last supplied
argument, with no gaps. Empty heartbeat and Clear are valid historical command shapes.

Example command using `QGI-MOD-0031` and `QGI-MOD-0035`:

```json
{"spec":"0.1.0","family":"wsjtx","mapping":1,"kind":"control","context":{"direction":"to_application","exchange":"request"},"wire_id":"test-instance","fields":{"operation":"send_text","text":"synthetic message","transmit":false}}
```

**`QGI-WSJ-0058` — Encoding boundary.** Encoding with omitted options uses schema 3. Explicit `schema: 1` uses schema 1 even without
previous wire traffic. Encoding performs no negotiation, transport or action. An application
that negotiates limits the schema to its supported maximum, peer datagram schema and peer maximum;
a heartbeat without maximum implies 2 to that application (header lines 135–138). Peer state,
periodic heartbeats and transport stay outside this family codec. This does not prohibit bounded
template state in templated families under `QGI-CLS-0021`.

**`QGI-WSJ-0059` — Evidence operations.** Observed re-encoding is available for the sending types; observation types 1, 2, 5, 10 and 12 may
also be offered for simulators. Explicit original-byte retrieval remains separate from re-encoding
and always returns `native.data` exactly, including noncanonical boolean bytes. Commands cannot
retrieve or re-encode absent native evidence. An observed out-of-range Clear window remains
lossless under B.3; that does not make it an accepted argument for command construction.

## B.7 Observation output budget

**`QGI-WSJ-0060` — Bounded representation.** An observer MUST serialize with compact JSON
separators and UTF-8 text (escaping only JSON-required characters), or a representation no larger
than the bound below; it MUST enforce `QGI-SER-0004` before emission. An optional structured view
MUST follow B.5, rather than arbitrary extra expansion. Contexts MUST meet `QGI-SER-0010`.

The largest native datagram is N = 65,507 bytes. Its base64 evidence uses
4 × ceil(N / 3) = 87,344 bytes. A valid text byte costs at most six JSON bytes (a control escape);
an invalid UTF-8 byte array costs at most four per byte including commas, and UTF-16 unit arrays
at most six per two native bytes. The common and structured views therefore use at most 12N =
786,084 bytes for text/extension contents. The client ID occurs in `wire_id`, `radio.instance` and
structured `id`; reserving two copies in 12N plus one extra maximum escaped ID costs 6 × 256 =
1,536 bytes. Fixed member names, punctuation, scalar values, text-wrapper names and date/time
objects use a conservative 32,768-byte reserve: each supported type has fewer than 256 such
members/scalars across both views, each budgeted 128 bytes. Context and all hops use at most the
65,536-byte reserve. Total: 87,344 + 786,084 + 1,536 + 32,768 + 65,536 = 973,268 bytes,
leaving 75,308 bytes below 1 MiB. Consequently every admitted WSJT-X datagram fits, including
maximal common fields and the specified structured view. The terms intentionally overcount
shared wire storage; they are an upper bound rather than a claim that all maxima coexist.
