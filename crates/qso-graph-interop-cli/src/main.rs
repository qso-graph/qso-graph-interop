// SPDX-License-Identifier: GPL-3.0-or-later
//! UDP listener and finite scripted simulator. All destinations are numeric socket addresses.
#![forbid(unsafe_code)]

mod numeric_udp;
mod output;

use clap::{Args, Parser, Subcommand};
use numeric_udp::NumericUdp;
use qso_graph_interop::wsjtx::{self, Body, Packet, QDateTime, QTime};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(
    name = "qg-interop",
    version,
    disable_help_subcommand = true,
    about = "Listen to and simulate amateur radio UDP messages",
    after_help = "Examples:\n  qg-interop listen --wsjtx\n  qg-interop sim --wsjtx --to 127.0.0.1:2237"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print each decoded datagram as one line.
    #[command(
        after_help = "Examples:\n  qg-interop listen --wsjtx\n  qg-interop listen --wsjtx 127.0.0.1:2237 --json"
    )]
    Listen(Listen),
    /// Send a finite heartbeat, status, decode and logged-QSO script.
    #[command(
        after_help = "Examples:\n  qg-interop sim --wsjtx --to 127.0.0.1:2237\n  qg-interop sim --wsjtx --to 127.0.0.1:2237 --call N0CALL --dx-call W1AW --mode FT4 --frequency 7074000"
    )]
    Sim(Sim),
}

#[derive(Args)]
struct Listen {
    /// Select the WSJT-X protocol family.
    #[arg(long, required = true)]
    wsjtx: bool,
    /// Numeric local address to bind (IPv6 uses brackets).
    #[arg(default_value = "127.0.0.1:2237")]
    address: SocketAddr,
    /// Output one JSON object per datagram.
    #[arg(long)]
    json: bool,
    /// Include sender, schema, and client ID in human output.
    #[arg(long)]
    verbose: bool,
    /// Exit after this many datagrams (0 listens indefinitely).
    #[arg(long, default_value_t = 0)]
    count: u64,
}

#[derive(Args)]
struct Sim {
    #[arg(long, required = true)]
    wsjtx: bool,
    /// Numeric destination; required, with no implicit broadcast or discovery.
    #[arg(long)]
    to: SocketAddr,
    /// Simulated station callsign.
    #[arg(long, default_value = "N0CALL")]
    call: String,
    #[arg(long, default_value = "W1AW")]
    dx_call: String,
    #[arg(long, default_value = "FN31")]
    grid: String,
    #[arg(long, default_value = "FT8")]
    mode: String,
    /// Dial frequency in Hz.
    #[arg(long, default_value_t = 14_074_000)]
    frequency: u64,
    #[arg(long, default_value = "WSJT-X")]
    id: String,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("qg-interop: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Listen(options) => listen(options),
        Command::Sim(options) => simulate(options),
    }
}

fn listen(options: Listen) -> Result<(), Box<dyn std::error::Error>> {
    let socket = NumericUdp::bind(options.address)?;
    let mut bytes = [0_u8; 65_535];
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut seen = 0_u64;
    loop {
        let (n, peer) = socket.recv_from(&mut bytes)?;
        let received_ms = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
        let received_ms = i64::try_from(received_ms)?;
        match wsjtx::decode(&bytes[..n]) {
            Ok(message) => {
                let value = output::object(&message, peer, received_ms);
                if options.json {
                    serde_json::to_writer(&mut output, &value)?;
                    writeln!(output)?;
                } else {
                    writeln!(output, "{}", output::human(&value, options.verbose))?;
                }
                output.flush()?;
            }
            Err(e) => eprintln!("{peer}: {e}"),
        }
        seen += 1;
        if options.count != 0 && seen >= options.count {
            break;
        }
    }
    Ok(())
}

fn simulate(options: Sim) -> Result<(), Box<dyn std::error::Error>> {
    let socket = NumericUdp::sender(options.to)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?;
    for packet in script(&options, now.as_secs())? {
        socket.send(&wsjtx::encode(&packet)?)?;
    }
    Ok(())
}

