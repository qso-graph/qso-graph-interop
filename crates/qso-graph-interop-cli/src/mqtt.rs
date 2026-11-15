// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded local MQTT capture adapter. No transport implementation.
#![forbid(unsafe_code)]
use qso_graph_interop::{PublicationOptions, json, pskreporter_mqtt};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;

// The supplied capture is below 1 MiB. NDJSON is streamed with a per-line cap.
const MAX_CAPTURE: usize = qso_graph_interop::model::MAX_RECORD;
const MAX_LINE: usize = 32_768;
fn emit(
    record: &serde_json::Value,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = pskreporter_mqtt::decode_record(record)?;
    if !outcome.skips.is_empty() {
        return Err("unsupported future MQTT version (future_version skip)".into());
    }
    for event in outcome.events {
        output.write_all(&json::write(&event, &PublicationOptions::default())?)?;
        writeln!(output)?;
    }
    Ok(())
}
fn decode(input: impl Read, output: &mut dyn Write) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(input);
    let mut first = Vec::new();
    // Distinguish one-line capture/record and multiline JSON without an unbounded read_line.
    reader
        .by_ref()
        .take((MAX_CAPTURE + 1) as u64)
        .read_until(b'\n', &mut first)?;
    if first.len() > MAX_CAPTURE {
        return Err("MQTT capture exceeds 1 MiB".into());
    }
    if let Ok(value) = json::parse_value(&first, MAX_CAPTURE) {
        if value.get("records").is_some() {
            let mut trailing = Vec::new();
            reader
                .take((MAX_CAPTURE + 1 - first.len()) as u64)
                .read_to_end(&mut trailing)?;
            if first.len() + trailing.len() > MAX_CAPTURE
                || !trailing.iter().all(u8::is_ascii_whitespace)
            {
                return Err("oversized capture or data after capture object".into());
            }
            return capture(&value, output);
        }
        if first.len() > MAX_LINE {
            return Err("MQTT capture line exceeds 32768 bytes".into());
        }
        emit(&value, output)?;
        loop {
            let mut line = Vec::new();
            let n = reader
                .by_ref()
                .take((MAX_LINE + 1) as u64)
                .read_until(b'\n', &mut line)?;
            if n == 0 {
                return Ok(());
            }
            if line.len() > MAX_LINE {
                return Err("MQTT capture line exceeds 32768 bytes".into());
            }
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            emit(&json::parse_value(&line, MAX_LINE)?, output)?;
        }
    }
    let mut rest = Vec::new();
    reader
        .take((MAX_CAPTURE + 1 - first.len()) as u64)
        .read_to_end(&mut rest)?;
    first.extend(rest);
    capture(&json::parse_value(&first, MAX_CAPTURE)?, output)
}
fn capture(
    value: &serde_json::Value,
    output: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let records = value
        .get("records")
        .and_then(serde_json::Value::as_array)
        .ok_or("capture must have a records array")?;
    for (index, record) in records.iter().enumerate() {
        emit(record, output).map_err(|e| format!("MQTT capture record {index}: {e}"))?;
    }
    Ok(())
}
pub fn file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    if path.as_os_str() == "-" {
        decode(io::stdin().lock(), &mut output)?;
    } else {
        decode(std::fs::File::open(path)?, &mut output)?;
    }
    output.flush()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_wrappers_and_invalid_records() {
        let record = r#"{"topic":"pskr/rxinfo/Q1AAA","payload_b64":"e30="}"#;
        for input in [
            format!("{{\n\"records\":[{record}]\n}}"),
            format!("{record}\n"),
            format!("{{\"records\":[{record}]}}\n\n  "),
        ] {
            let mut out = Vec::new();
            decode(input.as_bytes(), &mut out).unwrap();
            let event: serde_json::Value = serde_json::from_slice(&out).unwrap();
            assert_eq!(event["kind"], "receiver_information");
            assert_eq!(event["native"]["retained"], false);
        }
        for input in [
            format!("{record}\n{record}\n"),
            format!("{record}\n\n{record}"),
        ] {
            let mut out = Vec::new();
            decode(input.as_bytes(), &mut out).unwrap();
            assert_eq!(out.iter().filter(|b| **b == b'\n').count(), 2);
        }
        for input in [
            r#"{"records":[],"records":[]}"#,
            r#"{"topic":"pskr/rxinfo/Q1AAA","payload_b64":"e30=","qos":3}"#,
            r#"{"topic":"pskr/rxinfo/Q1AAA","payload_b64":"?"}"#,
        ] {
            assert!(decode(input.as_bytes(), &mut Vec::new()).is_err());
        }
        assert!(decode(&vec![b' '; MAX_CAPTURE + 1][..], &mut Vec::new()).is_err());
    }
    #[test]
    fn capture_and_line_bounds_are_independent() {
        let record = r#"{"topic":"pskr/rxinfo/Q1AAA","payload_b64":"e30="}"#;
        let capture = format!("{{\"records\":[{}]}}", vec![record; 1000].join(","));
        assert!(capture.len() > MAX_LINE);
        let mut output = Vec::new();
        decode(capture.as_bytes(), &mut output).unwrap();
        assert_eq!(output.iter().filter(|b| **b == b'\n').count(), 1000);
        let mut boundary = format!("{{\"records\":[{record}]}}");
        boundary.push_str(&" ".repeat(MAX_CAPTURE - boundary.len()));
        decode(boundary.as_bytes(), &mut Vec::new()).unwrap();
        boundary.push(' ');
        assert!(decode(boundary.as_bytes(), &mut Vec::new()).is_err());
        let oversized_line = format!(
            "{{\"topic\":\"pskr/rxinfo/Q1AAA\",\"payload_b64\":\"e30=\",\"extra\":\"{}\"}}",
            "x".repeat(MAX_LINE)
        );
        assert!(
            decode(oversized_line.as_bytes(), &mut Vec::new())
                .unwrap_err()
                .to_string()
                .contains("line exceeds")
        );
        let future =
            r#"{"topic":"pskr/filter/v3/20m/FT8/Q1AAA/Q2BBB/AA00/BB11/1/2","payload_b64":"e30="}"#;
        assert!(
            decode(future.as_bytes(), &mut Vec::new())
                .unwrap_err()
                .to_string()
                .contains("future_version")
        );
    }
}
