// SPDX-License-Identifier: GPL-3.0-or-later
//! Exercise exports exactly as foreign callers do, including invalid tokens.
use super::*;
use std::ptr::{null, null_mut};

fn new(family: &str) -> QgiCodec {
    let mut id = 0;
    assert_eq!(
        // SAFETY: family is readable; id is aligned writable disjoint storage.
        unsafe { qgi_codec_new(family.as_ptr(), family.len(), &mut id) },
        QGI_OK
    );
    id
}
fn take(buffer: QgiBuffer) -> Vec<u8> {
    assert_ne!(buffer.token, 0);
    // SAFETY: buffer came from a successful ABI call and has not been freed.
    let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) }.to_vec();
    assert_eq!(qgi_buffer_free(buffer), QGI_OK);
    bytes
}

#[test]
fn exported_buffer_survives_registry_moves_and_other_frees() {
    let mut buffers = Vec::new();
    for _ in 0..40 {
        let mut out = QgiBuffer::default();
        assert_eq!(
            // SAFETY: out is live aligned writable storage.
            unsafe { qgi_version(&mut out) },
            QGI_OK
        );
        buffers.push(out);
        // Read every older pointer after insertions, including B-tree splits.
        for buffer in &buffers {
            // SAFETY: each descriptor still owns a live immutable ABI buffer.
            let bytes = unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) };
            assert_eq!(bytes, env!("CARGO_PKG_VERSION").as_bytes());
        }
    }
    // Removal can move other entries, too; remaining readers stay valid.
    while let Some(buffer) = buffers.pop() {
        assert_eq!(take(buffer), env!("CARGO_PKG_VERSION").as_bytes());
        assert_eq!(qgi_buffer_free(buffer), QGI_BAD_BUFFER);
    }
}
fn decode(id: QgiCodec, bytes: &[u8], options: &Value) -> Value {
    let mut out = QgiBuffer::default();
    let options = serde_json::to_vec(options).unwrap();
    assert_eq!(
        // SAFETY: slices are live; out is aligned writable and disjoint.
        unsafe {
            qgi_decode(
                id,
                bytes.as_ptr(),
                bytes.len(),
                options.as_ptr(),
                options.len(),
                &mut out,
            )
        },
        QGI_OK
    );
    serde_json::from_slice(&take(out)).unwrap()
}
fn error() -> String {
    let view = qgi_last_error();
    if view.len == 0 {
        return String::new();
    }
    // SAFETY: same-thread view copied before the next mutating ABI call.
    String::from_utf8(unsafe { std::slice::from_raw_parts(view.data, view.len) }.to_vec()).unwrap()
}

