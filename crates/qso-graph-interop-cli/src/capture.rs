// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded NDJSON capture admission and UDP-only playback.
use crate::{
    Sim,
    alltxt::{MAX_DECODES, MAX_FILE, invalid},
    numeric_udp::NumericUdp,
    sim,
};
use qso_graph_interop::{
    Codec,
    model::{Event, MAX_RECORD},
    qgi::{Qgi, Transport},
};
use std::io;
use std::time::Instant;

pub struct Frame {
    pub bytes: Vec<u8>,
    pub at_ms: u64,
}
/// Convert a validated millisecond UTC instant without a host calendar or time-zone lookup.
fn instant_ms(s: &str) -> io::Result<i64> {
    if s.len() != 24 || s.as_bytes()[19] != b'.' {
        return Err(invalid(
            "capture received_at requires exactly millisecond UTC precision",
        ));
    }
    qso_graph_interop::model::Instant::try_from(s.to_owned())
        .map_err(|_| invalid("invalid capture received_at UTC instant"))?;
    let n = |a, b| s[a..b].parse::<i64>().unwrap_or(0);
    let (y, m, d) = (n(0, 4), n(5, 7), n(8, 10));
    let prev = y - 1;
    let mut days = 365 * prev + prev / 4 - prev / 100 + prev / 400;
    for month in 1..m {
        days += match month {
            4 | 6 | 9 | 11 => 30,
            2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
            2 => 28,
            _ => 31,
        };
    }
    days += d - 1;
    Ok((days - 719162) * 86_400_000
        + n(11, 13) * 3_600_000
        + n(14, 16) * 60_000
        + n(17, 19) * 1000
        + n(20, 23))
}
/// Optional observer extension, preserved by the reader profile. Not a native decode time.
fn received(e: &Event) -> io::Result<Option<i64>> {
    e.value()
        .get("received_at")
        .map(|v| {
            v.as_str()
                .ok_or_else(|| invalid("capture received_at must be a UTC instant"))
                .and_then(instant_ms)
        })
        .transpose()
}
fn qgi_bytes(event: &Event) -> io::Result<Vec<u8>> {
    let bytes =
        qso_graph_interop::json::write(event, false, None).map_err(|e| invalid(&e.to_string()))?;
    Qgi::new(Transport::Udp)
        .decode(&bytes, &Default::default())
        .map_err(|e| invalid(&e.to_string()))?;
    Ok(bytes)
}
pub fn parse(bytes: &[u8], carriage: bool) -> io::Result<Vec<Frame>> {
    if bytes.len() > MAX_FILE {
        return Err(invalid("capture exceeds 64 MiB"));
    }
    let mut frames = Vec::new();
    let mut previous = None;
    let mut at = 0_u64;
    for raw in bytes.split(|b| *b == b'\n') {
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        if raw.len() > MAX_RECORD {
            return Err(invalid("capture line exceeds 1 MiB"));
        }
        if raw.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if frames.len() == MAX_DECODES {
            return Err(invalid("capture exceeds 100000 events"));
        }
        let event =
            qso_graph_interop::json::parse(raw, MAX_RECORD).map_err(|e| invalid(&e.to_string()))?;
        let time = received(&event)?;
        if !frames.is_empty() {
            let delta = match (previous, time) {
                (Some(p), Some(t)) => u64::try_from(t - p)
                    .map_err(|_| invalid("capture receive times go backward"))?,
                _ => 1000,
            };
            at = at
                .checked_add(delta)
                .ok_or_else(|| invalid("capture duration overflow"))?;
        }
        previous = time;
        let family = event.value()["family"].as_str().unwrap_or("");
        let carried = carriage || family == "qgi";
        let payload = if carried {
            qgi_bytes(&event)?
        } else {
            // Transport catalog from the family annexes; never infer a transport from raw data.
            if !matches!(family, "wsjtx" | "n1mm" | "pskreporter-ipfix") {
                return Err(invalid(
                    "capture family has no supported UDP native transport; use --qgi for event carriage",
                ));
            }
            if event
                .value()
                .get("transport")
                .and_then(|t| t.as_str())
                .is_some_and(|t| t != "udp")
            {
                return Err(invalid(
                    "TCP capture playback requires the future stream transport helper",
                ));
            }
            let data = event.native_bytes().map_err(|e| invalid(&e.to_string()))?;
            if data.len() > 65_507 {
                return Err(invalid("capture native datagram exceeds 65507 bytes"));
            }
            if family == "wsjtx" {
                let out = qso_graph_interop::wsjtx::Wsjtx::default()
                    .decode(&data, &Default::default())
                    .map_err(|e| invalid(&e.to_string()))?;
                if out.events.len() != 1 {
                    return Err(invalid("capture has unsupported WSJT-X native evidence"));
                }
            }
            data
        };
        frames.push(Frame {
            bytes: payload,
            at_ms: at,
        });
    }
    if frames.is_empty() {
        return Err(invalid("capture contains no events"));
    }
    Ok(frames)
}
pub fn run(s: &Sim) -> Result<(), Box<dyn std::error::Error>> {
    let path = s
        .capture
        .as_ref()
        .ok_or_else(|| invalid("capture path is required"))?;
    let bytes = sim::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let frames = parse(&bytes, s.qgi)?;
    sim::delay(frames.last().map_or(0, |f| f.at_ms), s.speed)?;
    sim::install_stop()?;
    let socket = NumericUdp::sender(s.to.ok_or("--to is required")?)?;
    loop {
        let begun = Instant::now();
        for frame in &frames {
            let until = begun
                .checked_add(sim::delay(frame.at_ms, s.speed)?)
                .ok_or_else(|| invalid("capture deadline too large"))?;
            sim::wait_capture(&socket, until)?;
            if sim::stopped() {
                return Ok(());
            }
            socket.send(&frame.bytes)?;
        }
        if !s.repeat {
            break;
        }
        // Even a single-frame or zero-duration capture cannot busy-loop indefinitely.
        sim::wait_capture(&socket, Instant::now() + sim::delay(1000, s.speed)?)?;
        if sim::stopped() {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(at: Option<&str>) -> Vec<u8> {
        let bytes = qso_graph_interop::wsjtx::encode(&qso_graph_interop::wsjtx::Packet::new(
            "synthetic",
            qso_graph_interop::wsjtx::Body::Close,
        ))
        .unwrap();
        let e = qso_graph_interop::wsjtx::Wsjtx::default()
            .decode(&bytes, &Default::default())
            .unwrap()
            .events
            .remove(0);
        let mut v = e.into_value();
        if let Some(at) = at {
            v["received_at"] = at.into();
        }
        serde_json::to_vec(&v).unwrap()
    }
    #[test]
    fn tcp_families_malformed_evidence_and_datagram_limits_are_rejected() {
        let mut v: serde_json::Value = serde_json::from_slice(&record(None)).unwrap();
        for family in ["js8call", "fldigi", "pskreporter-mqtt", "unknown"] {
            v["family"] = family.into();
            let bytes = serde_json::to_vec(&v).unwrap();
            assert!(parse(&bytes, false).is_err(), "{family}");
            assert!(parse(&bytes, true).is_ok(), "qgi can carry {family}");
        }
        for family in ["n1mm", "pskreporter-ipfix"] {
            v["family"] = family.into();
            assert_eq!(
                parse(&serde_json::to_vec(&v).unwrap(), false).unwrap()[0].bytes,
                serde_json::from_slice::<Event>(&record(None))
                    .unwrap()
                    .native_bytes()
                    .unwrap()
            );
        }
        v["family"] = "wsjtx".into();
        v["native"]["data"] = "!!!!".into();
        assert!(parse(&serde_json::to_vec(&v).unwrap(), false).is_err());
        v = serde_json::from_slice(&record(None)).unwrap();
        v["transport"] = "tcp".into();
        assert!(parse(&serde_json::to_vec(&v).unwrap(), false).is_err());
    }

    #[test]
    fn relative_times_fallback_bounds_and_carriage() {
        let mut bytes = record(Some("2026-11-12T00:00:00.000Z"));
        bytes.push(b'\n');
        bytes.extend(record(Some("2026-11-12T00:00:01.250Z")));
        bytes.push(b'\n');
        bytes.extend(record(None));
        assert_eq!(
            parse(&bytes, false)
                .unwrap()
                .iter()
                .map(|f| f.at_ms)
                .collect::<Vec<_>>(),
            [0, 1250, 2250]
        );
        let frames = parse(&bytes, true).unwrap();
        assert!(
            Qgi::new(Transport::Udp)
                .decode(&frames[0].bytes, &Default::default())
                .is_ok()
        );
        let mut reversed = record(Some("2026-11-12T00:00:01.000Z"));
        reversed.push(b'\n');
        reversed.extend(record(Some("2026-11-12T00:00:00.000Z")));
        assert!(parse(&reversed, false).is_err());
        assert!(parse(b"", false).is_err());
        for stamp in [
            "2026-11-12T00:00:00Z",
            "2026-11-12T00:00:00.1Z",
            "2026-11-12T00:00:00.1234Z",
        ] {
            assert!(parse(&record(Some(stamp)), false).is_err());
        }
        assert!(parse(&vec![b' '; MAX_RECORD + 1], false).is_err());
        assert_eq!(instant_ms("1970-01-01T00:00:00.001Z").unwrap(), 1);
        assert_eq!(
            instant_ms("2000-02-29T00:00:00.000Z").unwrap(),
            951782400000
        );
    }
}
