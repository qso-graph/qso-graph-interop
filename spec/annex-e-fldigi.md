<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex E: fldigi placeholder

Reserved family: `fldigi`. This is a scope placeholder, not a complete mapping or conformance claim.

| Item | Scope to be completed |
|---|---|
| Native message | One XML-RPC HTTP request body or response body, excluding HTTP headers. |
| Framing | The HTTP layer hands the codec a complete body; uses `QGI-CLS-0003`–`0006` and XML isolation rules. |
| Sending direction | Method calls to fldigi and method results/faults from it. |
| Source to pin | Released fldigi source and a dated, hashed [4.2.00 XML-RPC method reference](https://www.w1hkj.org/FldigiHelp/xmlrpc_control_page.html), consulted 2026-11-12, with caller-supplied capabilities for the actual build. |

The published rig/carrier frequency, text-buffer and transmit methods inform Annex A's provisional
operations. A response body may not identify its method: the caller supplies request association
and correlation. Valid faults can use `result` with `status: "error"`; malformed XML remains a
codec error. Encoding a native request does not call a server. Annex work includes settling the
single-request versus multi-request text/transmit issue recorded in Annex A.

Radio parts have no established mapping in this placeholder (`QGI-MOD-0014`); their native
source and per-kind availability remain annex work. The family output limit and expansion proof
required by `QGI-MAP-0003` are also unestablished; no conformance is claimed for an assumed limit.