#[test]
fn invalid_pointers_lengths_handles_buffers_and_text() {
    assert_eq!(qgi_abi_version(), QGI_ABI_VERSION);
    let id = new("wsjtx");
    let mut out = QgiBuffer::default();
    let mut count = 0;
    let mut used = 0;
    let mut handle = 99;
    // SAFETY: intentional NULLs/oversize lengths are rejected before reading;
    // all non-NULL outputs refer to aligned live disjoint storage.
    unsafe {
        assert_eq!(qgi_version(null_mut()), QGI_NULL_POINTER);
        assert_eq!(qgi_codec_new(null(), 1, &mut handle), QGI_NULL_POINTER);
        assert_eq!(handle, 0);
        assert_eq!(qgi_codec_new(null(), 0, &mut handle), QGI_INVALID_VALUE);
        assert_eq!(
            qgi_codec_new(b"\xff".as_ptr(), 1, &mut handle),
            QGI_INVALID_VALUE
        );
        assert_eq!(
            qgi_codec_new(b"wsjtx".as_ptr(), 5, null_mut()),
            QGI_NULL_POINTER
        );
        assert_eq!(
            qgi_decode(id, null(), 1, null(), 0, &mut out),
            QGI_NULL_POINTER
        );
        assert_eq!(
            qgi_decode(id, null(), 0, null(), 0, &mut out),
            QGI_TRUNCATED
        );
        assert_eq!(
            qgi_decode(id, null(), usize::MAX, null(), 0, &mut out),
            QGI_BUFFER_TOO_LARGE
        );
        assert_eq!(
            qgi_decode(id, null(), 0, null(), 1, &mut out),
            QGI_NULL_POINTER
        );
        assert_eq!(
            qgi_decode(id, null(), 0, null(), usize::MAX, &mut out),
            QGI_BUFFER_TOO_LARGE
        );
        assert_eq!(
            qgi_decode(0, null(), 0, null(), 0, &mut out),
            QGI_BAD_HANDLE
        );
        assert_eq!(
            qgi_decode(id, null(), 0, null(), 0, null_mut()),
            QGI_NULL_POINTER
        );
        assert_eq!(
            qgi_encode_command(id, null(), 1, null(), 0, 0, &mut count, &mut out),
            QGI_NULL_POINTER
        );
        assert_eq!(
            qgi_encode_command(id, null(), 0, null(), 0, 0, null_mut(), &mut out),
            QGI_NULL_POINTER
        );
        assert_eq!(
            qgi_encode_command(id, null(), usize::MAX, null(), 0, 0, &mut count, &mut out),
            QGI_BUFFER_TOO_LARGE
        );
        assert_eq!(
            qgi_reencode(id, null(), 1, null(), 0, &mut out),
            QGI_NULL_POINTER
        );
        assert_eq!(
            qgi_reencode(id, null(), usize::MAX, null(), 0, &mut out),
            QGI_BUFFER_TOO_LARGE
        );
        assert_eq!(qgi_reset(id, null(), 1), QGI_NULL_POINTER);
        assert_eq!(qgi_reset(id, null(), usize::MAX), QGI_BUFFER_TOO_LARGE);
        assert_eq!(
            qgi_reset(id, std::ptr::without_provenance(usize::MAX), 2),
            QGI_INVALID_POINTER
        );
        assert_eq!(qgi_reset(0, null(), 0), QGI_BAD_HANDLE);
        assert_eq!(qgi_reset(id, null(), 0), QGI_OK);
        assert_eq!(
            qgi_decode_stream(id, null(), 1, 0, null(), 0, &mut used, &mut out),
            QGI_NULL_POINTER
        );
        assert_eq!(
            qgi_decode_stream(id, null(), usize::MAX, 0, null(), 0, &mut used, &mut out),
            QGI_BUFFER_TOO_LARGE
        );
        assert_eq!(
            qgi_decode_stream(id, null(), 0, 2, null(), 0, &mut used, &mut out),
            QGI_INVALID_VALUE
        );
        assert_eq!(
            qgi_decode_stream(id, null(), 0, 0, null(), 0, &mut used, &mut out),
            QGI_INVALID_VALUE
        );
        assert_eq!(qgi_version(&mut out), QGI_OK);
    }
    assert_eq!(take(out), env!("CARGO_PKG_VERSION").as_bytes());
    assert_eq!(qgi_buffer_free(out), QGI_BAD_BUFFER);
    assert!(!error().is_empty());
    let ptr = qgi_last_error().data;
    assert_eq!(ptr, qgi_last_error().data);
    assert_eq!(qgi_abi_version(), 1);
    assert_eq!(ptr, qgi_last_error().data);
    assert_eq!(qgi_buffer_free(QgiBuffer::default()), QGI_OK);
    assert!(error().is_empty());
    assert_eq!(qgi_codec_free(id), QGI_OK);
    // A stale token must be checked without accessing the freed codec.
    assert_eq!(
        // SAFETY: empty inputs are allowed; out is live writable storage.
        unsafe { qgi_decode(id, null(), 0, null(), 0, &mut out) },
        QGI_BAD_HANDLE
    );
    assert_eq!(qgi_codec_free(id), QGI_BAD_HANDLE);
    assert_eq!(qgi_codec_free(u64::MAX), QGI_BAD_HANDLE);
    // Deliberately misaligned output must be rejected without dereferencing it.
    let mut words = [0_u64; 8];
    let ptr = words
        .as_mut_ptr()
        .cast::<u8>()
        .wrapping_add(1)
        .cast::<QgiBuffer>();
    // SAFETY: the misaligned pointer is rejected before access; backing memory is live.
    assert_eq!(unsafe { qgi_version(ptr) }, QGI_INVALID_POINTER);
    let wrapping =
        std::ptr::without_provenance_mut(usize::MAX & !(std::mem::align_of::<QgiBuffer>() - 1));
    assert_eq!(
        // SAFETY: the overflowing range is rejected numerically before access.
        unsafe { qgi_version(wrapping) },
        QGI_INVALID_POINTER
    );
}

