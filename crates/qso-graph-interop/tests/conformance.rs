// SPDX-License-Identifier: GPL-3.0-or-later
//! Acceptance: every manifest entry, every on-disk vector, every instruction class.
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Codec,
    conformance::{check_hash, run_vector},
    qgi::{Delimiter, Qgi, Transport},
    wsjtx::Wsjtx,
};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
#[test]
fn all_specification_vectors() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/vectors");
    let mut failures = Vec::new();
    for family in ["wsjtx", "qgi"] {
        let dir = base.join(family);
        let manifest: Value =
            serde_json::from_slice(&std::fs::read(dir.join("manifest.json")).unwrap()).unwrap();
        if family == "wsjtx" {
            assert_eq!(
                manifest["source_sha256"],
                "fd73b7f92fc0f66ea8f12a147b59a6072408c24656735d59df772dc55a5eeb62"
            );
        }
        if let Some(source) = manifest["source"].as_str() {
            assert!(check_hash(
                &std::fs::read(dir.join(source)).unwrap(),
                manifest["source_sha256"].as_str().unwrap()
            ));
        }
        let mut seen = BTreeSet::new();
        let mut counts = BTreeMap::<String, usize>::new();
        for entry in manifest["vectors"].as_array().unwrap() {
            let name = entry["file"].as_str().unwrap();
            assert!(seen.insert(name.to_owned()), "duplicate manifest entry");
            let data = std::fs::read(dir.join(name)).unwrap();
            assert!(
                check_hash(&data, entry["sha256"].as_str().unwrap()),
                "{family}/{name} hash"
            );
            let vector: Value = serde_json::from_slice(&data).unwrap();
            for key in ["spec", "family", "mapping", "class"] {
                assert_eq!(entry[key], vector[key], "{family}/{name} {key}");
            }
            let mut codec: Box<dyn Codec> = if family == "wsjtx" {
                Box::new(Wsjtx::default())
            } else {
                let transport = match vector["input"]["transport"].as_str() {
                    Some("udp") => Transport::Udp,
                    Some("tcp") => Transport::Tcp,
                    _ => panic!("unhandled transport"),
                };
                let delimiter = if vector["reencode"]["delimiter"] == "\r\n" {
                    Delimiter::CrLf
                } else {
                    Delimiter::Lf
                };
                Box::new(Qgi::new(transport).with_delimiter(delimiter))
            };
            match run_vector(codec.as_mut(), &vector) {
                Ok(()) => {
                    *counts
                        .entry(vector["class"].as_str().unwrap().into())
                        .or_default() += 1;
                    if vector.get("reencode").is_some() {
                        *counts.entry("reencode (embedded)".into()).or_default() += 1;
                    }
                }
                Err(e) => failures.push(format!("{family}/{name}: {e}")),
            }
        }
        let files: BTreeSet<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".json") && n != "manifest.json")
            .collect();
        assert_eq!(seen, files, "unexecuted vector files");
        eprintln!("{family}: {counts:?}");
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
#[test]
fn planted_manifest_and_unknown_class_failures() {
    assert!(check_hash(
        b"abc",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    ));
    assert!(!check_hash(
        b"abd",
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    ));
    assert!(
        run_vector(
            &mut Wsjtx::default(),
            &serde_json::json!({"class":"unrecognized"})
        )
        .is_err()
    );
}

#[test]
fn declared_message_limits() {
    use qso_graph_interop::conformance::check_message_limit;
    assert!(check_message_limit(&mut Wsjtx::default()).is_ok());
    assert!(check_message_limit(&mut Qgi::new(Transport::Udp)).is_ok());
    assert!(check_message_limit(&mut Qgi::new(Transport::Tcp)).is_ok());
}
