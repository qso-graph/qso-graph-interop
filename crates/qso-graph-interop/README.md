<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
# QG Interop library

Encode and decode WSJT-X UDP messages with bounded Rust parsers.
Applications choose which messages to send and supply their own sockets.

```rust
use qso_graph_interop::wsjtx::{self, Body, Outcome, Packet};

let original = Packet::new("WSJT-X", Body::Replay);
let bytes = wsjtx::encode(&original)?;
match wsjtx::decode(&bytes)? {
    Outcome::Message(packet) => assert_eq!(*packet, original),
    Outcome::Skipped { .. } => unreachable!(),
}
# Ok::<(), wsjtx::Error>(())
```

The common model keeps the original family packet, including unrecognized trailing data.
Byte arrays distinguish null, empty, absent, and invalid UTF-8 without replacing the original bytes.

Licensed under GPL-3.0-or-later.
