<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex A: common fields, version 0.1.0

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MOD-0020` | The common `fields` object MUST follow the catalog below and the selected family mapping. All listed members are optional except `operation` for `control`, `notification` and `ambiguous`, `state` for `lifecycle`, and `status` for `result`. | Test |
| `QGI-MOD-0021` | A common field MUST retain the native field's meaning and units; a family MUST define any exact unit conversion before using it. | Test |

**`QGI-MOD-0023` — Catalog value types.** `text` means the string/null/raw-code-unit alternatives in `QGI-SER-0002`. `token` means a nonempty
ASCII lowercase identifier (`[a-z][a-z0-9_]*`); an operation additionally permits a family prefix (`<family>.<token>`). `integer` is bounded by `QGI-MOD-0013`.
`binary64` includes the nonfinite-bit representation in `QGI-SER-0003`. `instant` and `time_of_day`
are defined by `QGI-MOD-0006`; no time field accepts null. No optional field has an implied default.
Standard common member names use `snake_case`; extension names follow `QGI-EXT-0003`.

**Catalog scope (informative).** The expected-family column is a core design check, not an unwritten protocol mapping. The completed `wsjtx`, `n1mm` and `fldigi` annexes assign only their explicit mappings; unassigned
expected-family entries remain capacity, not support claims. Other entries identify intended uses
to verify when Annexes C, G and H are pinned. They do not authorize guessing a value or its units. In particular, an
XML-RPC result can use common fields only when the caller supplies the associated method; a response
body alone does not name that method. Extra contest exchange components remain in `native` unless
a later annex assigns a named common field. Serial numbers are text to preserve leading zeros.

**`QGI-MOD-0024` — Partial records.** No data field is universally required: a decode may contain only text, a radio report may contain
only a changed frequency, a logged QSO may contain only ADIF, and a QSO change/delete may identify a
record without repeating its content. A missing record ID does not authorize synthesizing one.
`operation`, `state` and `status` are required to identify the meaning of otherwise empty events. The `native` kind
needs no common members; `{}` is its common view unless a family annex defines optional profile members.

**`QGI-MOD-0025` — Field meanings.** `frequency_hz` on a decode or spot is the native reported frequency, not dial plus audio offset.
`tx_frequency_hz` and `dial_frequency_hz` are distinct on a QSO; no addition is implicit. `tx_power`
is text (such as the peer's entry), distinct from a decode's numeric `power_dbm`.
`start` and `end` are QSO instants. `time` is an observation instant, independent of observer receipt
time. A score is a reported snapshot, not an instruction to increment a counter. A `qso_changed`
event contains the fields the native change carries, not a reconstructed complete record.
N1MM `previous_start` is the prior contact-key instant carried with an edit, independent of the
current `start`; it is not a QSO end time. Each timestamp maps independently.

## decode

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `radio` | radio object (`QGI-MOD-0014`) | — | wsjtx (types in B.3), n1mm; others only when defined by their annex |
| `time_of_day` | time_of_day | UTC | wsjtx, js8call |
| `time` | instant | UTC | js8call |
| `snr_db` | integer | dB | wsjtx, js8call |
| `dt_seconds` | binary64 | seconds | wsjtx, js8call |
| `audio_offset_hz` | integer | Hz | wsjtx, js8call |
| `frequency_hz` | integer | Hz | wsjtx, js8call |
| `mode` | text | — | wsjtx, js8call |
| `text` | text | — | wsjtx, fldigi (RX chunks, Annex E), js8call |
| `call` | text | — | wsjtx, js8call |
| `grid` | text | — | wsjtx, js8call |
| `power_dbm` | integer | dBm | wsjtx |
| `drift_hz` | integer | Hz | wsjtx |
| `is_new` | boolean | — | wsjtx |
| `low_confidence` | boolean | — | wsjtx |
| `off_air` | boolean | — | wsjtx |

## radio_state

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `radio` | radio object (`QGI-MOD-0014`) | — | wsjtx (types in B.3), n1mm; others only when defined by their annex |
| `dial_frequency_hz` | integer | Hz | wsjtx, n1mm, fldigi, js8call |
| `rx_offset_hz` | integer | Hz | wsjtx, n1mm, fldigi, js8call |
| `tx_offset_hz` | integer | Hz | wsjtx, n1mm, fldigi, js8call |
| `mode` | text | — | wsjtx, n1mm, fldigi, js8call |
| `tx_mode` | text | — | wsjtx, n1mm, fldigi, js8call |
| `tx_enabled` | boolean | — | wsjtx, n1mm, fldigi, js8call |
| `transmitting` | boolean | — | wsjtx, n1mm (Annex D), fldigi, js8call |
| `decoding` | boolean | — | wsjtx |
| `tx_watchdog` | boolean | — | wsjtx |
| `fast_mode` | boolean | — | wsjtx |
| `station_call` | text | — | wsjtx, n1mm, fldigi, js8call |
| `station_grid` | text | — | wsjtx, n1mm, fldigi, js8call |
| `dx_call` | text | — | wsjtx, n1mm, fldigi, js8call |
| `dx_grid` | text | — | wsjtx, n1mm, fldigi, js8call |
| `report` | text | — | wsjtx |
| `sub_mode` | text | — | wsjtx |
| `configuration_name` | text | — | wsjtx |
| `tx_message` | text | — | wsjtx |
| `band` | text | — | n1mm |
| `frequency_tolerance_hz` | integer | Hz | wsjtx |
| `tr_period_seconds` | integer | seconds | wsjtx |
| `focus_radio` | integer | native radio number | n1mm (Annex D) |
| `active_radio` | integer | native radio number | n1mm (Annex D) |
| `running` | boolean | — | n1mm (Annex D) |
| `stereo` | boolean | — | n1mm (Annex D) |

## qso_logged, qso_changed and qso_deleted

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `radio` | radio object (`QGI-MOD-0014`) | — | wsjtx (types in B.3), n1mm; others only when defined by their annex |
| `record_id` | text | — | n1mm |
| `call` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `grid` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `mode` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `station_call` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `station_grid` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `operator_call` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `report_sent` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `report_received` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `exchange_sent` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `exchange_received` | text | — | wsjtx (logged only), n1mm, fldigi, js8call |
| `start` | instant | UTC | wsjtx (logged only), n1mm, fldigi, js8call |
| `end` | instant | UTC | wsjtx (logged only), n1mm, fldigi, js8call |
| `previous_start` | instant | UTC | n1mm profile only; prior contact key (Annex D) |
| `tx_frequency_hz` | integer | Hz | wsjtx (logged: tx only), n1mm, fldigi, js8call |
| `dial_frequency_hz` | integer | Hz | wsjtx (logged: tx only), n1mm, fldigi, js8call |
| `band` | text | — | n1mm |
| `contest_name` | text | — | n1mm |
| `serial_sent` | text | — | n1mm |
| `serial_received` | text | — | n1mm |
| `section` | text | — | n1mm |
| `precedence` | text | — | n1mm |
| `check` | text | — | n1mm |
| `tx_power` | text | — | wsjtx (logged only), n1mm, fldigi |
| `comments` | text | — | wsjtx (logged only), n1mm, fldigi |
| `name` | text | — | wsjtx (logged only), n1mm, fldigi |
| `propagation_mode` | text | — | wsjtx (logged only), n1mm, fldigi |
| `adif` | text | — | wsjtx (logged only), fldigi, js8call |

## spot

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `radio` | radio object (`QGI-MOD-0014`) | — | wsjtx (types in B.3), n1mm; others only when defined by their annex |
| `call` | text | — | n1mm, js8call |
| `grid` | text | — | n1mm, js8call |
| `mode` | text | — | n1mm, js8call |
| `text` | text | — | n1mm, js8call |
| `spotter_call` | text | — | n1mm, js8call |
| `source` | text | — | n1mm, js8call |
| `frequency_hz` | integer | Hz | n1mm, js8call |
| `time` | instant | UTC | n1mm, js8call |
| `time_of_day` | time_of_day | UTC | n1mm |
| `receiver_call` | text | — | pskreporter-mqtt, pskreporter-ipfix |
| `receiver_locator` | text | — | pskreporter-mqtt, pskreporter-ipfix |
| `sender_call` | text | — | pskreporter-mqtt, pskreporter-ipfix |
| `sender_locator` | text | — | pskreporter-mqtt, pskreporter-ipfix |
| `reporting_software` | text | — | pskreporter-mqtt, pskreporter-ipfix |
| `snr_db` | integer | dB | pskreporter-mqtt, pskreporter-ipfix |

**`QGI-MOD-0026` — Reception fields.** PSK Reporter uses `spot`: a reception report describes who heard whom, which fits a spot without
creating a new kind. Explicit `receiver_*` and `sender_*` names avoid treating a broker as the
receiver or assuming the reporting station transmitted. `frequency_hz`, `mode` and `time` above
also apply to both PSK Reporter families. `time` is the native report's instant, not arrival time;
it does not claim transmission-start semantics unless the pinned mapping establishes that meaning.
`reporting_software` names the receiving decoder's software, never the broker or this codec.
Absent locators, SNR or software remain absent; a later metadata lookup cannot silently fill them.
Annex H must include supporting receiver records as native dependencies when it maps their fields
onto a sender reception event. These choices specify model capacity, not a completed PSK mapping.

## score

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `contest_name` | text | — | n1mm |
| `station_call` | text | — | n1mm |
| `band` | text | — | n1mm |
| `mode` | text | — | n1mm |
| `qso_count` | integer | count | n1mm |
| `multiplier_count` | integer | count | n1mm |
| `points` | integer | points | n1mm |
| `total_score` | integer | points | n1mm |
| `time` | instant | UTC | n1mm; native snapshot instant |

## control

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `radio` | radio object (`QGI-MOD-0016`) | — | n1mm `set_frequency` selector; Annex D |
| `operation` | operation token | — | wsjtx, n1mm, fldigi, js8call |
| `window` | integer | — | wsjtx |
| `time_of_day` | time_of_day | UTC | wsjtx |
| `snr_db` | integer | dB | wsjtx |
| `dt_seconds` | binary64 | seconds | wsjtx |
| `audio_offset_hz` | integer | Hz | wsjtx, n1mm, fldigi, js8call |
| `rx_offset_hz` | integer | Hz | wsjtx, n1mm, fldigi, js8call |
| `frequency_tolerance_hz` | integer | Hz | wsjtx, n1mm, fldigi, js8call |
| `frequency_hz` | integer | Hz | wsjtx, n1mm, fldigi, js8call |
| `mode` | text | — | wsjtx, n1mm, fldigi, js8call |
| `text` | text | — | wsjtx, n1mm, fldigi, js8call |
| `sub_mode` | text | — | wsjtx, n1mm, fldigi, js8call |
| `dx_call` | text | — | wsjtx, n1mm, fldigi, js8call |
| `dx_grid` | text | — | wsjtx, n1mm, fldigi, js8call |
| `location` | text | — | wsjtx, n1mm, fldigi, js8call |
| `call` | text | — | wsjtx, n1mm, fldigi, js8call |
| `configuration_name` | text | — | wsjtx, n1mm, fldigi, js8call |
| `low_confidence` | boolean | — | wsjtx |
| `auto_tx_only` | boolean | — | wsjtx |
| `transmit` | boolean | — | wsjtx, fldigi, js8call |
| `highlight_last` | boolean | — | wsjtx |
| `fast_mode` | boolean | — | wsjtx |
| `generate_messages` | boolean | — | wsjtx |
| `sort_order_provided` | boolean | — | wsjtx |
| `modifiers` | integer | — | wsjtx |
| `sort_order` | integer | — | wsjtx |
| `max_schema` | integer | — | wsjtx |
| `version` | text | — | wsjtx |
| `revision` | text | — | wsjtx |
| `background_color` | QColor object (Annex B.2) | — | wsjtx |
| `foreground_color` | QColor object (Annex B.2) | — | wsjtx |
| `tr_period_seconds` | integer | seconds | wsjtx |

## Operation registry

| ID | Requirement | Verify |
|---|---|---|
| `QGI-MOD-0022` | An operation MUST retain the action and argument meanings in the registry below; a family annex MUST define supported argument combinations and reject unsupported combinations as `invalid_value`. Each shared operation's possible side effects are **family-defined; see the family annex** (WSJT-X: B.3). `send_text.text` is the native text argument; it does not guarantee the transmitted content. The operation registry below defines no cross-family guarantee about buffering, timing or RF state. | Test |

This is a capability catalog, not permission to transmit or a claim that the placeholder codecs
exist. N1MM and fldigi entries are defined in Annexes D and E; JS8Call entries remain provisional. A dash means no
support established here, not a claim that the program can never offer the action.

| Shared operation | Shared arguments and meaning | WSJT-X | Other capability (Annex D/E or provisional) | Possible side effects |
|---|---|---|---|---|
| `halt_tx` | Stop transmission; optional `auto_tx_only` restricts automatic TX | HaltTx, requires `auto_tx_only` | fldigi `main.abort` (Annex E); JS8Call-improved `RIG.TX_HALT` — to be confirmed in the family's annex | family-defined; see the family annex |
| `set_frequency` | `frequency_hz`: absolute radio dial frequency; `radio`: family-defined selection; optional `rx_offset_hz`: audio offset | — (see Configure effects in B.3) | N1MM `radio_setfrequency` (Annex D encodes Hz as decimal kHz); fldigi `rig.set_frequency`, optionally followed by `modem.set_carrier` (Annex E); JS8Call `RIG.SET_FREQ` — to be confirmed in the family's annex | family-defined; see the family annex |
| `send_text` | `text`: native text argument; `transmit`: request transmission; staging and empty-text behavior are family-defined | FreeText | fldigi `text.add_tx`, followed by `main.tx` only for transmit true (Annex E); JS8Call `TX.SET_TEXT` / `TX.SEND_MESSAGE` — to be confirmed in the family's annex | family-defined; see the family annex |
| `reply` | Select a decode to reply to; `time_of_day`, `snr_db`, `dt_seconds`, `audio_offset_hz`, `mode`, `text`; optional `low_confidence`, `modifiers` | Reply | — | family-defined; see the family annex |
| `clear` | Clear activity display; optional `window` | Clear | — | family-defined; see the family annex |
| `close` | Request application close; family annex defines shutdown arguments | Close | — | family-defined; see the family annex |
| `replay` | Request replay of stored observations | Replay | — | family-defined; see the family annex |

**`QGI-MOD-0027` — Registered operations.** The seven shared tokens are reserved core actions even if this revision establishes only one
family's mapping. A family-specific action uses a prefixed token: WSJT-X registers
`wsjtx.heartbeat`, `wsjtx.location`, `wsjtx.highlight_callsign`, `wsjtx.switch_configuration`,
`wsjtx.configure` and `wsjtx.annotation_info`. Heartbeat is a protocol maintenance command;
its observed form remains `lifecycle`. Annex B defines every required and optional argument
for those operations. Annex D additionally defines `n1mm.spectrum` and its family arguments. Annex E defines
fldigi method operation tokens and its family-only `rx_start`/`rx_length` control arguments.

Published references consulted 2026-11-12; N1MM and fldigi pins are in Annexes D and E; C remains pending:
[N1MM External UDP Messages](https://n1mmwp.hamdocs.com/appendices/external-udp-broadcasts/),
[fldigi 4.2.00 XML-RPC methods](https://www.w1hkj.org/FldigiHelp/xmlrpc_control_page.html), and
[JS8Call-improved API v3.0.0](https://js8call.com/JS8Call-improved/d7/d15/md_docs_2API.html).
Annex E establishes RF dial frequency separately from the modem audio offset, using released
handlers in preference to help wording. Its `send_text` and optional-offset `set_frequency`
return ordered request bodies for caller orchestration, without atomicity. Polled log fields
map to method-associated results or contact state; they do not assert a newly logged QSO.
Station call/locator and independent transmit offset are not exposed by this pinned mapping.
JS8Call still needs a pinned staging/sending distinction and original v2.3.1 comparison.

## result

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `status` | `ok` or `error` | — | request/response families |
| `operation` | operation token | — | request/response families, only with associated request |
| `native_code` | text | native status code, exact spelling | request/response families |
| `text` | text | native error or result description | request/response families |
| `frequency_hz` | integer | Hz | fldigi frequency setter's returned old value (Annex E); other request/response mappings when established |

**`QGI-MOD-0028` — Result meaning.** `result` separates a well-formed program rejection from a codec failure. It does not translate
native error codes into a universal enum. Unmapped result data remains in native evidence.

## notification

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `operation` | operation token | — | wsjtx, n1mm, fldigi, js8call |
| `window` | integer | — | wsjtx |

## ambiguous

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `operation` | operation token | — | wsjtx, n1mm, fldigi, js8call |
| `window` | integer | — | wsjtx |

## lifecycle

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `state` | token | — | wsjtx, n1mm, js8call |
| `max_schema` | integer | — | wsjtx |
| `version` | text | — | wsjtx |
| `revision` | text | — | wsjtx |

## native

| Name | JSON type | Unit | Expected families |
|---|---|---|---|
| `start` | instant | UTC | n1mm profile only; lookup contact timestamp (Annex D) |
| `previous_start` | instant | UTC | n1mm profile only; lookup prior contact key (Annex D) |

Other families have no defined `native` common members.


## Consolidating receivers (informative)

A WSJT-X WSPRDecode names the transmitting station and does not say who heard it. An application
consolidating receivers first scopes `radio.instance` labels by its own transport or session
association: labels can collide between programs or networks. Within that scope, it can pair each
decode with the latest `radio_state` from the same instance to obtain the receiving station's call
and grid. This is application-maintained state; the decode does not acquire those fields from an
implicit codec join. A serialized instance label is untrusted and does not establish that association.

The same station heard by three receivers during one period represents three spots, rather than
duplicate evidence of a single receiver. A `time_of_day` supplies no date; the application supplies
the date when constructing an instant and handles midnight boundaries using its own policy.
