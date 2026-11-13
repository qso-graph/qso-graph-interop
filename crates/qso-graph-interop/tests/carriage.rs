// SPDX-License-Identifier: GPL-3.0-or-later
//! Partial frames, multiple frames, EOF and recovery through caller-owned buffers.
#![forbid(unsafe_code)]
#[cfg(feature = "qgi")]
mod tests {
    use qso_graph_interop::{
        Category, Codec, Event,
        codec::{DecodeOptions, StreamStatus},
        json,
        qgi::{Qgi, Transport},
    };
    use serde_json::json;
    fn frame(
        codec: &mut Qgi,
        buffer: &mut Vec<u8>,
        eof: bool,
        options: &DecodeOptions<'_>,
    ) -> Result<Option<qso_graph_interop::Outcome>, qso_graph_interop::Error> {
        let step = codec.stream().unwrap().decode_stream(buffer, eof, options);
        buffer.drain(..step.consumed());
        match step.into_result()? {
            StreamStatus::NeedMore | StreamStatus::Discarded => Ok(None),
            StreamStatus::Frame(out) => Ok(Some(out)),
            _ => panic!("unexpected stream status"),
        }
    }
    fn event() -> Event {
        Event::from_value(json!({"spec":"0.1.0","family":"qgi","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"fields":{"operation":"replay"}})).unwrap()
    }
    #[test]
    fn partial_multiple_frames_eof_and_delimiter_recovery() {
        let mut codec = Qgi::new(Transport::Tcp);
        let options = DecodeOptions::default();
        let encoded = codec
            .encode_event(&event(), &qso_graph_interop::PublicationOptions::default())
            .unwrap();
        let mut buffer = encoded[..encoded.len() - 1].to_vec();
        let snapshot = buffer.clone();
        assert!(
            frame(&mut codec, &mut buffer, false, &options)
                .unwrap()
                .is_none()
        );
        assert_eq!(buffer, snapshot);
        buffer.push(b'\n');
        buffer.extend_from_slice(b"not-json\r\n");
        buffer.extend_from_slice(&encoded);
        assert_eq!(
            frame(&mut codec, &mut buffer, false, &options)
                .unwrap()
                .unwrap()
                .events
                .len(),
            1
        );
        assert_eq!(
            frame(&mut codec, &mut buffer, false, &options)
                .unwrap_err()
                .category(),
            Category::Malformed
        );
        let out = frame(&mut codec, &mut buffer, false, &options)
            .unwrap()
            .unwrap();
        assert_eq!(out.events[0].value()["via"].as_array().unwrap().len(), 1);
        assert!(buffer.is_empty());
        buffer.extend_from_slice(b"unfinished");
        assert_eq!(
            frame(&mut codec, &mut buffer, true, &options)
                .unwrap_err()
                .category(),
            Category::Truncated
        );
        assert!(buffer.is_empty());
        buffer.resize(1_048_577, b' ');
        assert_eq!(
            frame(&mut codec, &mut buffer, false, &options)
                .unwrap_err()
                .category(),
            Category::Oversized
        );
        assert!(buffer.is_empty());
        buffer.extend_from_slice(b"discard this tail\n");
        buffer.extend_from_slice(&encoded);
        assert!(
            frame(&mut codec, &mut buffer, false, &options)
                .unwrap()
                .is_none()
        );
        assert!(
            frame(&mut codec, &mut buffer, false, &options)
                .unwrap()
                .is_some()
        );
        assert!(buffer.is_empty());
        codec.reset(None);
        assert_eq!(
            json::parse(b"{}", 1_048_576).unwrap_err().category(),
            Category::InvalidValue
        );
    }
    #[test]
    fn producer_checks_nested_hop_additions_and_framing() {
        let mut v = event().into_value();
        v["via"] = json!([{"family":"qgi","context":{"direction":"unknown","exchange":"unknown","addition":true}}]);
        let e = Event::from_value(v).unwrap();
        assert!(
            json::write(
                &e,
                &qso_graph_interop::PublicationOptions::new(
                    qso_graph_interop::PublicationMode::Preserved
                )
            )
            .is_ok()
        );
        assert!(
            Qgi::new(Transport::Udp)
                .encode_event(&e, &qso_graph_interop::PublicationOptions::default())
                .is_err()
        );
        let mut v = event().into_value();
        v["fields"]["text"] = "x".repeat(65_507).into();
        let e = Event::from_value(v).unwrap();
        assert_eq!(
            Qgi::new(Transport::Udp)
                .encode_event(&e, &qso_graph_interop::PublicationOptions::default())
                .unwrap_err()
                .category(),
            Category::Oversized
        );
    }
    #[test]
    fn recovery_drains_buffered_frame_without_new_input() {
        let mut codec = Qgi::new(Transport::Tcp);
        let options = DecodeOptions::default();
        let mut buffer = vec![b' '; 1_048_577];
        assert!(frame(&mut codec, &mut buffer, false, &options).is_err());
        buffer.extend_from_slice(b"tail\n");
        buffer.extend_from_slice(
            &codec
                .encode_event(&event(), &qso_graph_interop::PublicationOptions::default())
                .unwrap(),
        );
        let mut frames = 0;
        loop {
            let step = codec
                .stream()
                .unwrap()
                .decode_stream(&buffer, false, &options);
            buffer.drain(..step.consumed());
            match step.into_result().unwrap() {
                StreamStatus::NeedMore => break,
                StreamStatus::Discarded => continue,
                StreamStatus::Frame(out) => frames += out.events.len(),
                _ => panic!("unexpected stream status"),
            }
        }
        assert_eq!(frames, 1);
        assert!(buffer.is_empty());
    }
}
