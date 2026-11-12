<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex C: JS8Call placeholder

Reserved family: `js8call`. This is a scope placeholder, not a complete mapping or conformance claim.

| Item | Scope to be completed |
|---|---|
| Native message | One JSON line without its line delimiter. |
| Framing | Newline-delimited JSON over a TCP stream; exact delimiter, buffering, maximum size and recovery rules to be pinned. Uses `QGI-CLS-0002`. |
| Sending direction | API requests to JS8Call; responses and notifications from it. Transport and connection ownership remain with the caller. |
| Source to pin | Released JS8Call-improved source plus its [API documentation v3.0.0](https://js8call.com/JS8Call-improved/d7/d15/md_docs_2API.html), consulted 2026-11-12. Compare the original v2.3.1 before assigning a mapping revision (`QGI-JS8-0001`). The live documentation identifies a master build and is not a released-source pin. |

`RIG.SET_FREQ`, `TX.SET_TEXT`, `TX.SEND_MESSAGE` and `RIG.TX_HALT` inform the provisional
Annex A operation catalog. Required parameters, IDs, method availability and exact staging versus
sending semantics remain to be confirmed in this annex. No encoding or field assignment is
established by this placeholder.
