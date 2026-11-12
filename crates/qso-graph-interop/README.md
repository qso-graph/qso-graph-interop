<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- vale Google.Headings = NO -->
# QG Interop
<!-- vale Google.Headings = YES -->

QG Interop is a Rust library for applications that exchange amateur radio events and commands.
It implements QGI-SPEC-001 0.1.0 for WSJT-X schemas 1–3, N1MM Logger+ XML, and QGI event carriage over UDP and TCP.
Callers supply native bytes and context, and receive specification events with preserved evidence.
Applications own transport, destinations, session identity, and control policy.

Install with `cargo add qso-graph-interop`. Add `serde_json` with its `float_roundtrip` feature
for the JSON construction in the example below. The default features are `wsjtx`, `n1mm`, `qgi`, and `serde`.
Use `default-features = false` for the core without protocol families. Enable `conformance` to use the
vector runner, manifest hashing, round-trip, prefix, and fuzz helpers for another codec.

Build a command from common fields and an explicit wire target:

```rust
use qso_graph_interop::{Context, Event, Text};
use serde_json::json;
let fields = json!({"operation": "send_text", "text": "CQ Q1AAA FN42", "transmit": false});
let command = Event::command("wsjtx", 1, Context::default(),
    fields.as_object().unwrap().clone(), Some(Text::from("synthetic-instance")))?;
assert!(command.is_command());
# Ok::<(), qso_graph_interop::Error>(())
```

With `wsjtx`, pass the command to `Codec::encode_command` on `wsjtx::Wsjtx`.
Use `Codec::decode` for observations and `Codec::reencode` to encode their native evidence.
`Event::native_bytes` retrieves original bytes. These preserve noncanonical booleans and extensions.
`json::parse` reads bounded event JSON; `json::write` admits compact records before publication.
`Event::recognized` returns a compatible view and a separate native verification status.

With `n1mm`, use `n1mm::N1mm` through the same traits. For example, build a frequency command
from integer hertz and a radio selector:

```rust
# #[cfg(feature = "n1mm")] {
use qso_graph_interop::{Codec, Context, Event, codec::EncodeOptions, n1mm::N1mm};
use serde_json::json;
let fields = json!({"operation":"set_frequency", "app":"synthetic-client",
    "radio":{"number":2}, "frequency_hz":14053284});
let command = Event::command("n1mm", 1, Context::default(),
    fields.as_object().unwrap().clone(), None)?;
let bytes = N1mm::default().encode_command(&command, &EncodeOptions::default())?;
assert!(std::str::from_utf8(&bytes).unwrap().contains("<frequency>14053.284</frequency>"));
# }
# Ok::<(), qso_graph_interop::Error>(())
```

N1MM also encodes `n1mm.spectrum` from fields and re-encodes observations byte for byte after
checking their native evidence. See the [N1MM guide](../../docs/user/n1mm.md).

The public `Codec` and `Profile` traits support third-party families without a fork. Stream codecs
consume a caller-owned buffer; stateful codecs receive a caller session key and expiry input and
support resetting one session or all sessions. No codec opens a socket or reads a clock.

See [the user guides](../../docs/user/) and [protocol documentation](../../docs/PROTOCOL.md).
Licensed under GPL-3.0-or-later.
