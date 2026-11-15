// SPDX-License-Identifier: GPL-3.0-or-later
//! Every family's failures carry its own error type as their source (QGI-RI-0024).
#![forbid(unsafe_code)]
use qso_graph_interop::{
    Category, Error, Event, Profile, VerifyOptions, fldigi, n1mm, pskreporter_ipfix,
    pskreporter_mqtt, qgi, wsjtx,
};
use serde_json::json;
use std::error::Error as _;

/// A family name, its profile's failure, and whether that failure carries the family error.
type Check = (&'static str, Error, fn(&Error) -> bool);

fn source_is<T: std::error::Error + 'static>(error: &Error) -> bool {
    error.source().is_some_and(|e| e.is::<T>())
}

/// An observation from another family, so verify() returns Unavailable, carrying the native
/// version this profile supports, so the compatible view projects (and rejects) its field.
fn foreign_spot(native_version: Option<&str>) -> Event {
    let mut v = json!({"spec":"0.2.0","family":"x-example.test","mapping":1,"kind":"spot",
        "context":{"direction":"unknown","exchange":"unknown"},
        "fields":{"frequency_hz":"not a number"},"native":{"format":"opaque","data":"AA=="}});
    if let Some(version) = native_version {
        v["native_version"] = json!(version);
    }
    Event::from_value(v).unwrap()
}

#[test]
fn recognize_wraps_compatible_view_failures_in_every_family() {
    let o = VerifyOptions::default();
    let checks: [Check; 6] = [
        (
            "wsjtx",
            wsjtx::Wsjtx::default()
                .recognize(&foreign_spot(Some("3")), &o)
                .unwrap_err(),
            source_is::<wsjtx::Error>,
        ),
        (
            "n1mm",
            n1mm::N1mm::default()
                .recognize(&foreign_spot(None), &o)
                .unwrap_err(),
            source_is::<n1mm::N1mmError>,
        ),
        (
            "fldigi",
            fldigi::Fldigi::default()
                .recognize(&foreign_spot(None), &o)
                .unwrap_err(),
            source_is::<fldigi::FldigiError>,
        ),
        (
            "qgi",
            qgi::Qgi::new(qgi::Transport::Udp)
                .recognize(&foreign_spot(None), &o)
                .unwrap_err(),
            source_is::<qgi::Error>,
        ),
        (
            "pskreporter-ipfix",
            pskreporter_ipfix::PskReporterIpfix::default()
                .recognize(&foreign_spot(Some("10")), &o)
                .unwrap_err(),
            source_is::<pskreporter_ipfix::IpfixError>,
        ),
        (
            "pskreporter-mqtt",
            pskreporter_mqtt::PskReporterMqtt::default()
                .recognize(&foreign_spot(Some("2")), &o)
                .unwrap_err(),
            source_is::<pskreporter_mqtt::MqttError>,
        ),
    ];
    for (family, error, has_family_source) in checks {
        assert_eq!(
            error.category(),
            Category::InvalidValue,
            "{family}: {error}"
        );
        assert!(
            has_family_source(&error),
            "{family}: no family source: {error}"
        );
    }
}
