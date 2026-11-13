// SPDX-License-Identifier: GPL-3.0-or-later
//! C ABI. Only this crate may use unsafe Rust; codecs remain sans-I/O.
#![deny(unsafe_op_in_unsafe_fn)]

use qso_graph_interop::{
    Category, Codec,
    codec::{DecodeOptions, EncodeOptions, Outcome},
    json,
    model::{MAX_CONTEXT, MAX_RECORD},
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex, MutexGuard, OnceLock},
};

/// ABI 1. Call qgi_abi_version before any other operation; require equality.
pub const QGI_ABI_VERSION: u32 = 1;
/// Success; an outcome may contain skips or no complete stream frame.
pub const QGI_OK: i32 = 0;
/// Core category: incomplete native input or unfinished EOF frame.
pub const QGI_TRUNCATED: i32 = 1;
/// Core category: message, field, parser, or context resource limit.
pub const QGI_OVERSIZED: i32 = 2;
/// Core category: malformed native or JSON syntax.
pub const QGI_MALFORMED: i32 = 3;
/// Core category: unsupported command or family version.
pub const QGI_UNSUPPORTED_VERSION: i32 = 4;
/// Core category: invalid value, UTF-8 text, option, family, or request index.
pub const QGI_INVALID_VALUE: i32 = 5;
/// Required pointer is NULL (nonempty input or required output).
pub const QGI_NULL_POINTER: i32 = 100;
/// Unknown, zero, freed, or panic-invalidated handle.
pub const QGI_BAD_HANDLE: i32 = 101;
/// ABI input, live-resource, or token-space limit exceeded.
pub const QGI_BUFFER_TOO_LARGE: i32 = 102;
/// Unknown, freed, or modified owned-buffer descriptor.
pub const QGI_BAD_BUFFER: i32 = 103;
/// Rust unwind contained; affected handle is invalidated. Abort/OOM is unrecoverable.
pub const QGI_PANIC: i32 = 104;
/// Misaligned output or overflowing input/output address range.
pub const QGI_INVALID_POINTER: i32 = 105;
/// Maximum bytes in any single ABI input, including caller stream buffers.
pub const QGI_MAX_BUFFER: usize = 1_048_578;
const MAX_RESOURCES: usize = 4096;

/// Opaque nonzero codec token. Never dereference; tokens are not reused.
pub type QgiCodec = u64;

/// Library-owned immutable bytes, without a NUL terminator. Never change these fields.
/// Copying this descriptor does not copy ownership. Release exactly once using
/// qgi_buffer_free, on any thread, after all readers finish. {NULL,0,0} is empty.
/// The allocation survives codec reset/free. Never use free/delete on data.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct QgiBuffer {
    pub data: *const u8,
    pub len: usize,
    pub token: u64,
}
impl Default for QgiBuffer {
    fn default() -> Self {
        Self {
            data: std::ptr::null(),
            len: 0,
            token: 0,
        }
    }
}

/// Borrowed UTF-8 error text, without a NUL terminator. Valid on this thread until
/// its next ABI call other than qgi_last_error or qgi_abi_version, or thread exit.
/// Copy before then. Never free it or read it concurrently with another call on
/// its originating thread. No error is {NULL,0}.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QgiError {
    pub data: *const u8,
    pub len: usize,
}

