// SPDX-License-Identifier: GPL-3.0-or-later
//! Prevent retired invented calls from returning in source or encoded fixtures.
use crate::{Check, files, read};
use serde_json::Value;
use std::path::Path;

// Every fixed non-Q invented base call replaced in the Q-prefix migration.
// Portable variants and lowercase input are covered by the same base call.
const OLD_CALLS: &[&str] = &[
    "K1AAA", "K1BBB", "K7ABC", "W1AAA", "W1AAB", "W1AAC", "W1AAD", "W1AAE", "W1AAF", "W1AAG",
    "W1AAH", "W1AAI", "W1AAJ", "W1AAK", "W1AAL", "W1AAM", "W1AAN", "W1AAO", "W1AAP", "W1AAQ",
    "W1AAR", "W1AAS", "W1AAT", "W1AAU", "W1AAV", "W1AAW", "W1AAX", "W1AAY", "W1AAZ", "W1ABA",
    "W1ABB", "W1ABC", "W1ABD", "W1ABE", "W1ABF", "W1ABG", "W1ABH", "W1ABI", "W1ABJ", "W1ABK",
    "W1ABL", "W1ABM", "W1ABN", "W1ABO", "W1ABP", "W1ABQ", "W1ABR", "W1ABS", "W1ABT", "W1ABU",
    "W1ABV", "W1ABW", "W1ABX", "W1ABY", "W1ABZ", "W1ACA", "W1ACB", "W1ACC", "W1ACD", "W1ACE",
    "W1ACF", "W1ACG", "W1ACH", "W1ACI", "W1ACJ", "W1ACK", "W1ACL", "W1ACM", "W1ACN", "W1ACO",
    "W1ACP", "W1ACQ", "W1ACR", "W1ACS", "W1ACT", "W1ACU", "W1ACV", "W1ACW", "W1ACX", "W1ACY",
    "W1ACZ", "W1ADA", "W1ADB", "W1ADC", "W1ADD", "W1ADE", "W1ADF", "W1ADG", "W1ADH", "W1ADI",
    "W1AW", "W1BBB", "W1ZZZ",
];

// FT8 cannot pack Q-prefixed synthetic calls as standard messages. KI7MT authorized
// exactly his current/former, checked-not-reissued calls for live-capture fixtures.
const LIVE_CALLS: &[&str] = &["KI7MT", "KE1HA", "KN4YU", "KC4QPQ", "M0EHA"];

fn real_call(token: &str) -> bool {
    let bytes = token.as_bytes();
    let prefix = if bytes.first().is_some_and(u8::is_ascii_digit)
        && bytes.get(1).is_some_and(u8::is_ascii_uppercase)
    {
        2
    } else {
        bytes.iter().take_while(|b| b.is_ascii_uppercase()).count()
    };
    (1..=2).contains(&prefix)
        && bytes.get(prefix).is_some_and(u8::is_ascii_digit)
        && (1..=4).contains(&bytes.len().saturating_sub(prefix + 1))
        && bytes[prefix + 1..].iter().all(u8::is_ascii_uppercase)
}

