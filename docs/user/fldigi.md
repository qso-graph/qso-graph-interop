<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# Use fldigi

QG Interop decodes fldigi XML-RPC requests and responses into specification events.
It also polls a local fldigi instance for radio state, contact fields, and received text.
Install the command-line tool from the checkout:

```sh
cargo install --path crates/qso-graph-interop-cli --locked
```

## Poll local state and received text

Enable fldigi's XML-RPC server on loopback. Its documented default port is **7362**;
see `--xmlrpc-server-port` in `doc/fldigi.1.txt` of the
[released 4.2.13 archive](https://www.w1hkj.org/files/fldigi/fldigi-4.2.13.tar.gz).
The same release defines `DEFAULT_XMLRPC_IP_PORT` as `7362` in `src/include/data_io.h`.

```sh
qg-interop poll --fldigi
qg-interop poll --fldigi 127.0.0.1:7362 --interval 500 --count 3
qg-interop poll --fldigi '[::1]:7362' --count 1
```

`--interval` is the delay in milliseconds between complete cycles, from 1 through
86,400,000; it defaults to 1,000. `--count` counts cycles; zero polls until interrupted.
Each cycle queries these methods separately:

| Method | Common observation |
| --- | --- |
| `rig.get_frequency` | Dial frequency in hertz |
| `modem.get_carrier` | RX audio offset in hertz |
| `modem.get_name` | Native modem name |
| `modem.get_submode` | Native submode |
| `main.get_trx_state` | Transmitting for TX or RX |
| `main.get_trx_status` | Transmitting for tx, tune, or rx |
| `log.get_call` | Contact callsign |
| `log.get_locator` | Contact grid |
| `rx.get_data` | RX text since the preceding query |

Standard output contains one specification NDJSON event per response, with direction
`from_application`, exchange `response`, and a distinct `correlation_id` matching the
poll request. The native view identifies the associated method. Faults produce error
results. Connection, framing, or codec failures stop polling and go to standard error.
The poller sends no setters or transmit commands.

Only numeric addresses in IPv4's 127.0.0.0/8 range or IPv6's ::1 are accepted.
Names such as `localhost` are rejected. fldigi XML-RPC uses plain HTTP, so the client
restricts it to loopback to keep unencrypted state and RX text off outside networks.
Each request has a five-second total timeout, at most 16 KiB of response headers,
and a 256 KiB body limit. Chunked replies allow at most 1,024 chunks, with 128-byte
chunk lines and 16 KiB of trailers. There are no automatic retries or redirects.

RX chunks are separate observations. Multiple clients can consume the same app
buffer differently; the codec does not join split characters, deduplicate text, or infer
a completed QSO. Callsigns, grids, and timestamps are never parsed from received text.

## Decode a captured body

Files contain one XML-RPC body. Exclude HTTP headers and transfer framing.
Use a request's own method name, or explicitly associate a response:

```sh
qg-interop decode --fldigi request.xml
qg-interop decode --fldigi response.xml --request-method rig.get_frequency
qg-interop decode --fldigi response.xml --request-file request.xml
qg-interop decode --fldigi --request-method modem.get_name - < response.xml
```

Responses contain no method name. The command-line tool therefore requires `--request-method` or
`--request-file` for responses, including faults. A request file also supplies byte
range arguments for `text.get_rx`. Capture decoding keeps direction `unknown`.
The library can decode unassociated responses as native evidence; valid faults always
map to error results.

## Use the library

Enable the default `fldigi` feature. Supply one body and caller context through the
public codec traits. To associate a response and retain correlation:

```rust
use qso_graph_interop::{Codec, codec::DecodeOptions, fldigi::Fldigi};
use serde_json::json;

let mut codec = Fldigi::with_request(json!({"method":"rx.get_data"}));
let mut options = DecodeOptions::default();
options.context = json!({"direction":"from_application", "exchange":"response",
    "correlation_id":"synthetic-Q1AAA-1"});
let body = b"<methodResponse><params><param><value><base64>Q1EgUTFBQUE=</base64></value></param></params></methodResponse>";
let event = codec.decode(body, &options)?.events.remove(0);
assert_eq!(event.value()["fields"]["text"], "CQ Q1AAA");
```

Alternatively, set `options.native["request"]` to an object with `method` and optional
`arguments`. Use `verify_with` and `reencode_with` to supply that association for
per-call consistency checking. `Event::recognized_with` and `json::write_with`
use the same options for compatible reading and strict publication. A `text.get_rx` response needs its request's two
integer arguments, for example `{"method":"text.get_rx","arguments":[0,64]}`.
Without association, method-dependent verification is unavailable. The structured
view never substitutes for caller association.

Use `Codec::encode_commands` to construct commands from fields. `send_text` requires
string `text` and boolean `transmit`; it returns `text.add_tx` first, then `main.tx`
only for true. `set_frequency` requires integer `frequency_hz` and optionally
`rx_offset_hz`; it returns `rig.set_frequency` first, then `modem.set_carrier` if
requested. These positive integers are bounded by 2,147,483,647. `halt_tx` takes no
arguments and returns `main.abort`. Family operations use Annex E's exact arguments.

Send each following request only after the preceding request returns its matching
success type: double for `rig.set_frequency`, nil for `text.add_tx`. Stop on a fault,
wrong type, lost association, or transport failure. In particular, an empty-string
response is not a nil acknowledgment and must not trigger the next request. Supply
separate association and correlation for each request. The sequence has no atomicity,
rollback, or retry guarantee.

An empty append preserves previously queued text. Appended text does not guarantee
on-air content. Command strings reject CR and XML-forbidden characters. Observations
preserve original bytes and invalid text. The library opens no sockets and never
executes controls; the app owns transport and permission to transmit.

See [Annex E](../../spec/annex-e-fldigi.md), [protocol evidence](../PROTOCOL.md),
and [SECURITY.md](../../SECURITY.md) for acceptance limits and validation evidence.
