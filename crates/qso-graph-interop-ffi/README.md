<!-- SPDX-License-Identifier: GPL-3.0-or-later -->
<!-- vale Google.Headings = NO -->
# QG Interop C ABI
<!-- vale Google.Headings = YES -->

Use WSJT-X, N1MM Logger+, fldigi, and QGI protocol codecs from C, C++, Python,
.NET, or Node through a C ABI. The library provides shared and static libraries,
JSON events, native command encoding, byte-preserving re-encoding, and
caller-buffered QGI TCP framing. Your app controls transport and policy.

## Installation

Add `include/qso_graph_interop.h` and the matching platform library to your
app's include and link paths. Consumers need no Rust toolchain or runtime.
For source builds, run this command from the repository root:

```sh
cargo build -p qso-graph-interop-ffi --release --locked
```

Artifacts are in `target/release`: `.so` or `.a` on Linux, `.dylib` or `.a` on
macOS, and `.dll` with an import library or a static `.lib` on Windows MSVC.

## First use

Check `qgi_abi_version() == QGI_ABI_VERSION`, create a codec with
`qgi_codec_new`, then call `qgi_decode` with native bytes and caller context JSON.
Read the returned JSON outcome using its explicit byte length. Release every
returned buffer with `qgi_buffer_free` and each handle with `qgi_codec_free`.
Handles can be shared across threads; operations are serialized internally.

See the [C ABI guide](../../docs/C-ABI.md) for a complete C example, platform
linking commands, function contracts, error handling, and ownership rules.
