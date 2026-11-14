// SPDX-License-Identifier: GPL-3.0-or-later
//! Numeric unicast collector and synthetic Annex H submissions.
use crate::numeric_udp::NumericUdp;
use qso_graph_interop::{
    Event,
    pskreporter_ipfix::{SubmissionOptions, SubmissionProfile, encode_submission},
};
use qso_graph_interop_cli::collector::Collector;
use serde_json::{Value, json};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub fn unicast(value: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = value
        .parse()
        .map_err(|_| "supply a numeric unicast IP:PORT (IPv6 in brackets); no DNS".to_owned())?;
    let ip = address.ip().to_canonical();
    if ip.is_multicast()
        || address.port() == 0
        || ip == std::net::IpAddr::V4(std::net::Ipv4Addr::BROADCAST)
    {
        return Err("IPFIX requires a numeric unicast address with a nonzero port".into());
    }
    Ok(address)
}
pub fn collect(options: crate::Collect) -> Result<(), Box<dyn std::error::Error>> {
    let socket = NumericUdp::bind(options.address, None)?;
    let mut collector = Collector::default();
    let mut bytes = [0_u8; 65535];
    let mut count = 0;
    let start = Instant::now();
    let mut output = io::stdout().lock();
    loop {
        let (n, peer) = socket.recv_from(&mut bytes)?;
        match collector.observe(
            &bytes[..n],
            peer.to_string().as_bytes(),
            u64::try_from(start.elapsed().as_millis())?,
        ) {
            Ok((out, diagnostics)) => {
                for event in out.events {
                    output.write_all(&qso_graph_interop::write(
                        &event,
                        &qso_graph_interop::PublicationOptions::default(),
                    )?)?;
                    writeln!(output)?;
                }
                for d in diagnostics {
                    eprintln!(
                        "{peer}: {} {} set={:?} record={:?}: {}",
                        d.rule, d.severity, d.set_index, d.record_index, d.message
                    );
                }
            }
            Err(e) => eprintln!("{peer}: H.1-FRAMING: {e}"),
        }
        output.flush()?;
        count += 1;
        if options.count != 0 && count >= options.count {
            break;
        }
    }
    Ok(())
}
fn event(kind: &str, fields: Value) -> Result<Event, qso_graph_interop::Error> {
    Event::from_value(
        json!({"spec":"0.2.0","family":"pskreporter-ipfix","mapping":1,"kind":kind,"context":{"direction":"from_application","exchange":"notification"},"fields":fields,"native":{"format":"synthetic","data":""}}),
    )
}
pub fn simulate(options: crate::Sim) -> Result<(), Box<dyn std::error::Error>> {
    let target = unicast(&options.to.ok_or("--to is required")?.to_string())?;
    if options.interface.is_some()
        || options.speed != 1.0
        || !options.call.is_empty()
        || !options.grid.is_empty()
        || options.mode.is_some()
        || options.frequency.is_some()
        || options.dx_call.is_some()
        || !options.from.is_empty()
    {
        return Err("IPFIX simulation uses its Q-prefixed source profile; unrelated simulator options are not supported".into());
    }
    let socket = NumericUdp::sender(target, None)?;
    let profile = match options.profile.as_str() {
        "wsjtx" => SubmissionProfile::Wsjtx,
        "fldigi" => SubmissionProfile::Fldigi,
        _ => return Err("profile must be wsjtx or fldigi".into()),
    };
    let mut rx = json!({"receiver_call":"Q0RX","receiver_locator":"AA00aa","reporting_software":if profile==SubmissionProfile::Wsjtx {"WSJT-X v3.0.2 synthetic"} else {"fldigi-4.2.13 synthetic"},"antenna_information":"Synthetic dipole"});
    if profile == SubmissionProfile::Wsjtx {
        rx["rig_information"] = "Synthetic receiver".into();
    } else {
        rx["persistent_identifier"] = "QGI00001".into();
    }
    let receiver = event("receiver_information", rx)?;
    let mut number = 0_u32;
    loop {
        let now = u32::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?;
        let time = utc(now);
        let mut fields = json!({"sender_call":format!("Q{}AAA",number%16+1),"sender_locator":"BB11bb","frequency_hz":14074000,"mode":if profile==SubmissionProfile::Wsjtx {"FT8"} else {"PSK31"},"information_source":1,"time":time});
        if profile == SubmissionProfile::Wsjtx {
            fields["snr_db"] = (-12).into();
        }
        let spot = event("spot", fields)?;
        let mut submission = SubmissionOptions::default();
        submission.profile = profile;
        submission.export_time = now;
        submission.sequence_number = number.wrapping_mul(2);
        submission.observation_domain_id = 0x51474901;
        submission.include_templates = true;
        let bytes = encode_submission(&receiver, &[spot], &submission)?;
        socket.send(&bytes)?;
        number = number.wrapping_add(1);
        if !options.repeat {
            break;
        }
        // A stable source port, startup-relative timer and deterministic jitter avoid clock synchronization.
        std::thread::sleep(Duration::from_secs(301 + u64::from(number % 17)));
    }
    Ok(())
}
fn utc(seconds: u32) -> String {
    let mut days = seconds / 86400;
    let mut year = 1970;
    let leap = |y| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    while days >= if leap(year) { 366 } else { 365 } {
        days -= if leap(year) { 366 } else { 365 };
        year += 1;
    }
    let months = [
        31,
        if leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 0;
    while days >= months[month] {
        days -= months[month];
        month += 1;
    }
    format!(
        "{year:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        month + 1,
        days + 1,
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}