#[cfg(qgi_planted_read)]
#[test]
fn planted_memory_read() {
    // Same C entry point used by the sanitizer's independent C caller.
    qgi_abi_version();
}

#[test]
fn qgi_tcp_and_udp_round_trip() {
    let event = br#"{"spec":"0.1.0","family":"x-synthetic","mapping":1,"kind":"native","context":{"direction":"unknown","exchange":"unknown"},"fields":{},"native":{"format":"synthetic","data":""}}"#;
    for family in ["qgi", "qgi-udp"] {
        let id = new(family);
        let mut count = 0;
        let mut out = QgiBuffer::default();
        assert_eq!(
            // SAFETY: event and disjoint output slots are live for this call.
            unsafe {
                qgi_encode_command(
                    id,
                    event.as_ptr(),
                    event.len(),
                    null(),
                    0,
                    0,
                    &mut count,
                    &mut out,
                )
            },
            QGI_OK
        );
        assert_eq!(count, 1);
        let wire = take(out);
        let observed = if family == "qgi-udp" {
            decode(id, &wire, &json!({}))
        } else {
            let mut consumed = 0;
            assert_eq!(
                // SAFETY: live wire bytes and disjoint aligned output slots.
                unsafe {
                    qgi_decode_stream(
                        id,
                        wire.as_ptr(),
                        wire.len(),
                        0,
                        null(),
                        0,
                        &mut consumed,
                        &mut out,
                    )
                },
                QGI_OK
            );
            assert_eq!(consumed, wire.len());
            serde_json::from_slice::<Value>(&take(out)).unwrap()
        };
        assert_eq!(observed["events"][0]["family"], "x-synthetic");
        assert_eq!(observed["events"][0]["native"]["data"], "");
        assert_eq!(qgi_codec_free(id), QGI_OK);
    }
}

// Full vector coverage belongs to native tests; Miri uses the tiny cases below
// to exercise the same foreign-memory paths without interpreting codec sweeps.
#[cfg(not(miri))]
#[test]
fn vector_decode_command_and_reencode_for_each_family() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors");
    for family in ["wsjtx", "n1mm", "fldigi"] {
        let id = new(family);
        let manifest: Value = serde_json::from_slice(
            &std::fs::read(root.join(family).join("manifest.json")).unwrap(),
        )
        .unwrap();
        for class in ["positive", "command"] {
            let mut checked = 0;
            for entry in manifest["vectors"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["class"] == class)
                .take(4)
            {
                let vector: Value = serde_json::from_slice(
                    &std::fs::read(root.join(family).join(entry["file"].as_str().unwrap()))
                        .unwrap(),
                )
                .unwrap();
                let input = &vector["input"];
                if class == "positive" {
                    let bytes = evidence_bytes(input["data"].as_str().unwrap()).unwrap();
                    let mut options = json!({"context":input["context"],"structured": false});
                    if let Some(request) = input.get("request") {
                        options["native"] = json!({"request":request});
                    }
                    let result = decode(id, &bytes, &options);
                    assert_eq!(
                        result["events"][0], vector["expected"]["event"],
                        "{family}: {}",
                        entry["file"]
                    );
                    assert_eq!(result["skips"], json!([]));
                    let event = serde_json::to_vec(&result["events"][0]).unwrap();
                    let opts = serde_json::to_vec(&json!({"decode":options})).unwrap();
                    let mut out = QgiBuffer::default();
                    assert_eq!(
                        // SAFETY: event/options slices and disjoint output remain live.
                        unsafe {
                            qgi_reencode(
                                id,
                                event.as_ptr(),
                                event.len(),
                                opts.as_ptr(),
                                opts.len(),
                                &mut out,
                            )
                        },
                        QGI_OK,
                        "{family}: {}",
                        error()
                    );
                    assert_eq!(take(out), bytes);
                } else {
                    let event = serde_json::to_vec(&input["event"]).unwrap();
                    let opts =
                        serde_json::to_vec(input.get("options").unwrap_or(&json!({}))).unwrap();
                    let expected = vector["expected"]
                        .get("requests")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_else(|| vec![vector["expected"]["data"].clone()]);
                    for (index, request) in expected.iter().enumerate() {
                        let mut count = 0;
                        let mut out = QgiBuffer::default();
                        assert_eq!(
                            // SAFETY: input slices and disjoint aligned outputs remain live.
                            unsafe {
                                qgi_encode_command(
                                    id,
                                    event.as_ptr(),
                                    event.len(),
                                    opts.as_ptr(),
                                    opts.len(),
                                    index,
                                    &mut count,
                                    &mut out,
                                )
                            },
                            QGI_OK,
                            "{}: {}",
                            entry["file"],
                            error()
                        );
                        assert_eq!(count, expected.len());
                        let bytes = take(out);
                        assert_eq!(
                            bytes,
                            evidence_bytes(request.as_str().unwrap()).unwrap(),
                            "{}",
                            entry["file"]
                        );
                        let observed =
                            decode(id, &bytes, &json!({"context":input["event"]["context"]}));
                        assert!(!observed["events"].as_array().unwrap().is_empty());
                    }
                }
                checked += 1;
            }
            assert_eq!(checked, 4, "{family}/{class}");
        }
        assert_eq!(qgi_codec_free(id), QGI_OK);
    }
}

