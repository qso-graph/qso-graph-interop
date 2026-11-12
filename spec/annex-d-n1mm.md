<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex D: N1MM Logger+ placeholder

Reserved family: `n1mm`. This is a scope placeholder, not a complete mapping or conformance claim.

| Item | Scope to be completed |
|---|---|
| Native message | One XML UDP datagram payload. |
| Framing | Datagram boundary, not an XML element or text line; uses `QGI-CLS-0001` and XML isolation rules. |
| Sending direction | External broadcasts from N1MM Logger+; inbound `radio_setfrequency` to it. |
| Source to pin | A dated, hashed copy of [External UDP Messages](https://n1mmwp.hamdocs.com/appendices/external-udp-broadcasts/) and its XML examples, consulted 2026-11-12. |

The published inbound frequency message informs shared `set_frequency` in Annex A. Its native
frequency unit, radio selection and response behavior need confirmation here before encoding is
specified. Other native fields are not inferred from superficially similar ADIF names.

## Radio and SO2R field evidence (provisional)

**`QGI-N1M-0002` — Radio labels.** When Annex D is pinned, the mapping MUST take
`radio.instance` from native `StationName`, and `radio.number` from native `RadioNr` on radio
messages or `radionr` on contacts, only where the respective native part is carried. A two-radio
instance MUST NOT be split by inventing instance names; a multi-operator station's `StationName`
is a label, never authentication or `source_id` (`QGI-MAP-0004`). Spot/decode and other radio-bearing
kinds MUST omit parts absent from their native message; no remembered station/radio fills them.

**`QGI-N1M-0003` — Provisional SO2R state.** The intended `radio_state` assignments below are
**to be confirmed in Annex D** against a dated, hashed External UDP Broadcasts appendix and examples
before this family claims conformance. An implementation MUST NOT guess the integer/boolean
wire spelling or missing-value behavior from this capacity table.

| Native field in External UDP Broadcasts appendix | Common member | Status |
|---|---|---|
| `FocusRadioNr` | `focus_radio` (integer) | to be confirmed in Annex D |
| `ActiveRadioNr` | `active_radio` (integer) | to be confirmed in Annex D |
| `IsTransmitting` | `transmitting` (boolean) | to be confirmed in Annex D |
| `IsRunning` | `running` (boolean) | to be confirmed in Annex D |
| `IsStereo` | `stereo` (boolean) | to be confirmed in Annex D |

The appendix URL above is the field-evidence reference supplied for this review. It has no dated
hash pin in this repository; this pass did not fetch it. Native units, value encodings, exact
per-message availability, limits and output expansion remain open profile work.