struct State {
    next: u64,
    codecs: BTreeMap<u64, Box<dyn Codec + Send>>,
    // Moving a Box after exporting its pointer retags its unique allocation and
    // invalidates foreign readers. Arc keeps shared provenance across map moves.
    buffers: BTreeMap<u64, Arc<[u8]>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            next: 1,
            codecs: BTreeMap::new(),
            buffers: BTreeMap::new(),
        }
    }
}
impl State {
    fn token(&mut self) -> Result<u64, Failure> {
        let id = self.next;
        self.next = id
            .checked_add(1)
            .ok_or(fail(QGI_BUFFER_TOO_LARGE, "token space exhausted"))?;
        Ok(id)
    }
    fn codec(&mut self, id: QgiCodec) -> Result<&mut Box<dyn Codec + Send>, Failure> {
        self.codecs
            .get_mut(&id)
            .ok_or(fail(QGI_BAD_HANDLE, "unknown or freed codec"))
    }
    fn buffer(&mut self, bytes: Vec<u8>) -> Result<QgiBuffer, Failure> {
        if self.buffers.len() >= MAX_RESOURCES {
            return Err(fail(QGI_BUFFER_TOO_LARGE, "too many outstanding buffers"));
        }
        let token = self.token()?;
        let bytes: Arc<[u8]> = bytes.into();
        let result = QgiBuffer {
            data: bytes.as_ptr(),
            len: bytes.len(),
            token,
        };
        self.buffers.insert(token, bytes);
        Ok(result)
    }
}
fn state() -> MutexGuard<'static, State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    // A panic can leave a codec partially changed. boundary removes that codec on panic.
    STATE
        .get_or_init(|| Mutex::new(State::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
thread_local! {
    static LAST_ERROR: RefCell<String> = const { RefCell::new(String::new()) };
}
struct Failure(i32, String);
fn fail(status: i32, message: &str) -> Failure {
    Failure(status, message.into())
}
impl From<qso_graph_interop::Error> for Failure {
    fn from(e: qso_graph_interop::Error) -> Self {
        let status = match e.category {
            Category::Truncated => QGI_TRUNCATED,
            Category::Oversized => QGI_OVERSIZED,
            Category::Malformed => QGI_MALFORMED,
            Category::UnsupportedVersion => QGI_UNSUPPORTED_VERSION,
            Category::InvalidValue => QGI_INVALID_VALUE,
            _ => QGI_INVALID_VALUE,
        };
        Self(status, e.to_string())
    }
}
fn message(text: String) {
    let _ = LAST_ERROR.try_with(|error| {
        if let Ok(mut error) = error.try_borrow_mut() {
            *error = text;
        }
    });
}
fn boundary(handle: Option<QgiCodec>, f: impl FnOnce() -> Result<(), Failure>) -> i32 {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => {
            message(String::new());
            QGI_OK
        }
        Ok(Err(Failure(status, text))) => {
            message(text);
            status
        }
        Err(payload) => {
            // Release ordinary payloads instead of leaking on every contained panic.
            // A custom payload's destructor can panic; contain that unwind too.
            if let Err(nested) = catch_unwind(AssertUnwindSafe(|| drop(payload))) {
                if nested.is::<String>() || nested.is::<&'static str>() {
                    // These standard payloads have nonpanicking destructors.
                    drop(nested);
                } else {
                    // An adversarial destructor can panic with another adversarial
                    // destructor. Do not allow unbounded unwinding across the C ABI.
                    std::mem::forget(nested);
                }
            }
            if let Some(id) = handle {
                state().codecs.remove(&id);
            }
            message("Rust panic contained; codec invalidated".into());
            QGI_PANIC
        }
    }
}
fn check_out<T>(ptr: *mut T) -> Result<(), Failure> {
    if ptr.is_null() {
        return Err(fail(QGI_NULL_POINTER, "required output pointer is null"));
    }
    if (ptr as usize)
        .checked_add(std::mem::size_of::<T>())
        .is_none()
    {
        return Err(fail(QGI_INVALID_POINTER, "output address range wraps"));
    }
    if !(ptr as usize).is_multiple_of(std::mem::align_of::<T>()) {
        return Err(fail(QGI_INVALID_POINTER, "misaligned output pointer"));
    }
    Ok(())
}
// All pointer access is confined to these two helpers. Checked lengths do not prove
// foreign memory is mapped: the caller must supply live allocations (see header).
unsafe fn input<'a>(ptr: *const u8, len: usize, limit: usize) -> Result<&'a [u8], Failure> {
    if len > limit || len > isize::MAX as usize {
        return Err(fail(QGI_BUFFER_TOO_LARGE, "ABI input exceeds byte limit"));
    }
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(fail(QGI_NULL_POINTER, "nonempty input pointer is null"));
    }
    if (ptr as usize).checked_add(len).is_none() {
        return Err(fail(QGI_INVALID_POINTER, "input address range wraps"));
    }
    // SAFETY: null, byte length and address overflow checked; caller promises a live,
    // readable allocation for len bytes for this call. u8 has alignment 1.
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}
unsafe fn put<T>(ptr: *mut T, value: T) {
    // SAFETY: every call first checks null/alignment; caller provides writable storage
    // for one T, disjoint from inputs and other outputs, for the duration of the call.
    unsafe { ptr.write(value) };
}
fn object(bytes: &[u8]) -> Result<Value, Failure> {
    if bytes.is_empty() {
        return Ok(json!({}));
    }
    std::str::from_utf8(bytes).map_err(|_| fail(QGI_INVALID_VALUE, "text must be UTF-8"))?;
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| fail(QGI_MALFORMED, "invalid options JSON"))?;
    if !value.is_object() {
        return Err(fail(QGI_INVALID_VALUE, "options must be a JSON object"));
    }
    json::bounds(&value, MAX_RECORD)?;
    Ok(value)
}
fn decode_options(value: &Value) -> Result<DecodeOptions<'static>, Failure> {
    let mut options = DecodeOptions::default();
    let map = value
        .as_object()
        .ok_or(fail(QGI_INVALID_VALUE, "options must be object"))?;
    if map
        .keys()
        .any(|k| !["context", "native", "session", "expiry", "structured"].contains(&k.as_str()))
    {
        return Err(fail(QGI_INVALID_VALUE, "unknown decode option"));
    }
    if let Some(context) = value.get("context") {
        if serde_json::to_vec(context)
            .map_err(|_| fail(QGI_INVALID_VALUE, "context JSON"))?
            .len()
            > MAX_CONTEXT
        {
            return Err(fail(QGI_OVERSIZED, "context exceeds 4096 bytes"));
        }
        // Validate context using the public event validator, preserving unknown additions.
        let test = json!({"spec":"0.1.0","family":"qgi","mapping":1,"kind":"native","context":context,"fields":{},"native":{"format":"datagram","data":""}});
        json::validate(&test, true)?;
        options.context = context.clone();
    }
    if let Some(native) = value.get("native") {
        options.native = native
            .as_object()
            .ok_or(fail(QGI_INVALID_VALUE, "native options must be object"))?
            .clone();
    }
    // Session bytes are supplied separately by reset today; official codecs do not
    // retain session references. Reject this reserved option rather than guessing.
    if value.get("session").is_some() {
        return Err(fail(QGI_INVALID_VALUE, "session option is reserved"));
    }
    if let Some(expiry) = value.get("expiry") {
        options.expiry = Some(
            expiry
                .as_u64()
                .ok_or(fail(QGI_INVALID_VALUE, "expiry must be u64"))?,
        );
    }
    if let Some(structured) = value.get("structured") {
        options.structured = structured
            .as_bool()
            .ok_or(fail(QGI_INVALID_VALUE, "structured must be boolean"))?;
    }
    Ok(options)
}
fn encode_options(value: &Value) -> Result<EncodeOptions, Failure> {
    let mut options = EncodeOptions::default();
    options.native = value
        .as_object()
        .ok_or(fail(QGI_INVALID_VALUE, "encoding options must be object"))?
        .clone();
    Ok(options)
}
fn outcome(outcome: Outcome) -> Result<Vec<u8>, Failure> {
    let mut events = Vec::new();
    for event in outcome.events {
        // Use the specification writer before embedding an event in the ABI outcome.
        events.push(
            serde_json::from_slice::<Value>(&json::write(&event, false, None)?)
                .map_err(|_| fail(QGI_MALFORMED, "event serialization"))?,
        );
    }
    let skips: Vec<_> = outcome
        .skips
        .into_iter()
        .map(|s| {
            json!({
                "reason":s.reason, "native_version":s.native_version, "native_type":s.native_type,
                "native_name":s.native_name, "set_index":s.set_index, "record_index":s.record_index
            })
        })
        .collect();
    serde_json::to_vec(&json!({"events":events,"skips":skips}))
        .map_err(|_| fail(QGI_INVALID_VALUE, "outcome serialization"))
}

