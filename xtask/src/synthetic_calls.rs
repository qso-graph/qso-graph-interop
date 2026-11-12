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

fn check_bytes(bytes: &[u8]) -> Check {
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
    Ok(())
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

fn check_value(value: &Value) -> Check {
    match value {
        Value::String(s) => {
            check_bytes(s.as_bytes())?;
            for (alphabet, bits) in [
                (b"0123456789abcdef".as_slice(), 4),
                (
                    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".as_slice(),
                    6,
                ),
            ] {
                if let Some(bytes) = decoded(s, alphabet, bits) {
                    check_bytes(&bytes)?;
                }
            }
            // event_json and stream examples can themselves contain native.data.
            if let Ok(nested) = serde_json::from_str::<Value>(s)
                && (nested.is_object() || nested.is_array())
            {
                check_value(&nested)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                check_value(value)?;
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                check_value(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn check_file(path: &Path) -> Check {
    let text = read(path)?;
    check_bytes(text.as_bytes())?;
    if path.extension().is_some_and(|ext| ext == "json") {
        if let Ok(value) = serde_json::from_str::<Value>(&text) {
            check_value(&value)?;
        } else {
            // Some rejection vectors deliberately contain unpaired surrogates.
            // Still inspect their encoded payloads without validating their JSON.
            for part in text.split('"') {
                check_value(&Value::String(part.into()))?;
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
                    Some("rs" | "py" | "json" | "xml" | "TXT" | "md")
                )
            }) {
                check_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
    }
    for file in ["README.md", "fuzz/seed.py"] {
        check_file(&root.join(file))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let error = check(&root).unwrap_err();
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
