// SPDX-License-Identifier: GPL-3.0-or-later
//! Partial frames, multiple frames, EOF and recovery through caller-owned buffers.
#![forbid(unsafe_code)]
#[cfg(feature = "qgi")]
mod tests {
    use qso_graph_interop::{
        Category, Codec, Event,
        codec::{DecodeOptions, EncodeOptions},
        json,
        qgi::{Qgi, Transport},
    };
    use serde_json::json;
    fn event() -> Event {
        Event::from_value(json!({"spec":"0.1.0","family":"qgi","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"fields":{"operation":"replay"}})).unwrap()
    }
    #[test]
    fn partial_multiple_frames_eof_and_delimiter_recovery() {
        let mut codec = Qgi::new(Transport::Tcp);
        let options = DecodeOptions::default();
        let encoded = codec.reencode(&event(), &EncodeOptions::default()).unwrap();
        let mut buffer = encoded[..encoded.len() - 1].to_vec();
        let snapshot = buffer.clone();
        assert!(
            codec
                .decode_stream(&mut buffer, false, &options)
                .unwrap()
                .is_none()
        );
        assert_eq!(buffer, snapshot);
        buffer.push(b'\n');
        buffer.extend_from_slice(b"not-json\r\n");
        buffer.extend_from_slice(&encoded);
        assert_eq!(
            codec
                .decode_stream(&mut buffer, false, &options)
                .unwrap()
                .unwrap()
                .events
                .len(),
            1
        );
        assert_eq!(
            codec
                .decode_stream(&mut buffer, false, &options)
                .unwrap_err()
                .category,
            Category::Malformed
        );
        let out = codec
            .decode_stream(&mut buffer, false, &options)
            .unwrap()
            .unwrap();
        assert_eq!(out.events[0].value()["via"].as_array().unwrap().len(), 1);
        assert!(buffer.is_empty());
        buffer.extend_from_slice(b"unfinished");
        assert_eq!(
            codec
                .decode_stream(&mut buffer, true, &options)
                .unwrap_err()
                .category,
            Category::Truncated
        );
        assert!(buffer.is_empty());
        buffer.resize(1_048_577, b' ');
        assert_eq!(
            codec
                .decode_stream(&mut buffer, false, &options)
                .unwrap_err()
                .category,
            Category::Oversized
        );
        assert!(buffer.is_empty());
        buffer.extend_from_slice(b"discard this tail\n");
        buffer.extend_from_slice(&encoded);
        assert!(
            codec
                .decode_stream(&mut buffer, false, &options)
                .unwrap()
                .is_none()
        );
        assert!(
            codec
                .decode_stream(&mut buffer, false, &options)
                .unwrap()
                .is_some()
        );
        assert!(buffer.is_empty());
        codec.reset(None);
        assert_eq!(
            json::parse(b"{}", 1_048_576).unwrap_err().category,
            Category::InvalidValue
        );
    }
    #[test]
    fn producer_checks_nested_hop_additions_and_framing() {
        let mut v = event().into_value();
        v["via"] = json!([{"family":"qgi","context":{"direction":"unknown","exchange":"unknown","addition":true}}]);
        let e = Event::from_value(v).unwrap();
        assert!(json::write(&e, false, None).is_ok());
        assert!(Qgi::new(Transport::Udp).encode_event(&e, None).is_err());
        let mut v = event().into_value();
        v["fields"]["text"] = "x".repeat(65_507).into();
        let e = Event::from_value(v).unwrap();
        assert_eq!(
            Qgi::new(Transport::Udp)
                .encode_event(&e, None)
                .unwrap_err()
                .category,
            Category::Oversized
        );
    }
}
