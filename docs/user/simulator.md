<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Test a receiver with the simulator

`qg-interop sim` plays WSJT-X decodes without a radio or a Python installation. Point it at the
numeric address and port where your app listens. The built-in synthetic ALL.TXT sample
contains 120 decodes in seven cycles.

```sh
qg-interop sim --wsjtx --to 127.0.0.1:2237
qg-interop sim --wsjtx --to 127.0.0.1:2237 --file ALL.TXT --speed 60
qg-interop sim --wsjtx --to '[::1]:2237' --call W1ZZZ --grid AA00 --loop
```

Use `--from YYMMDD_HHMMSS` to start at a timestamp in the log. Playback follows file order,
skips transmitted lines, other non-decodes, and modes without an established period. It reports
the skipped count and names the modes with missing periods. Both aligned WSJT-X logs
and single-spaced logs work. Blank lines and received decodes before `--from` aren't counted as
skipped. Each Decode carries the live cycle time. Speed changes the waiting time while cycle
stamps keep their original pace. A gap between logged timestamps doesn't insert empty cycles.

Each cycle sends Heartbeat, decoding-start Status, its Decodes, and decoding-end Status. The
simulator waits one mode period before the next cycle, including after the last cycle. `--speed`
accepts values greater than zero and at most 1000. `--loop` starts the log again with continuing
cycle times. Stop with Ctrl+C; the simulator sends Clear followed by Close. A termination signal
also triggers that sequence.

| Option | Behavior |
| --- | --- |
| `--wsjtx` | Play ALL.TXT as WSJT-X UDP messages. |
| `--to IP:PORT` | Required numeric IPv4 or IPv6 destination; port 1–65535. Bracket IPv6 addresses. |
| `--file PATH` | Read this local ALL.TXT instead of the embedded sample. |
| `--from YYMMDD_HHMMSS` | Include received decodes at or after this timestamp. |
| `--id NAME` | Instance Id, 1–64 characters; default `WSJT-X`. |
| `--call CALL` | Status callsign, up to 13 letters, digits, or slashes; converted to uppercase. Default empty. |
| `--grid GRID` | Status Maidenhead locator: two uppercase A–R letters, two digits, optionally two uppercase A–X letters. Default empty. |
| `--speed N` | Scale waiting time; default 1. |
| `--loop` | Repeat playback. |

A receiver can send Replay back to the simulator's source address and port. Only the configured
receiver's exact address and port can issue commands, using the simulator's Id. Replay sends all
decodes played so far, marked old, then Status. Clearing Band Activity clears that Replay window.
Close requests graceful exit. Changed settings produce Status; there is no separate command
acknowledgment. See [reaction details and validation](../simulator-validation.md).

## Play an NDJSON capture

Capture the native messages with `listen` or `decode`, then play each event's `native.data`:

```sh
qg-interop listen --wsjtx --count 20 > capture.ndjson
qg-interop sim --capture capture.ndjson --to 127.0.0.1:2237 --speed 100
qg-interop sim --capture capture.ndjson --qgi --to 127.0.0.1:2237
```

Native capture playback sends the recorded bytes unchanged. `--qgi` instead sends each complete
event as one qgi UDP datagram, including carried commands. It doesn't execute those commands.
The native UDP transport catalog includes WSJT-X, N1MM, and PSK Reporter IPFIX.
TCP and MQTT native captures require their own transport helpers.

An optional top-level `received_at` extension, spelled as `YYYY-MM-DDTHH:MM:SS.mmmZ`, supplies
receive timing. Adjacent records with timestamps retain their relative delay; a pair missing
either timestamp uses a one-second interval. Backward receive times are rejected. Current
`listen` and `decode` output has no receive-time extension, so it uses that fixed interval.
Speed and looping also apply to captures. Capture playback sends only the recorded datagrams;
it doesn't append WSJT-X shutdown messages or respond to recorded commands.

## Playback limits

ALL.TXT files have a 64 MiB limit, a 16 KiB line limit, and a 100,000-decode limit. The Replay window
has a 64 MiB budget and a 100,000-decode limit, including repeated cycles. Reaching a limit exits
with an error and sends Clear then Close. Invalid UTF-8 in a log is replaced, as in the Python reader.
Numbers outside native field ranges are skipped as invalid lines. Capture files have the same
64 MiB and 100,000-record limits, with at most 1 MiB per NDJSON record and 65,507 bytes per UDP
payload. Input validation finishes before opening a sender socket.

Confirmed fixed periods are FT8 at 15 seconds, FT4 at 7.5 seconds, and JT4/JT65 at 60 seconds.
The recorded `FT8_SH` label also uses FT8 timing.
Q65 plays when its recorded label includes the period and submode, such as `Q65-60C`.
Bare `Q65`, JT9, MSK144, FST4, and FST4W lack a recorded period setting and are skipped.
The simulator does not infer a period from gaps between timestamps. WSPR and Echo have known
periods but separate log layouts; they are skipped as non-decodes by the ALL.TXT player.
Use native capture playback for WSPRDecode messages.
[Protocol evidence](../PROTOCOL.md#simulator-timing-and-reactions) lists the periods and released sources.

A mixed log plays its supported cycles and reports a summary such as
`2 lines skipped: Tx, not a decode, or a mode without an established period: JT9, Q65`.
If nothing playable remains, playback fails with `no received decodes` and the skipped modes.

## Existing fixed QSO scripts

`--dx-call`, `--mode`, or `--frequency` selects the original finite nine-message script. Its
original defaults remain `N0CALL`, `FN31`, `W1AW`, `FT8`, and 14,074,000 Hz. It includes QSOLogged
and runs immediately. Use these options without `--file` or `--capture`.

```sh
qg-interop sim --wsjtx --to 127.0.0.1:2237 --mode FT4 --frequency 7074000
```
