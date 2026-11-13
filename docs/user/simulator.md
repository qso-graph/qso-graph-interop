<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Test a receiver with the simulator

`qg-interop sim` generates N1MM contest traffic and plays WSJT-X decodes without a radio or a Python
installation. Point it at the numeric address and port where your app listens.

## Play WSJT-X traffic

The built-in synthetic ALL.TXT sample
contains 120 decodes in seven cycles.

```sh
qg-interop sim --wsjtx --to 127.0.0.1:2237
qg-interop sim --wsjtx --to 127.0.0.1:2237 --file ALL.TXT --speed 60
qg-interop sim --wsjtx --to '[::1]:2237' --call Q1ZZZ --grid AA00 --loop
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

## Play N1MM contest traffic

Test a contest dashboard, logger bridge, or band map without running N1MM Logger+:

```sh
qg-interop sim --n1mm --to 127.0.0.1:12060 --speed 60
qg-interop sim --n1mm --to '[::1]:12060' --call Q1ZZZ --station SYNTHETIC-02 --radios 2 --rate 240 --speed 60 --seed 42 --count 35
qg-interop sim --n1mm --to 127.0.0.1:12060 --radios 2 --loop
```

Each run starts with `AppInfo`, radio reports, and a zero score. `RadioInfo` follows every
simulated second and at contact transitions. New `contactinfo` messages follow the chosen rate.
Every fifth contact gets a `contactreplace` with its previous callsign and timestamp. Every
seventh contact gets a `contactdelete`. Each contact also produces a spot addition and deletion.
`dynamicresults` follows each contact change with the current count and points: two points per
contact, three after a replacement, and no points for deleted contacts. The synthetic contest
uses points as its total score, with no multipliers. All generated callsigns and station labels
are synthetic. Set `--call` and `--station` to use your own test identities.

With two radios, focus and active radio alternate between 1 and 2 for successive contacts.
Radio 1 runs and radio 2 searches and pounces. Only the active radio transmits, and each
contact's `radionr` matches that radio. Initial frequencies are 14.025 MHz and 7.025 MHz, in CW.

| Option | Behavior |
| --- | --- |
| `--n1mm` | Select the scripted N1MM Logger+ peer. |
| `--to IP:PORT` | Required numeric IPv4 or IPv6 destination, port 1–65535. |
| `--call CALL` | Station and operator callsign, up to 13 ASCII letters, digits, or slashes. Converts to uppercase. Empty or omitted uses `N0CALL`. |
| `--station NAME` | Computer label, 1–64 printable ASCII characters with a non-space character. Default `SYNTHETIC-01`. |
| `--radios 1\|2` | One radio by default. Select 2 for single operator, two radio (SO2R) traffic. |
| `--rate QSOS_PER_HOUR` | Integer new-contact rate, 1–3600. Default 120. Edits and deletions don't count toward this rate. |
| `--speed N` | Scale waiting time, with 0 < N ≤ 1000. Default 1. |
| `--seed N` | Unsigned 64-bit seed, 0–18446744073709551615. Default 1. |
| `--loop` | Continue until interrupted, unless `--count` supplies a limit. |
| `--count N` | Stop after 1–100000 new contacts, including with `--loop`. Without either option, send 12 new contacts. |

The logical clock starts at `2026-01-01 00:00:00` UTC. The first contact arrives after
`3600 / rate` simulated seconds. Timestamps use N1MM's unzoned UTC format, `YYYY-MM-DD HH:MM:SS`.
Radio and contact frequencies use integer 10 Hz units, and spots use decimal kHz.
Speed changes only the waiting time. The same seed and options reproduce the same bytes,
including timestamps, when incoming commands are the same.

Send `radio_setfrequency` to the simulator's source address and port from the exact configured
target address. The selected radio's next scheduled `RadioInfo` reports the new frequency.
Later contacts and spots use that frequency too. Tuning uses 10 Hz resolution, dropping any
single-Hz remainder. `Spectrum` input changes no outgoing traffic. Neither input produces an
acknowledgment. This follows the documented QSY and display effects in
[Annex D's pinned source S1](../../spec/annex-d-n1mm.md#d5-commands-from-fields),
**Setting N1MM's Frequency** and **Sending Spectrum Data**.

Ctrl+C or a termination signal stops playback and sends final radio reports with transmission
cleared. There is no N1MM Close message. Use N1MM options without WSJT-X or capture selection.
See [validation and precision policy](../simulator-validation.md#n1mm-contest-simulator).

## Test fldigi controllers

Test an app that controls fldigi without starting fldigi. Start the stand-in in one
terminal, then run the poller in another:

```sh
qg-interop sim --fldigi
qg-interop poll --fldigi --count 3
```

The server uses HTTP/1.1 XML-RPC at `127.0.0.1:7362`. Point your controller's fldigi XML-RPC
connection at that address. Choose another numeric loopback address when needed:

```sh
qg-interop sim --fldigi 127.0.0.1:17362 --speed 10 --seed 42 --call Q7ABC --grid EM10
qg-interop poll --fldigi 127.0.0.1:17362 --count 3
```

`--speed` scales received text and queued transmission together, from more than zero through
1,000. `--seed` chooses a reproducible Q-prefixed peer call. `--call` and `--grid` customize the
synthetic script and initialize the contact call and locator fields. Defaults are `Q1AAA`,
`FN31`, BPSK31, 14,070,000 Hz, and a 1,500 Hz audio carrier. Changing contact fields through
XML-RPC changes their getters; it doesn't rewrite the script. Ctrl+C stops the server.

All 50 methods in [Annex E's mapping table](../../spec/annex-e-fldigi.md#e4-complete-mapping-catalog)
are served. RF and audio setters return their old values. Modem names and ADIF mode/submode
remain distinct. Unknown modem names return `No such modem`. Unmapped XML-RPC methods receive
a fault with code `-1` and text `<method>: unknown method name`.

Append text with `text.add_tx`, then call `main.tx`. Transmission consumes the queue at an
approximate rate for the mode and returns to receive when the queued text finishes.
BPSK31 uses about 3.1 bytes per second; BPSK63 and BPSK125 use twice and four times that rate.
RTTY uses about 6 bytes per second, CW 2, and MFSK16 3.9. Other modes use an approximate
3 bytes per second unless listed in the [implementation policy](../PROTOCOL.md#fldigi-simulator).
The text simulation returns to receive automatically. It produces no audio and uses no rig hardware.
Calling `main.tx` with an empty queue sends no text and waits for text, `main.rx`, or `main.abort`.
`main.rx` and `main.abort` stop consumption; unsent text remains until resumed or cleared with
`text.clear_tx`. `main.tune` reports tune until stopped.

The finite received script accumulates while receiving. `text.get_rx_length` reports its byte
length, and `text.get_rx(start, length)` reads zero-based byte ranges without consuming them.
Use `(0, -1)` to read all available text. `rx.get_data` returns bytes since its previous query
and drains that separate raw buffer. `text.clear_rx` clears the visible buffer; it doesn't drain
raw data or restart the script. Handle buffer shrinking when maintaining a cursor.

The bundled fldigi 4.2.13 server returns an empty string for nominal-nil methods, including
`text.add_tx`, `main.tx`, and `main.abort`. The stand-in preserves that response. The codec
observes it as `native`; a client that requires a mapped nil acknowledgment can't advance
its multi-request sequence from that reply. The stand-in accepts the ordered requests from
`Codec::encode_commands` so controllers can test the bundled behavior explicitly.

For the empty logbook, `logbook.last_record` returns `NONE` and `logbook.all_records` returns
the released ADIF header. `log.get_band` follows rig frequency. Other log fields use
fixed synthetic defaults. Received data pauses during transmission or tune. Timing approximates
text throughput and doesn't reproduce modem DSP, GUI scheduling, macros, or hardware effects.
See [source evidence and bounds](../PROTOCOL.md#fldigi-simulator) for malformed requests,
transport limits, and range boundaries. Use fldigi options without UDP or capture options.

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
defaults are `N0CALL`, `FN31`, `Q1AW`, `FT8`, and 14,074,000 Hz. It includes QSOLogged
and runs immediately. Use these options without `--file` or `--capture`.

```sh
qg-interop sim --wsjtx --to 127.0.0.1:2237 --mode FT4 --frequency 7074000
```
