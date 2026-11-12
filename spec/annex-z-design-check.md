<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Annex Z: informative extension design check

This annex checks the core's capacity, not protocol conformance. These four families are designed
for, not yet mapped, and outside the first release. Names below are proposed annex-local
operations, not new registered families.
An implementer first obtains a controlled extension-family name under `QGI-EXT-0002` and pins the
appropriate released definitions under `QGI-MAP-0001`. No wire layout, numeric error catalog or
product interoperability is established by these sketches. The native protocols in the task are
the design inputs; no additional network services were contacted to test them.

## Z.1 Hamlib rigctld

**Native message.** One complete text command or one complete response on the TCP stream. A
response can contain several lines, so a transport chunk or an arbitrary single line cannot be
the response boundary. The eventual annex selects the normal or extended response syntax,
specifies how request knowledge delimits a reply, and pins delimiters, limits and error returns.
`native.data` preserves that complete message excluding only its outer delimiter.

**Kinds and fields.** A decoded frequency setter is an observed `control` with
`fields: {"operation":"set_frequency","frequency_hz":14074000}` and native evidence. A frequency
getter is a `control` with a family-prefixed `get_frequency` operation. Its associated successful
reply is a `result` with `status: "ok"`, that operation and `frequency_hz`. A native error return
(such as rigctld's `RPRT` status response) is a `result` with `status: "error"`, exact code spelling
in `native_code`, and `operation` only when the caller supplies the associated request. The full
return remains in native evidence; no numeric code is translated into a fabricated common error.
A successful acknowledgment similarly has `status: "ok"` and its native code, if supplied.
An unassociated scalar response stays `native` because it does not identify the getter.

**Commands.** An application constructs `set_frequency` with `frequency_hz` and no `native`;
the annex supplies native syntax. A frequency getter uses `<family>.get_frequency`. Target rig,
VFO or other required selectors become annex-defined fields, with controlled extension names
where appropriate; a connection endpoint is not a `wire_id` unless the protocol carries one.

**Coverage.** `QGI-CLS-0002`–`0005` cover stream and request/response boundaries, association and
correlation. `QGI-MOD-0038` distinguishes an application rejection from a malformed codec input.
`QGI-SEC-0008` requires a recovery boundary or caller reset for an unframeable response; it no longer
incorrectly assumes every reply is exactly one delimiter-terminated line. A future annex must pin
normal/extended syntax, multi-line replies and method-dependent parsing, with negative vectors.

## Z.2 DX cluster spot over telnet

**Native message.** One complete spot line excluding its line terminator. Telnet negotiation is
transport, outside the spot codec; the annex states that input is the resulting application data
stream, not arbitrarily cut TCP chunks. Other cluster output lines need their own type rules,
not a guess that every line is a spot.

**Kinds and fields.** A recognized spot line maps to `spot`: `spotter_call`, `call`,
`frequency_hz`, `text`, and the actual time representation supplied by the line. A UTC clock with
no date maps to `time_of_day`, not an invented dated `time`. A mode is present only if a pinned
structural field supplies it; an incidental word in the comment does not establish mode.
For example, a synthetic spot of W1AAA by K1AAA at 14,074,000 Hz can have
`{"call":"W1AAA","spotter_call":"K1AAA","frequency_hz":14074000,"text":"synthetic spot"}`.
If the line only declares minute precision, the annex must establish a compatible common-time
representation or leave the common time absent under `QGI-MOD-0006`; it cannot imply measured seconds.
Exact original spacing, comment and clock spelling remain in `native.data`.

**Operations.** None are needed for receiving spot lines. A later annex may add a family-prefixed
spot-submission command if a pinned sending syntax exists; login, telnet negotiation and session
establishment remain transport. The shared `send_text` action is not an excuse to interpret an
arbitrary server command string.

**Coverage.** `QGI-CLS-0002`, `QGI-MOD-0006`, Annex A `spot`, `QGI-MAP-0002` and
`QGI-SEC-0008` cover framing, missing dates, structural parsing, unknown lines and recovery.
No cluster dialect, frequency unit conversion or timestamp grammar is assumed by the core.

## Z.3 Logged QSO as ADIF over UDP

**Native message.** One UDP datagram containing an ADIF logging payload, as in the task's JTAlert,
GridTracker and Log4OM use case. An eventual annex pins each product's wrapper/ADIF variant; these
product names do not imply identical datagram grammars. A datagram can contain multiple records,
so its complete bytes can be repeated in each event's `native.data` while the annex emits the
records in their declared order. No extra native envelope member is necessary.

**Kinds and fields.** `qso_logged` has verbatim `adif` and, if the pinned mapping parses the ADIF
record, structurally supplied `call`, `grid`, `mode`, `station_call`, `start`, `end`, reports and
frequency. A synthetic W1AAA QSO can expose `call: "W1AAA"` and `mode: "FT8"`; a bare logging
record does not imply a remote-control request. Exact tags, tag lengths, optional values, unknown
tags, capitalization and extensions remain in native evidence. A decimal native frequency needs
an annex-defined exact conversion to integer hertz, never an inferred frequency unit or rounding.
Unresolved dates/zones and unsupported common precision remain native only.

**Operations.** No control operation is needed for an observed log notification. Reconstructing
an observed datagram is the separate native re-encoding operation. A future application-built
log-submission action would be a family-prefixed control with its own structured arguments;
`qso_logged` by itself does not authorize sending or logging anything.

**Coverage.** `QGI-CLS-0001`, `QGI-MAP-0006`, `QGI-MOD-0005`–`0008`, Annex A `qso_logged`
and `QGI-SEC-0010` cover datagram boundaries, multiple events, lossless tags and consistency.
`QGI-WSJ-0027` forbids parsing WSJT-X LoggedADIF in Annex B only; it does not prohibit another
family annex from explicitly mapping structured ADIF tags.

## Z.4 APRS in a KISS frame

**Native message.** One complete KISS frame, excluding its outer frame delimiter while preserving
the command/port octet and every encoded escape byte. A complete APRS-bearing data frame is not
the same input boundary as decoded APRS text or a bare AX.25 payload. The annex pins KISS framing,
escaping, frame types and APRS/AX.25 definitions; it does not infer a packet type from station names.

**Kinds and fields.** An APRS message with explicit text may use `decode.text` and an explicitly
encoded originating call as `decode.call`. An arbitrary position, telemetry or unknown APRS packet
can use `native` with `fields: {}`; its structured `native.message` can expose AX.25 addresses,
path, KISS port, APRS type, coordinates or telemetry without adding native envelope members or
forcing those values into an unrelated common field. Unknown packet content is still exact and
self-contained. This proves carriage; it does not claim a registered shared geolocation model.

**Operations.** Receiving needs none. If a future annex specifies sending, `<family>.send_packet`
can take controlled extension arguments for destination, path, port and packet content. It is a
`control` command without `native`, whose annex writes the complete frame from those arguments.
Plain message text could use shared `send_text` only if the annex establishes that exact action
and any required destination arguments. KISS non-data commands need their own mapped operations
or an uninterpreted native event, not accidental APRS decoding.

**Coverage.** `QGI-CLS-0002` preserves encoded framing content, `QGI-SEC-0008` requires escaped
stream recovery and bounds, and `QGI-CLS-0013` keeps family details inside the optional structured
view. `QGI-MAP-0010` permits field-built commands, including structured extension arguments.
No new kind or native member is needed for the stated packet-carriage use case.

## Z.5 Findings applied to the core

The first-pass core needed two changes exposed by this check, both now made:

- Add `result`, `status`, `native_code`, optional operation and result value fields
  (`QGI-MOD-0038`, Annex A and Annex F), so rigctld errors are observations of a program response.
- Generalize request/response and stream boundaries/recovery (`QGI-CLS-0002`–`0005`,
  `QGI-SEC-0008`), so a reply can contain several lines and an escaped binary frame can resynchronize.

The task's separate zero-or-more-event change also covers multiple ADIF records without another
core revision. The final core as written carries all four cases through annex-defined parsing,
field assignment, operations and vectors. Completing their protocol pins and dialect choices is
future annex work; none requires a new transport in a pure codec or a claim of live compatibility.
