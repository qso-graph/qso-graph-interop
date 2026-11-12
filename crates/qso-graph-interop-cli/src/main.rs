// SPDX-License-Identifier: GPL-3.0-or-later
//! UDP listener and finite scripted simulator. All destinations are numeric socket addresses.
#![forbid(unsafe_code)]

mod numeric_udp;

use clap::{Args, Parser, Subcommand};
use numeric_udp::NumericUdp;
use qso_graph_interop::wsjtx::{self, Body, Packet, QDateTime, QTime};
use qso_graph_interop::{Codec, codec::DecodeOptions};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser)]
#[command(
    name = "qg-interop",
    version,
    disable_help_subcommand = true,
    about = "Listen to, decode, and simulate amateur radio messages",
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
    /// Decode one complete native WSJT-X datagram from a file or standard input.
    #[command(
        after_help = "Examples:\n  qg-interop decode --wsjtx capture.bin\n  qg-interop decode --wsjtx -"
    )]
    Decode(Decode),
}

#[derive(Args)]
struct Listen {
    /// Select the WSJT-X protocol family.
    #[arg(long, required = true)]
    wsjtx: bool,
    /// Numeric local address to bind (IPv6 uses brackets).
    #[arg(default_value = "127.0.0.1:2237")]
    address: SocketAddr,
    /// Emit specification NDJSON (also the default output).
    #[arg(long)]
    json: bool,
    /// Exit after this many datagrams (0 listens indefinitely).
    #[arg(long, default_value_t = 0)]
    count: u64,
}

#[derive(Args)]
struct Decode {
    #[arg(long, required = true)]
    wsjtx: bool,
    /// File containing one native datagram; '-' reads standard input.
    #[arg(default_value = "-")]
    file: PathBuf,
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
        Command::Decode(options) => decode_file(options),
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
        match emit(&bytes[..n], &mut output) {
            Ok(()) => output.flush()?,
            Err(e) => eprintln!("{peer}: {e}"),
        }
        seen += 1;
        if options.count != 0 && seen >= options.count {
            break;
        }
    }
    Ok(())
}

fn emit(bytes: &[u8], output: &mut impl Write) -> Result<(), Box<dyn std::error::Error>> {
    let outcome = wsjtx::Wsjtx::default().decode(bytes, &DecodeOptions::default())?;
    for event in outcome.events {
        output.write_all(&qso_graph_interop::json::write(
            &event,
            true,
            Some(&wsjtx::Wsjtx::default()),
        )?)?;
        writeln!(output)?;
    }
    for skip in outcome.skips {
        eprintln!(
            "skipped native message: {}",
            serde_json::to_string(&skip.reason)?
        );
    }
    Ok(())
}
fn decode_file(options: Decode) -> Result<(), Box<dyn std::error::Error>> {
    let input: Box<dyn Read> = if options.file.as_os_str() == "-" {
        Box::new(io::stdin().lock())
    } else {
        Box::new(std::fs::File::open(options.file)?)
    };
    let mut bytes = Vec::new();
    input
        .take((wsjtx::MAX_DATAGRAM + 1) as u64)
        .read_to_end(&mut bytes)?;
    emit(&bytes, &mut io::stdout().lock())
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
    let mut status = {
        let mut value = wsjtx::Status::default();
        value.dial_frequency_hz = s.frequency;
        value.mode = s.mode.as_str().into();
        value.dx_call = "".into();
        value.report = "".into();
        value.tx_mode = s.mode.as_str().into();
        value.tx_enabled = false;
        value.transmitting = false;
        value.decoding = true;
        value.rx_df_hz = 1200;
        value.tx_df_hz = 1500;
        value.de_call = Some(s.call.as_str().into());
        value.de_grid = Some(s.grid.as_str().into());
        value.dx_grid = Some("".into());
        value.tx_watchdog = Some(false);
        value.sub_mode = Some("".into());
        value.fast_mode = Some(false);
        value.special_operation_mode = Some(0);
        value.frequency_tolerance_hz = Some(u32::MAX);
        value.tr_period_seconds = Some(15);
        value.configuration_name = Some("Default".into());
        value.tx_message = Some("".into());
        value
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
        make(Body::Heartbeat({
            let mut value = wsjtx::Heartbeat::default();
            value.max_schema = Some(3);
            value.version = Some(env!("CARGO_PKG_VERSION").into());
            value.revision = Some("qg-interop".into());
            value
        })),
        make(Body::Status(status.clone())),
    ];
    let time = QTime::new(((seconds % 86_400) * 1000) as u32);
    for (index, text) in [
        format!("CQ {} {}", s.dx_call, s.grid),
        format!("{} {} -12", s.call, s.dx_call),
        format!("{} {} RR73", s.call, s.dx_call),
    ]
    .into_iter()
    .enumerate()
    {
        messages.push(make(Body::Decode({
            let mut value = wsjtx::Decode::default();
            value.new = true;
            value.time = time;
            value.snr_db = -12 + index as i32;
            value.delta_time_seconds = 0.2;
            value.delta_frequency_hz = 1500;
            value.mode = wsjtx::mode_character(&s.mode).into();
            value.message = text.into();
            value.low_confidence = Some(false);
            value.off_air = Some(false);
            value
        })));
    }
    status.decoding = false;
    messages.push(make(Body::Status(status)));
    let timestamp = |secs: u64| {
        let mut value = QDateTime::default();
        value.julian_day = (secs / 86_400) as i64 + 2_440_588;
        value.time = QTime::new(((secs % 86_400) * 1000) as u32);
        value.time_spec = 1;
        value.offset_seconds = None;
        value.time_zone = None;
        value
    };
    messages.push(make(Body::QsoLogged(Box::new({
        let mut value = wsjtx::QsoLogged::default();
        value.time_off = timestamp(seconds);
        value.dx_call = s.dx_call.as_str().into();
        value.dx_grid = s.grid.as_str().into();
        value.frequency_hz = tx_frequency_hz;
        value.mode = s.mode.as_str().into();
        value.report_sent = "-12".into();
        value.report_received = "-10".into();
        value.tx_power = "5".into();
        value.comments = "Simulated QSO".into();
        value.name = "".into();
        value.time_on = Some(timestamp(seconds.saturating_sub(45)));
        value.operator_call = Some(s.call.as_str().into());
        value.my_call = Some(s.call.as_str().into());
        value.my_grid = Some(s.grid.as_str().into());
        value.exchange_sent = Some("".into());
        value.exchange_received = Some("".into());
        value.propagation_mode = Some("".into());
        value
    }))));
    messages.push(make(Body::Clear({
        let mut value = wsjtx::Clear::default();
        value.window = None;
        value
    })));
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