#[test]
fn minimal_decode_reencode_and_command_for_each_native_family() {
    for (family, wire, fields) in [
        (
            "wsjtx",
            // 12-byte header plus the required empty sender-ID length; no body.
            b"\xad\xbc\xcb\xda\0\0\0\x03\0\0\0\0\0\0\0\0".as_slice(),
            json!({"operation":"wsjtx.heartbeat"}),
        ),
        (
            "n1mm",
            b"<AppInfo/>".as_slice(),
            json!({"operation":"set_frequency","app":"","radio":{"number":1},"frequency_hz":1}),
        ),
        (
            "fldigi",
            b"<methodResponse><params><param><value/></param></params></methodResponse>".as_slice(),
            json!({"operation":"send_text","text":"","transmit":true}),
        ),
    ] {
        let id = new(family);
        let observed = decode(id, wire, &json!({}));
        assert_eq!(observed["events"].as_array().unwrap().len(), 1);
        assert_eq!(observed["events"][0]["family"], family);
        assert_eq!(observed["skips"], json!([]));
        let event = serde_json::to_vec(&observed["events"][0]).unwrap();
        let mut out = QgiBuffer::default();
        assert_eq!(
            // SAFETY: event and disjoint output are live; NULL/0 options are allowed.
            unsafe { qgi_reencode(id, event.as_ptr(), event.len(), null(), 0, &mut out) },
            QGI_OK,
            "{family}: {}",
            error()
        );
        assert_eq!(take(out), wire);

        let mut command = json!({"spec":"0.1.0","family":family,"mapping":1,"kind":"control",
            "context":{"direction":"to_application","exchange":"unknown"},"fields":fields});
        if family == "wsjtx" {
            command["wire_id"] = json!("");
        }
        let command = serde_json::to_vec(&command).unwrap();
        let total = if family == "fldigi" { 2 } else { 1 };
        let mut buffers = Vec::new();
        // Only fldigi needs a second request and an out-of-range probe. Keep
        // both allocations alive together, then read/free after codec teardown.
        let attempts = if family == "fldigi" { total + 1 } else { total };
        for index in 0..attempts {
            let mut count = 99;
            let status =
                // SAFETY: command and disjoint aligned output slots remain live.
                unsafe {
                    qgi_encode_command(id, command.as_ptr(), command.len(), null(), 0,
                        index, &mut count, &mut out)
                };
            if index == total {
                assert_eq!(status, QGI_INVALID_VALUE);
                assert_eq!(count, 0);
                assert_eq!(out.token, 0);
            } else {
                assert_eq!(status, QGI_OK, "{family}: {}", error());
                assert_eq!(count, total);
                buffers.push(out);
            }
        }
        assert_eq!(qgi_codec_free(id), QGI_OK);
        for (index, buffer) in buffers.into_iter().enumerate() {
            let bytes = take(buffer);
            if family == "fldigi" {
                let method = if index == 0 { "text.add_tx" } else { "main.tx" };
                assert!(std::str::from_utf8(&bytes).unwrap().contains(method));
            } else if family == "wsjtx" {
                assert_eq!(bytes, wire);
            } else {
                assert!(
                    std::str::from_utf8(&bytes)
                        .unwrap()
                        .contains("<radio_setfrequency>")
                );
            }
        }
    }
}