fn check_bytes_at(bytes: &[u8], live: bool) -> Check {
    // UTF-16 examples have NULs between ASCII letters. Also inspect that view.
    let text = String::from_utf8_lossy(bytes).to_ascii_uppercase();
    let compact = text.replace('\0', "");
    for call in OLD_CALLS {
        if compact.contains(call) {
            return Err(format!(
                "retired invented callsign {call}; use its Q-prefixed replacement"
            ));
        }
    }
    // Remove only the known software label in its XML app/database context.
    // N1MM has the lexical shape of a call, but these two strings name the program.
    let calls = compact
        .replace("<APP>N1MM</APP>", "")
        .replace("N1MM DXLOG.S3DB", "");
    for token in calls.split(|c: char| !c.is_ascii_alphanumeric() && c != '/') {
        if LIVE_CALLS.contains(&token) {
            if !live {
                return Err(format!(
                    "live callsign {token} outside live-capture fixtures"
                ));
            }
        } else if live
            && token
                .split('/')
                .any(|part| !part.starts_with('Q') && real_call(part))
        {
            return Err(format!(
                "non-Q callsign {token} is not an authorized live call"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
fn check_bytes(bytes: &[u8]) -> Check {
    check_bytes_at(bytes, false)
}

fn decoded(text: &str, alphabet: &[u8], bits: u32) -> Option<Vec<u8>> {
    if text.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    let mut pending = 0_u32;
    let mut count = 0;
    for byte in text.trim_end_matches('=').bytes() {
        let value = alphabet.iter().position(|b| *b == byte)? as u32;
        pending = (pending << bits) | value;
        count += bits;
        if count >= 8 {
            count -= 8;
            out.push((pending >> count) as u8);
        }
    }
    Some(out)
}

fn check_value_at(value: &Value, live: bool) -> Check {
    match value {
        Value::String(s) => {
            check_bytes_at(s.as_bytes(), live)?;
            for (alphabet, bits) in [
                (b"0123456789abcdef".as_slice(), 4),
                (
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".as_slice(),
                    6,
                ),
            ] {
                if let Some(bytes) = decoded(s, alphabet, bits) {
                    check_bytes_at(&bytes, live)?;
                }
            }
            // event_json and stream examples can themselves contain native.data.
            if let Ok(nested) = serde_json::from_str::<Value>(s)
                && (nested.is_object() || nested.is_array())
            {
                check_value_at(&nested, live)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                check_value_at(value, live)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                check_value_at(value, live)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
fn check_value(value: &Value) -> Check {
    check_value_at(value, false)
}

fn check_recorder_bytes(bytes: &[u8], live: bool) -> Check {
    // WSJT-X Decode: after its length-prefixed ID, the new flag, QTime, SNR,
    // delta time and audio offset are 21 scalar bytes, not callsign text.
    // For example, QTime 7,485,000 ms contains ASCII r6H. Keep the ID, mode,
    // decoded text and trailing extensions in the scan. Incomplete framing
    // falls back to scanning all supplied bytes rather than hiding text.
    if bytes.starts_with(&[0xad, 0xbc, 0xcb, 0xda])
        && bytes
            .get(4..8)
            .is_some_and(|schema| schema == [0, 0, 0, 2] || schema == [0, 0, 0, 3])
        && bytes.get(8..12) == Some(&[0, 0, 0, 2])
        && let Some(length) = bytes.get(12..16)
        && let Ok(length) = usize::try_from(u32::from_be_bytes(length.try_into().expect("four")))
        && let Some(start) = 16_usize.checked_add(length)
        && let Some(end) = start.checked_add(21)
        && end <= bytes.len()
    {
        let mut text = bytes.to_vec();
        text[start..end].fill(0);
        return check_bytes_at(&text, live);
    }
    check_bytes_at(bytes, live)
}

fn check_file(path: &Path, live: bool) -> Check {
    let mut text = read(path)?;
    // Existing release governance names the account owner, not a test QSO.
    // Mask only those identity phrases; a callsign in a new fixture still fails.
    if path.ends_with("scripts/release_check.py") {
        text = text
            .replace(
                "KI7MT's manual main Release",
                "the owner's manual main Release",
            )
            .replace("actor.casefold() == \"ki7mt\"", "actor identity check");
    }
    if path.ends_with("scripts/test_release_check.py") {
        text = text
            .replace("actor=\"KI7MT\"", "actor=\"owner\"")
            .replace("actor=\"ki7mt\"", "actor=\"owner\"")
            .replace("\"KI7MT-other\"", "\"owner-other\"");
    }
    if path.ends_with("docs/RELEASING.md") {
        text = text
            .replace("KI7MT runs", "The owner runs")
            .replace("be KI7MT.", "be the owner.");
    }
    if !path.extension().is_some_and(|ext| ext == "ndjson") {
        check_bytes_at(text.as_bytes(), live)?;
    }
    if path.extension().is_some_and(|ext| ext == "ndjson") {
        for line in text.lines() {
            let value: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
            for (key, value) in value.as_object().ok_or("recorder object required")? {
                if !["data_b64", "request_b64", "response_b64"].contains(&key.as_str()) {
                    // Recorder labels such as SO1V are not on-wire callsigns.
                    check_value_at(value, false)?;
                }
            }
            for key in ["data_b64", "request_b64", "response_b64"] {
                if let Some(encoded) = value.get(key).and_then(Value::as_str) {
                    if encoded.is_empty() {
                        continue; // A timed-out recorder exchange has no response bytes.
                    }
                    let bytes = decoded(
                        encoded,
                        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
                        6,
                    )
                    .ok_or("invalid recorder base64")?;
                    check_recorder_bytes(&bytes, live)?;
                }
            }
        }
    }
    if path.extension().is_some_and(|ext| ext == "json") {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            check_value_at(&value, live)?;
        } else {
            // Some rejection vectors deliberately contain unpaired surrogates.
            // Still inspect their encoded payloads without validating their JSON.
            for part in text.split('"') {
                check_value_at(&Value::String(part.into()), live)?;
            }
        }
    }
    Ok(())
}

pub fn check(root: &Path) -> Check {
    // Keep the retired-name oracle here, outside the scanned product sources.
    for directory in ["crates", "scripts", "spec", "docs", "fuzz/fuzz_targets"] {
        for path in files(&root.join(directory))? {
            if path.extension().is_some_and(|ext| {
                matches!(
                    ext.to_str(),
                    Some("rs" | "py" | "json" | "ndjson" | "xml" | "TXT" | "md")
                )
            }) {
                check_file(
                    &path,
                    path.starts_with(
                        root.join("crates/qso-graph-interop/tests/fixtures/live-captures"),
                    ),
                )
                .map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
    }
    for file in ["README.md", "fuzz/seed.py"] {
        check_file(&root.join(file), false)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wsjtx_decode_scalar_bytes_are_not_callsign_text() {
        let capture = include_str!(
            "../../crates/qso-graph-interop/tests/fixtures/live-captures/wsjtx-3.3.0-beta6-ft8-ipfix-udp.ndjson"
        );
        let row: Value = serde_json::from_str(capture.lines().nth(137).unwrap()).unwrap();
        let raw = decoded(
            row["data_b64"].as_str().unwrap(),
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
            6,
        )
        .unwrap();
        assert!(check_bytes_at(&raw, true).unwrap_err().contains("R6H"));
        assert!(check_recorder_bytes(&raw, true).is_ok());
        assert!(check_recorder_bytes(&raw, false).is_err());
        let mut future = raw.clone();
        future[7] = 4;
        assert!(check_recorder_bytes(&future, true).is_err());
        let mut planted = raw.clone();
        let call = planted.windows(3).position(|w| w == b"CQ ").unwrap() + 3;
        planted[call..call + 5].copy_from_slice(b"W9XYZ");
        assert!(
            check_recorder_bytes(&planted, true)
                .unwrap_err()
                .contains("W9XYZ")
        );
        let mut extension = raw.clone();
        extension.extend_from_slice(b" W9XYZ ");
        assert!(check_recorder_bytes(&extension, true).is_err());
        let mut id = raw;
        id[16..21].copy_from_slice(b"W9XYZ");
        assert!(check_recorder_bytes(&id, true).is_err());
    }

    #[test]
    fn live_calls_are_exact_and_confined_to_fixtures() {
        for call in LIVE_CALLS {
            assert!(check_bytes_at(call.as_bytes(), true).is_ok());
            assert!(check_bytes_at(call.as_bytes(), false).is_err());
            for changed in [format!("{call}/P"), format!("{call}A")] {
                assert!(check_bytes_at(changed.as_bytes(), true).is_err());
            }
        }
        for call in ["W9XYZ", "AB1CD", "G4XYZ", "K1AAA", "3D2XYZ"] {
            assert!(check_bytes_at(call.as_bytes(), true).is_err(), "{call}");
        }
        assert!(check_bytes_at(b"CQ Q1ABC FN42", true).is_ok());
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/policy-live-planted");
        let fixtures = root.join("crates/qso-graph-interop/tests/fixtures/live-captures");
        std::fs::create_dir_all(&fixtures).unwrap();
        for directory in ["scripts", "spec", "docs", "fuzz/fuzz_targets"] {
            std::fs::create_dir_all(root.join(directory)).unwrap();
        }
        std::fs::write(root.join("README.md"), "Q1ABC").unwrap();
        std::fs::write(root.join("fuzz/seed.py"), "Q1ABC").unwrap();
        let path = fixtures.join("planted.ndjson");
        std::fs::write(&path, r#"{"data_b64":"S0UxSEE="}"#).unwrap();
        assert!(check(&root).is_ok());
        // A new, non-retired real-format call hidden in base64 must fail.
        std::fs::write(&path, r#"{"data_b64":"VzlYWVo="}"#).unwrap();
        assert!(check(&root).unwrap_err().contains("W9XYZ"));
        std::fs::write(&path, r#"{"data_b64":"S0UxSEE="}"#).unwrap();
        std::fs::rename(&path, root.join("crates/planted.ndjson")).unwrap();
        assert!(check(&root).unwrap_err().contains("outside live-capture"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn planted_retired_calls_are_caught_in_text_hex_and_base64() {
        for call in OLD_CALLS {
            assert!(check_bytes(call.as_bytes()).is_err());
            assert!(
                check_bytes(format!("<call>{}/P</call>", call.to_lowercase()).as_bytes()).is_err()
            );
        }
        for text in [
            r#"{"call":"Q1AAA","native":{"data":"VzFBQUE="}}"#,
            r#"{"wire_hex":"0000573141414100"}"#,
            r#"{"wire_hex":"00570031004100410041"}"#,
            r#"{"event_json":"{\"data\":\"VzFBQUE=\"}"}"#,
        ] {
            let value: Value = serde_json::from_str(text).unwrap();
            assert!(
                check_value(&value).is_err(),
                "missed planted legacy call: {text}"
            );
        }
        let value: Value = serde_json::from_str(
            r#"{"call":"Q1AAA/P","station_call":"N0CALL","data":"UTFBQUE=","hex":"5131414141"}"#,
        )
        .unwrap();
        assert!(check_value(&value).is_ok());
    }

    #[test]
    fn planted_files_are_caught_in_each_policy_directory() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/policy-calls-planted");
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        for directory in [
            "crates",
            "scripts",
            "spec/vectors",
            "docs",
            "fuzz/fuzz_targets",
        ] {
            std::fs::create_dir_all(root.join(directory)).unwrap();
        }
        for file in ["README.md", "fuzz/seed.py"] {
            std::fs::write(root.join(file), "Q1AAA").unwrap();
        }
        assert!(check(&root).is_ok());
        for file in [
            "crates/fixture.xml",
            "scripts/generator.py",
            "spec/vectors/planted.json",
        ] {
            let path = root.join(file);
            std::fs::write(&path, r#"{"call":"W1AAA"}"#).unwrap();
            // Windows prints the walked path with backslashes.
            let error = check(&root).unwrap_err().replace('\\', "/");
            assert!(error.contains(file) && error.contains("W1AAA"), "{error}");
            std::fs::remove_file(path).unwrap();
            assert!(check(&root).is_ok());
        }
        let path = root.join("spec/vectors/surrogate.json");
        std::fs::write(&path, r#"{"app":"\ud800","data":"UTFBQUE="}"#).unwrap();
        assert!(check(&root).is_ok());
        std::fs::write(&path, r#"{"app":"\ud800","data":"VzFBQUE="}"#).unwrap();
        assert!(check(&root).unwrap_err().contains("W1AAA"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