/// Returns 1 for this ABI, or 0 on a contained panic. Does not change last-error.
#[unsafe(no_mangle)]
pub extern "C" fn qgi_abi_version() -> u32 {
    #[cfg(qgi_planted_read)]
    planted::read();
    catch_unwind(|| QGI_ABI_VERSION).unwrap_or(0)
}
/// Returns borrowed thread-local error text; this call does not change it.
#[unsafe(no_mangle)]
pub extern "C" fn qgi_last_error() -> QgiError {
    catch_unwind(|| {
        LAST_ERROR.with(|e| {
            let error = e.borrow();
            if error.is_empty() {
                QgiError {
                    data: std::ptr::null(),
                    len: 0,
                }
            } else {
                QgiError {
                    data: error.as_ptr(),
                    len: error.len(),
                }
            }
        })
    })
    .unwrap_or(QgiError {
        data: std::ptr::null(),
        len: 0,
    })
}

/// Returns the UTF-8 package version in a library-owned buffer.
///
/// # Safety
/// All API pointers must refer to live readable/writable allocations for their
/// declared sizes, for the call. Outputs must be aligned and disjoint from inputs
/// and other outputs. Nonempty inputs cannot be NULL. NULL with zero length is
/// accepted. Numeric checks cannot detect dangling or unmapped foreign pointers.
/// Input bytes are borrowed only during the call. Output slots are overwritten;
/// release prior buffers before reusing slots. After all output pointer checks pass,
/// ordinary failures zero outputs (stream errors can still report consumed bytes).
/// All calls contain unwinding panics (status QGI_PANIC); abort/OOM is not recoverable.
/// Handles may be used/freed on any thread: operations are serialized internally.
/// A concurrent free may cause QGI_BAD_HANDLE. No callbacks; no reentrancy into a
/// running call. Distinct calls on one thread invalidate its last-error view.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgi_version(out: *mut QgiBuffer) -> i32 {
    boundary(None, || {
        check_out(out)?;
        // SAFETY: output checked; foreign storage follows the API pointer contract.
        unsafe { put(out, QgiBuffer::default()) };
        let buffer = state().buffer(env!("CARGO_PKG_VERSION").as_bytes().to_vec())?;
        // SAFETY: output checked above and remains live for this call.
        unsafe { put(out, buffer) };
        Ok(())
    })
}
/// Creates a codec for UTF-8 bytes "wsjtx", "n1mm", "fldigi", "qgi" (TCP),
/// or "qgi-udp". Name has explicit byte length, without NUL. Zero is invalid.
/// # Safety
/// Follow the pointer/ownership contract on qgi_version. out is required.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgi_codec_new(
    family: *const u8,
    family_len: usize,
    out: *mut QgiCodec,
) -> i32 {
    boundary(None, || {
        check_out(out)?;
        // SAFETY: output checked and caller provides live disjoint storage.
        unsafe { put(out, 0) };
        // SAFETY: input helper checks pointer/length; caller provides live input.
        let name = unsafe { input(family, family_len, 32)? };
        let name = std::str::from_utf8(name)
            .map_err(|_| fail(QGI_INVALID_VALUE, "family must be UTF-8"))?;
        let codec: Box<dyn Codec + Send> = match name {
            "wsjtx" => Box::new(qso_graph_interop::wsjtx::Wsjtx::default()),
            "n1mm" => Box::new(qso_graph_interop::n1mm::N1mm::default()),
            "fldigi" => Box::new(qso_graph_interop::fldigi::Fldigi::default()),
            "qgi" => Box::new(qso_graph_interop::qgi::Qgi::new(
                qso_graph_interop::qgi::Transport::Tcp,
            )),
            "qgi-udp" => Box::new(qso_graph_interop::qgi::Qgi::new(
                qso_graph_interop::qgi::Transport::Udp,
            )),
            _ => return Err(fail(QGI_INVALID_VALUE, "unknown codec family")),
        };
        let mut state = state();
        if state.codecs.len() >= MAX_RESOURCES {
            return Err(fail(QGI_BUFFER_TOO_LARGE, "too many codecs"));
        }
        let id = state.token()?;
        state.codecs.insert(id, codec);
        // SAFETY: output checked above and remains live for this call.
        unsafe { put(out, id) };
        Ok(())
    })
}
/// Releases a codec. Zero, invented, or already-freed tokens return BAD_HANDLE.
/// Buffers previously returned by this codec remain valid.
#[unsafe(no_mangle)]
pub extern "C" fn qgi_codec_free(handle: QgiCodec) -> i32 {
    boundary(Some(handle), || {
        state()
            .codecs
            .remove(&handle)
            .map(|_| ())
            .ok_or(fail(QGI_BAD_HANDLE, "unknown or freed codec"))
    })
}
/// Releases library-owned bytes. Empty {NULL,0,0} succeeds; mismatched or stale
/// descriptors return BAD_BUFFER. All copies become invalid after a successful free.
#[unsafe(no_mangle)]
pub extern "C" fn qgi_buffer_free(buffer: QgiBuffer) -> i32 {
    boundary(None, || {
        if buffer.token == 0 && buffer.data.is_null() && buffer.len == 0 {
            return Ok(());
        }
        let mut state = state();
        let bytes = state
            .buffers
            .get(&buffer.token)
            .ok_or(fail(QGI_BAD_BUFFER, "unknown or freed buffer"))?;
        if bytes.as_ptr() != buffer.data || bytes.len() != buffer.len {
            return Err(fail(
                QGI_BAD_BUFFER,
                "buffer descriptor does not match allocation",
            ));
        }
        state.buffers.remove(&buffer.token);
        Ok(())
    })
}
/// Decodes native bytes into UTF-8 JSON {"events":[event,...],"skips":[...]}.
/// options is a UTF-8 object with optional context (specification context object),
/// native (family metadata; fldigi native.request), structured (bool), expiry (u64).
/// Empty options use core defaults. No key loses null/empty/absent/raw distinctions.
/// # Safety
/// Follow qgi_version's pointer contract. out is required and zeroed on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgi_decode(
    handle: QgiCodec,
    data: *const u8,
    len: usize,
    options: *const u8,
    options_len: usize,
    out: *mut QgiBuffer,
) -> i32 {
    boundary(Some(handle), || {
        check_out(out)?;
        // SAFETY: checked output; caller owns live disjoint storage.
        unsafe { put(out, QgiBuffer::default()) };
        // SAFETY: helper validates lengths/null; caller owns live input allocations.
        let (bytes, options) = unsafe {
            (
                input(data, len, QGI_MAX_BUFFER)?,
                input(options, options_len, MAX_RECORD)?,
            )
        };
        let options = decode_options(&object(options)?)?;
        let mut state = state();
        let result = state.codec(handle)?.decode(bytes, &options)?;
        let buffer = state.buffer(outcome(result)?)?;
        // SAFETY: checked output remains live throughout call.
        unsafe { put(out, buffer) };
        Ok(())
    })
}
/// Returns one native request by zero-based index and the total request count.
/// fldigi may need multiple ordered independent requests: call for indexes 0..count.
/// Each returned buffer has separate ownership. options is the family-native
/// encoding options object (e.g. {"schema":3} for WSJT-X); empty means defaults.
/// event is a specification JSON event object. Out-of-range index is INVALID_VALUE.
/// # Safety
/// Follow qgi_version's pointer contract. count and out are required, disjoint outputs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgi_encode_command(
    handle: QgiCodec,
    event: *const u8,
    event_len: usize,
    options: *const u8,
    options_len: usize,
    index: usize,
    count: *mut usize,
    out: *mut QgiBuffer,
) -> i32 {
    boundary(Some(handle), || {
        check_out(count)?;
        check_out(out)?;
        // SAFETY: both outputs checked; caller guarantees disjoint live storage.
        unsafe {
            put(count, 0);
            put(out, QgiBuffer::default());
        }
        // SAFETY: helper checks both inputs; caller guarantees live allocations.
        let (event, options) = unsafe {
            (
                input(event, event_len, MAX_RECORD)?,
                input(options, options_len, MAX_RECORD)?,
            )
        };
        let event = json::parse(event, MAX_RECORD)?;
        let options = encode_options(&object(options)?)?;
        let mut state = state();
        let mut requests = state.codec(handle)?.encode_commands(&event, &options)?;
        let total = requests.len();
        if index >= total {
            return Err(fail(QGI_INVALID_VALUE, "request index out of range"));
        }
        let buffer = state.buffer(requests.remove(index))?;
        // SAFETY: validated, disjoint writable outputs.
        unsafe {
            put(count, total);
            put(out, buffer);
        }
        Ok(())
    })
}
/// Re-encodes an observed event into native bytes. options has optional "encode"
/// (family-native encoding options) and "decode" (qgi_decode options, including
/// fldigi response association). Empty means defaults. Exact native evidence and
/// extensions are preserved by the family codec.
/// # Safety
/// Follow qgi_version's pointer contract; out is required.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgi_reencode(
    handle: QgiCodec,
    event: *const u8,
    event_len: usize,
    options: *const u8,
    options_len: usize,
    out: *mut QgiBuffer,
) -> i32 {
    boundary(Some(handle), || {
        check_out(out)?;
        // SAFETY: checked live output following caller contract.
        unsafe { put(out, QgiBuffer::default()) };
        // SAFETY: helper checks input lengths/null; caller provides live storage.
        let (event, options) = unsafe {
            (
                input(event, event_len, MAX_RECORD)?,
                input(options, options_len, MAX_RECORD)?,
            )
        };
        let event = json::parse(event, MAX_RECORD)?;
        let value = object(options)?;
        if value
            .as_object()
            .is_some_and(|m| m.keys().any(|k| k != "encode" && k != "decode"))
        {
            return Err(fail(QGI_INVALID_VALUE, "unknown reencode option"));
        }
        let empty = json!({});
        let encode = encode_options(value.get("encode").unwrap_or(&empty))?;
        let decode = decode_options(value.get("decode").unwrap_or(&empty))?;
        let mut state = state();
        let bytes = state
            .codec(handle)?
            .reencode_with(&event, &encode, &decode)?;
        let buffer = state.buffer(bytes)?;
        // SAFETY: checked live output following caller contract.
        unsafe { put(out, buffer) };
        Ok(())
    })
}
/// Resets all codec state if session is NULL/0, or the specified raw session bytes
/// otherwise (a non-NULL pointer with length 0 selects an empty session).
/// # Safety
/// Follow qgi_version's pointer contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgi_reset(
    handle: QgiCodec,
    session: *const u8,
    session_len: usize,
) -> i32 {
    boundary(Some(handle), || {
        // SAFETY: helper validates length/null; caller guarantees readable memory.
        let bytes = unsafe { input(session, session_len, QGI_MAX_BUFFER)? };
        state()
            .codec(handle)?
            .reset(if session.is_null() { None } else { Some(bytes) });
        Ok(())
    })
}
/// Decodes at most one frame from the caller's QGI TCP buffer. Copies input only
/// during this call; never retains it. consumed reports bytes to remove, even on a
/// native framing error (documented recovery boundary). Zero consumed means append
/// more input and retry. out is empty when no complete event is available.
/// eof must be 0 or 1; 1 diagnoses unfinished trailing frames. options matches decode.
/// Keep unconsumed bytes and append new data yourself. MAX_BUFFER bounds each call;
/// larger streams must be chunked. Other codecs return INVALID_VALUE.
/// # Safety
/// Follow qgi_version's pointer contract. consumed and out are required and disjoint.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qgi_decode_stream(
    handle: QgiCodec,
    data: *const u8,
    len: usize,
    eof: u8,
    options: *const u8,
    options_len: usize,
    consumed: *mut usize,
    out: *mut QgiBuffer,
) -> i32 {
    boundary(Some(handle), || {
        check_out(consumed)?;
        check_out(out)?;
        // SAFETY: outputs checked; caller guarantees disjoint live allocations.
        unsafe {
            put(consumed, 0);
            put(out, QgiBuffer::default());
        }
        // SAFETY: helper validates both inputs; caller guarantees readable allocations.
        let (bytes, options) = unsafe {
            (
                input(data, len, QGI_MAX_BUFFER)?,
                input(options, options_len, MAX_RECORD)?,
            )
        };
        if eof > 1 {
            return Err(fail(QGI_INVALID_VALUE, "eof must be 0 or 1"));
        }
        let options = decode_options(&object(options)?)?;
        let mut bytes = bytes.to_vec();
        let mut state = state();
        let result = state
            .codec(handle)?
            .decode_stream(&mut bytes, eof == 1, &options);
        // SAFETY: output checked; consumption must be returned even on framing error.
        unsafe { put(consumed, len - bytes.len()) };
        if let Some(result) = result? {
            let buffer = state.buffer(outcome(result)?)?;
            // SAFETY: checked output remains live for call.
            unsafe { put(out, buffer) };
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests;

// Compiled only by the verifier's isolated negative-control builds. Never enabled
// by a Cargo feature, including --all-features, or in ordinary library artifacts.
#[cfg(qgi_planted_read)]
mod planted {
    #[inline(never)]
    pub(super) fn read() {
        let bytes = Box::new([7_u8]);
        let past = bytes.as_ptr().wrapping_add(1);
        // SAFETY: intentionally violated ONLY in the planted tool check: a one-byte
        // heap overread. Volatile keeps the read observable to both memory tools.
        std::hint::black_box(unsafe { past.read_volatile() });
    }
}