#[test]
fn caller_buffer_stream_partial_multiple_eof_and_recovery() {
    let id = new("qgi");
    let event = br#"{"spec":"0.1.0","family":"x-synthetic","mapping":1,"kind":"native","context":{"direction":"unknown","exchange":"unknown"},"fields":{},"native":{"format":"synthetic","data":""}}"#;
    let mut bytes = event.to_vec();
    let mut out = QgiBuffer::default();
    let mut used = 99;
    // SAFETY: live bytes and disjoint aligned outputs for each call.
    unsafe {
        assert_eq!(
            qgi_decode_stream(
                id,
                bytes.as_ptr(),
                bytes.len(),
                0,
                null(),
                0,
                &mut used,
                &mut out
            ),
            QGI_OK
        );
        assert_eq!(used, 0);
        assert_eq!(out.token, 0);
        bytes.extend_from_slice(b"\r\n");
        let first = bytes.len();
        bytes.extend_from_slice(b"bad\n");
        assert_eq!(
            qgi_decode_stream(
                id,
                bytes.as_ptr(),
                bytes.len(),
                0,
                null(),
                0,
                &mut used,
                &mut out
            ),
            QGI_OK
        );
        assert_eq!(used, first);
        assert_eq!(
            serde_json::from_slice::<Value>(&take(out)).unwrap()["events"][0]["via"][0]["family"],
            "qgi"
        );
        bytes.drain(..used);
        assert_eq!(
            qgi_decode_stream(
                id,
                bytes.as_ptr(),
                bytes.len(),
                0,
                null(),
                0,
                &mut used,
                &mut out
            ),
            QGI_MALFORMED
        );
        assert_eq!(used, bytes.len());
        assert_eq!(out.token, 0);
        assert_eq!(
            qgi_decode_stream(
                id,
                event.as_ptr(),
                event.len(),
                1,
                null(),
                0,
                &mut used,
                &mut out
            ),
            QGI_TRUNCATED
        );
        assert_eq!(used, event.len());
        // The same codec-error/consumption path as a huge frame, using only 68
        // bytes: one container beyond the JSON depth limit, plus its delimiter.
        let deep = format!("{}0{}\n", "[".repeat(32 + 1), "]".repeat(32 + 1));
        assert_eq!(
            qgi_decode_stream(
                id,
                deep.as_ptr(),
                deep.len(),
                0,
                null(),
                0,
                &mut used,
                &mut out
            ),
            QGI_OVERSIZED
        );
        assert_eq!(used, deep.len());
        assert_eq!(out.token, 0);
        assert_eq!(qgi_reset(id, null(), 0), QGI_OK);
    }
    assert_eq!(qgi_codec_free(id), QGI_OK);
}

// Scanning a MiB is safe codec work, not a foreign-memory boundary check. Keep
// the real stream size/discard recovery regression in the ordinary suite.
#[cfg(not(miri))]
#[test]
fn caller_buffer_stream_large_frame_and_recovery() {
    let id = new("qgi");
    let mut out = QgiBuffer::default();
    let mut used = 0;
    let large = vec![b'x'; QGI_MAX_BUFFER];
    // SAFETY: live input and disjoint aligned output slots for each call.
    unsafe {
        assert_eq!(
            qgi_decode_stream(
                id,
                large.as_ptr(),
                large.len(),
                0,
                null(),
                0,
                &mut used,
                &mut out
            ),
            QGI_OVERSIZED
        );
        assert_eq!(used, large.len());
        assert_eq!(
            qgi_decode_stream(id, b"tail\n".as_ptr(), 5, 0, null(), 0, &mut used, &mut out),
            QGI_OK
        );
        assert_eq!(used, 5);
        assert_eq!(out.token, 0);
        assert_eq!(qgi_reset(id, null(), 0), QGI_OK);
    }
    assert_eq!(qgi_codec_free(id), QGI_OK);
}