fn script(s: &Sim, seconds: u64) -> Result<Vec<Packet>, io::Error> {
    let make = |body| Packet::new(s.id.as_str(), body);
    let mut status = wsjtx::Status {
        dial_frequency_hz: s.frequency,
        mode: s.mode.as_str().into(),
        dx_call: "".into(),
        report: "".into(),
        tx_mode: s.mode.as_str().into(),
        tx_enabled: false,
        transmitting: false,
        decoding: true,
        rx_df_hz: 1200,
        tx_df_hz: 1500,
        de_call: Some(s.call.as_str().into()),
        de_grid: Some(s.grid.as_str().into()),
        dx_grid: Some("".into()),
        tx_watchdog: Some(false),
        sub_mode: Some("".into()),
        fast_mode: Some(false),
        special_operation_mode: Some(0),
        frequency_tolerance_hz: Some(u32::MAX),
        tr_period_seconds: Some(15),
        configuration_name: Some("Default".into()),
        tx_message: Some("".into()),
    };
    let tx_frequency_hz = s
        .frequency
        .checked_add(u64::from(status.tx_df_hz))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "dial frequency plus TX offset overflows",
            )
        })?;
    let mut messages = vec![
        make(Body::Heartbeat(wsjtx::Heartbeat {
            max_schema: Some(3),
            version: Some(env!("CARGO_PKG_VERSION").into()),
            revision: Some("qg-interop".into()),
        })),
        make(Body::Status(status.clone())),
    ];
    let time = QTime(((seconds % 86_400) * 1000) as u32);
    for (index, text) in [
        format!("CQ {} {}", s.dx_call, s.grid),
        format!("{} {} -12", s.call, s.dx_call),
        format!("{} {} RR73", s.call, s.dx_call),
    ]
    .into_iter()
    .enumerate()
    {
        messages.push(make(Body::Decode(wsjtx::Decode {
            new: true,
            time,
            snr_db: -12 + index as i32,
            delta_time_seconds: 0.2,
            delta_frequency_hz: 1500,
            mode: wsjtx::mode_character(&s.mode).into(),
            message: text.into(),
            low_confidence: Some(false),
            off_air: Some(false),
        })));
    }
    status.decoding = false;
    messages.push(make(Body::Status(status)));
    let timestamp = |secs: u64| QDateTime {
        julian_day: (secs / 86_400) as i64 + 2_440_588,
        time: QTime(((secs % 86_400) * 1000) as u32),
        time_spec: 1,
        offset_seconds: None,
        time_zone: None,
    };
    messages.push(make(Body::QsoLogged(Box::new(wsjtx::QsoLogged {
        time_off: timestamp(seconds),
        dx_call: s.dx_call.as_str().into(),
        dx_grid: s.grid.as_str().into(),
        frequency_hz: tx_frequency_hz,
        mode: s.mode.as_str().into(),
        report_sent: "-12".into(),
        report_received: "-10".into(),
        tx_power: "5".into(),
        comments: "Simulated QSO".into(),
        name: "".into(),
        time_on: Some(timestamp(seconds.saturating_sub(45))),
        operator_call: Some(s.call.as_str().into()),
        my_call: Some(s.call.as_str().into()),
        my_grid: Some(s.grid.as_str().into()),
        exchange_sent: Some("".into()),
        exchange_received: Some("".into()),
        propagation_mode: Some("".into()),
    }))));
    messages.push(make(Body::Clear(wsjtx::Clear { window: None })));
    messages.push(make(Body::Close));
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    use qso_graph_interop::wsjtx::Utf8;

    #[test]
    fn help_examples_and_no_color() {
        let mut command = Cli::command();
        command.debug_assert();
        command = Cli::command();
        let help = command.render_long_help().to_string();
        assert!(help.contains("Examples:"));
        assert!(!help.contains('\u{1b}'));
        for sub in command.get_subcommands_mut() {
            assert!(sub.render_long_help().to_string().contains("Examples:"));
        }
    }

    #[test]
    fn addresses_are_numeric_and_sim_destination_is_required() {
        assert!(Cli::try_parse_from(["qg-interop", "sim", "--wsjtx"]).is_err());
        assert!(
            Cli::try_parse_from(["qg-interop", "sim", "--wsjtx", "--to", "example.com:2237"])
                .is_err()
        );
        assert!(Cli::try_parse_from(["qg-interop", "listen"]).is_err());
        let c = Cli::try_parse_from(["qg-interop", "listen", "--wsjtx"]).unwrap();
        let Command::Listen(l) = c.command else {
            panic!("listen");
        };
        assert_eq!(l.address.to_string(), "127.0.0.1:2237");
    }

    #[test]
    fn simulator_output_has_expected_human_lines_and_json_types() {
        let cli = Cli::try_parse_from(["qg-interop", "sim", "--wsjtx", "--to", "127.0.0.1:2237"])
            .unwrap();
        let Command::Sim(sim) = cli.command else {
            panic!("sim")
        };
        let expected = [
            "Heartbeat",
            "Status",
            "Decode",
            "Decode",
            "Decode",
            "Status",
            "QSO Logged",
            "Clear",
            "Close",
        ];
        for (packet, label) in script(&sim, 0).unwrap().into_iter().zip(expected) {
            let value = output::object(
                &wsjtx::Outcome::Message(Box::new(packet)),
                "127.0.0.1:1234".parse().unwrap(),
                0,
            );
            let line = output::human(&value, false);
            assert!(line.starts_with(&format!("00:00:00  {label}")), "{line}");
            assert!(!line.contains("Utf8"));
            let json: serde_json::Value = serde_json::from_str(&value.to_string()).unwrap();
            assert!(json["schema"].is_u64());
            assert!(json["client_id"].is_string());
            if label == "Decode" {
                assert!(json["snr_db"].is_i64());
                assert!(json["new"].is_boolean());
                assert_eq!(json["time"], "00:00:00.000");
                assert!(line.contains("CQ W1AW FN31") || line.contains("N0CALL W1AW"));
            }
        }
    }

    #[test]
    fn simulator_tx_frequency_matches_status_and_overflow_is_rejected() {
        for mode in ["FT8", "FT4"] {
            let cli = Cli::try_parse_from([
                "qg-interop",
                "sim",
                "--wsjtx",
                "--to",
                "127.0.0.1:2237",
                "--mode",
                mode,
            ])
            .unwrap();
            let Command::Sim(mut sim) = cli.command else {
                panic!("sim");
            };
            let packets = script(&sim, 0).unwrap();
            let Body::Status(status) = &packets[1].body else {
                panic!("status");
            };
            let Body::QsoLogged(qso) = &packets[6].body else {
                panic!("QSO");
            };
            assert_eq!(
                qso.frequency_hz,
                status.dial_frequency_hz + u64::from(status.tx_df_hz)
            );
            assert_eq!(qso.frequency_hz, 14_075_500);
            sim.frequency = u64::MAX - 1500;
            assert!(script(&sim, 0).is_ok());
            sim.frequency += 1;
            assert!(script(&sim, 0).is_err());
        }
    }

    #[test]
    fn script_respects_options_and_codec() {
        let c = Cli::try_parse_from([
            "qg-interop",
            "sim",
            "--wsjtx",
            "--to",
            "127.0.0.1:2237",
            "--mode",
            "FT4",
            "--frequency",
            "7074000",
            "--call",
            "K7ABC",
        ])
        .unwrap();
        let Command::Sim(s) = c.command else {
            panic!("sim");
        };
        let packets = script(&s, 100_000).unwrap();
        assert_eq!(packets.len(), 9);
        for p in packets {
            assert_eq!(
                wsjtx::decode(&wsjtx::encode(&p).unwrap()).unwrap(),
                wsjtx::Outcome::Message(Box::new(p.clone()))
            );
            match p.body {
                Body::Status(v) => {
                    assert_eq!(v.dial_frequency_hz, 7_074_000);
                    assert_eq!(v.de_call, Some(Utf8::from("K7ABC")));
                }
                Body::Decode(v) => assert_eq!(v.mode, Utf8::from("+")),
                Body::QsoLogged(v) => assert_eq!(v.mode, Utf8::from("FT4")),
                _ => {}
            }
        }
    }
}
