<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QSO Graph UDP library

Encode and decode WSJT-X and JTDX UDP messages with bounded Rust parsers.
Applications choose which messages to send and supply their own sockets.

```rust
use qso_graph_udp::wsjtx::{self, Body, Outcome, Packet, Variant};

let original = Packet::new("WSJT-X", Body::Replay);
let bytes = wsjtx::encode(&original)?;
match wsjtx::decode(&bytes, Variant::Wsjtx)? {
    Outcome::Message(packet) => assert_eq!(*packet, original),
    Outcome::Skipped { .. } => unreachable!(),
}
# Ok::<(), wsjtx::Error>(())
```

The common model keeps the original family packet, including unrecognized trailing data.
Byte arrays distinguish null, empty, absent, and invalid UTF-8 without replacing the original bytes.

Licensed under GPL-3.0-or-later.