#[test]
fn thread_local_errors_shared_handles_and_buffer_lifetime() {
    let id = new("wsjtx");
    assert_eq!(qgi_codec_free(0), QGI_BAD_HANDLE);
    let parent_error = error();
    let threads: Vec<_> = (0..4)
        .map(|_| {
            std::thread::spawn(move || {
                assert!(error().is_empty());
                let mut out = QgiBuffer::default();
                assert_eq!(
                    // SAFETY: live output; empty input is admitted and reports truncation.
                    unsafe { qgi_decode(id, null(), 0, null(), 0, &mut out) },
                    QGI_TRUNCATED
                );
                assert!(!error().is_empty());
                // SAFETY: live aligned disjoint output.
                assert_eq!(unsafe { qgi_version(&mut out) }, QGI_OK);
                TransferredBuffer(out)
            })
        })
        .collect();
    assert_eq!(error(), parent_error);
    for thread in threads {
        let buffer = thread.join().unwrap();
        assert_eq!(take(buffer.0), env!("CARGO_PKG_VERSION").as_bytes());
    }
    assert_eq!(qgi_codec_free(id), QGI_OK);
}

// Preserve pointer provenance while exercising the documented cross-thread free.
struct TransferredBuffer(QgiBuffer);
// SAFETY: the ABI buffer is immutable and uniquely owned by this test wrapper;
// the spawning thread stops reading before transferring it through join.
unsafe impl Send for TransferredBuffer {}

struct PanicCodec(Option<Arc<std::sync::atomic::AtomicUsize>>);

struct PanicPayload(Arc<std::sync::atomic::AtomicUsize>);
impl Drop for PanicPayload {
    fn drop(&mut self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        panic!("planted payload destructor panic");
    }
}
impl qso_graph_interop::codec::Profile for PanicCodec {
    fn native_version_supported(&self, _: &qso_graph_interop::Event) -> bool {
        true
    }
    fn validate_authored(
        &self,
        _: &qso_graph_interop::Event,
        _: &qso_graph_interop::VerifyOptions,
    ) -> Result<(), qso_graph_interop::Error> {
        panic!("planted authored panic")
    }
    fn recognize(
        &self,
        _: &qso_graph_interop::Event,
        _: &qso_graph_interop::VerifyOptions,
    ) -> Result<qso_graph_interop::model::ProfileView, qso_graph_interop::Error> {
        panic!("planted recognition panic")
    }

    fn family(&self) -> &str {
        "wsjtx"
    }
    fn mapping(&self) -> u32 {
        1
    }
    fn verify(
        &self,
        _: &qso_graph_interop::Event,
        _: &qso_graph_interop::VerifyOptions,
    ) -> Result<qso_graph_interop::model::Verification, qso_graph_interop::Error> {
        panic!("planted verify panic")
    }
}
impl Codec for PanicCodec {
    fn framing(&self) -> qso_graph_interop::codec::Framing {
        qso_graph_interop::codec::Framing::Datagram
    }
    fn limits(&self) -> qso_graph_interop::codec::Limits {
        qso_graph_interop::codec::Limits::stateless(10, 10)
    }
    fn decode(
        &mut self,
        _: &[u8],
        _: &DecodeOptions<'_>,
    ) -> Result<Outcome, qso_graph_interop::Error> {
        if let Some(count) = &self.0 {
            std::panic::panic_any(PanicPayload(Arc::clone(count)));
        }
        panic!("planted decode panic")
    }
    fn encode_commands(
        &self,
        _: &qso_graph_interop::Event,
        _: &EncodeOptions,
    ) -> Result<qso_graph_interop::codec::EncodedCommands, qso_graph_interop::Error> {
        panic!("planted encode panic")
    }
    fn reencode(
        &self,
        _: &qso_graph_interop::Event,
        _: &EncodeOptions,
        _: &qso_graph_interop::VerifyOptions,
    ) -> Result<Vec<u8>, qso_graph_interop::Error> {
        panic!("planted reencode panic")
    }
    fn reset(&mut self, _: Option<&[u8]>) {
        panic!("planted reset panic")
    }
}
#[test]
fn panic_is_contained_and_handle_invalidated() {
    let healthy = new("wsjtx");
    let id = new("wsjtx");
    state().codecs.insert(id, Box::new(PanicCodec(None)));
    let mut out = QgiBuffer::default();
    assert_eq!(
        // SAFETY: NULL/0 input is allowed and out is live aligned writable storage.
        unsafe { qgi_decode(id, null(), 0, null(), 0, &mut out) },
        QGI_PANIC
    );
    assert_eq!(out.token, 0);
    assert!(error().contains("panic contained"));
    assert_eq!(qgi_codec_free(id), QGI_BAD_HANDLE);
    // A panic while holding the registry lock does not invalidate other handles.
    assert_eq!(
        // SAFETY: NULL/0 session is allowed; healthy is a live handle.
        unsafe { qgi_reset(healthy, null(), 0) },
        QGI_OK
    );
    assert_eq!(qgi_codec_free(healthy), QGI_OK);
}

