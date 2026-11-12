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
frequency unit, radio selection and response behavior must be confirmed here before encoding is
specified. Other native fields are not inferred from superficially similar ADIF names.