#[test]
fn panic_payload_is_dropped_and_its_destructor_panic_contained() {
    let dropped = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let id = new("wsjtx");
    state()
        .codecs
        .insert(id, Box::new(PanicCodec(Some(Arc::clone(&dropped)))));
    let mut out = QgiBuffer::default();
    assert_eq!(
        // SAFETY: empty inputs are allowed; out is live aligned storage.
        unsafe { qgi_decode(id, null(), 0, null(), 0, &mut out) },
        QGI_PANIC
    );
    assert_eq!(dropped.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(out.token, 0);
    assert!(error().contains("panic contained"));
    assert_eq!(qgi_codec_free(id), QGI_BAD_HANDLE);
}

#[test]
fn stable_categories_and_altered_buffer_descriptor() {
    for (category, status) in [
        (Category::Truncated, 1),
        (Category::Oversized, 2),
        (Category::Malformed, 3),
        (Category::UnsupportedVersion, 4),
        (Category::InvalidValue, 5),
    ] {
        assert_eq!(
            Failure::from(qso_graph_interop::Error::new(category, "synthetic")).0,
            status
        );
    }
    let mut out = QgiBuffer::default();
    assert_eq!(
        // SAFETY: out is live aligned writable storage.
        unsafe { qgi_version(&mut out) },
        QGI_OK
    );
    let mut altered = out;
    altered.len += 1;
    assert_eq!(qgi_buffer_free(altered), QGI_BAD_BUFFER);
    altered = out;
    altered.data = null();
    assert_eq!(qgi_buffer_free(altered), QGI_BAD_BUFFER);
    assert_eq!(take(out), env!("CARGO_PKG_VERSION").as_bytes());
}

#[cfg(not(miri))]
fn evidence_bytes(data: &str) -> Result<Vec<u8>, qso_graph_interop::Error> {
    qso_graph_interop::model::NativeEvidence::from_value(
        serde_json::json!({"format":"test","data":data}),
    )?
    .data()
}

#[test]
fn explicit_carriage_uses_original_family_and_per_call_association() {
    use qso_graph_interop::{
        Profile,
        fldigi::{Fldigi, RequestAssociation},
    };
    let body = b"<methodResponse><params><param><value><double>137500.0</double></value></param></params></methodResponse>";
    let options = DecodeOptions::default().with_metadata(
        RequestAssociation::for_method("rig.set_frequency")
            .unwrap()
            .into(),
    );
    let mut codec = Fldigi::default();
    let event = codec.decode(body, &options).unwrap().events.remove(0);
    assert_eq!(
        codec.verify(&event, &options.verification()).unwrap(),
        qso_graph_interop::Verification::Verified
    );
    let bytes = serde_json::to_vec(&event).unwrap();
    let association = br#"{"decode":{"native":{"request":{"method":"rig.set_frequency"}}}}"#;
    let id = new("qgi");
    let mut out = QgiBuffer::default();
    assert_eq!(
        // SAFETY: all inputs are live byte slices; out is disjoint aligned writable storage.
        unsafe {
            qgi_encode_event(
                id,
                bytes.as_ptr(),
                bytes.len(),
                association.as_ptr(),
                association.len(),
                &mut out,
            )
        },
        QGI_OK
    );
    let framed = take(out);
    assert_eq!(framed.last(), Some(&b'\n'));
    assert_eq!(
        qso_graph_interop::parse(&framed[..framed.len() - 1], MAX_RECORD).unwrap(),
        event
    );
    let mut wrong = event.into_value();
    wrong["fields"]["frequency_hz"] = 137501.into();
    let bytes = serde_json::to_vec(&wrong).unwrap();
    assert_eq!(
        // SAFETY: live input slices and aligned disjoint writable output.
        unsafe {
            qgi_encode_event(
                id,
                bytes.as_ptr(),
                bytes.len(),
                association.as_ptr(),
                association.len(),
                &mut out,
            )
        },
        QGI_INVALID_VALUE
    );
    assert_eq!(out.token, 0);
    assert_eq!(qgi_codec_free(id), QGI_OK);
}
