// SPDX-License-Identifier: GPL-3.0-or-later
//! UDP listener and timed simulator. All destinations are numeric socket addresses.
#![forbid(unsafe_code)]

use qso_graph_interop_cli::alltxt;
mod capture;
mod numeric_udp;
mod sim;

use clap::{Args, Parser, Subcommand};
use numeric_udp::NumericUdp;
use qso_graph_interop::wsjtx::{self, Body, Packet, QDateTime, QTime};
use qso_graph_interop::{Codec, codec::DecodeOptions};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;

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
    /// Play ALL.TXT or native NDJSON captures over UDP.
    #[command(
        after_help = "Examples:\n  qg-interop sim --wsjtx --to 127.0.0.1:2237\n  qg-interop sim --wsjtx --to 127.0.0.1:2237 --call N0CALL --dx-call W1AW --mode FT4 --frequency 7074000"
    )]
    Sim(Box<Sim>),
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
    #[arg(long, required_unless_present = "capture", conflicts_with = "qgi")]
    wsjtx: bool,
    /// Play native.data from an NDJSON capture.
    #[arg(long, conflicts_with_all = ["file", "from", "dx_call", "mode", "frequency"])]
    capture: Option<PathBuf>,
    /// Send each captured event as qgi UDP event carriage.
    #[arg(long, requires = "capture")]
    qgi: bool,
    /// Numeric destination; no implicit broadcast or discovery.
    #[arg(long, value_parser = sim::target)]
    to: SocketAddr,
    /// ALL.TXT to play; the built-in synthetic sample is the default.
    #[arg(long, conflicts_with_all = ["dx_call", "mode", "frequency"])]
    file: Option<PathBuf>,
    /// Start at this timestamp in the log.
    #[arg(long, default_value = "", value_parser = sim::start)]
    from: String,
    #[arg(long, default_value = "", value_parser = sim::call)]
    call: String,
    #[arg(long, default_value = "", value_parser = sim::grid)]
    grid: String,
    #[arg(long, default_value = "WSJT-X", value_parser = sim::id)]
    id: String,
    #[arg(long, default_value = "1", value_parser = sim::speed)]
    speed: f64,
    #[arg(long = "loop")]
    repeat: bool,
    /// Select the legacy fixed QSO script, with this DX callsign.
    #[arg(long, value_parser = sim::call)]
    dx_call: Option<String>,
    /// Select the legacy fixed QSO script, with this mode.
    #[arg(long)]
    mode: Option<String>,
    /// Select the legacy fixed QSO script, with this dial frequency in Hz.
    #[arg(long)]
    frequency: Option<u64>,
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
        Command::Sim(options) => sim::run(*options),
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
        // A WSJT-X server port receives only what WSJT-X instances send, so the listener, as the
        // observer, knows the direction (QGI-MOD-0002). The codec still infers nothing.
        match emit(&bytes[..n], &mut output, "from_application") {
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

fn emit(
    bytes: &[u8],
    output: &mut impl Write,
    direction: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut options = DecodeOptions::default();
    options.context = serde_json::json!({"direction": direction, "exchange": "unknown"});
    let outcome = wsjtx::Wsjtx::default().decode(bytes, &options)?;
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
    emit(&bytes, &mut io::stdout().lock(), "unknown")
}

fn script(s: &Sim, seconds: u64) -> Result<Vec<Packet>, io::Error> {
    let make = |body| Packet::new(s.id.as_str(), body);
    let mode = s.mode.as_deref().unwrap_or("FT8");
    let dx_call = s.dx_call.as_deref().unwrap_or("W1AW");
    let frequency = s.frequency.unwrap_or(14_074_000);
    let call = if s.call.is_empty() { "N0CALL" } else { &s.call };
    let grid = if s.grid.is_empty() { "FN31" } else { &s.grid };
    let mut status = {
        let mut value = wsjtx::Status::default();
        value.dial_frequency_hz = frequency;
        value.mode = mode.into();
        value.dx_call = "".into();
        value.report = "".into();
        value.tx_mode = mode.into();
        value.tx_enabled = false;
        value.transmitting = false;
        value.decoding = true;
        value.rx_df_hz = 1200;
        value.tx_df_hz = 1500;
        value.de_call = Some(call.into());
        value.de_grid = Some(grid.into());
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
    let tx_frequency_hz = frequency
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
        format!("CQ {} {}", dx_call, grid),
        format!("{} {} -12", call, dx_call),
        format!("{} {} RR73", call, dx_call),
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
            value.mode = wsjtx::mode_character(mode).into();
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
        value.dx_call = dx_call.into();
        value.dx_grid = grid.into();
        value.frequency_hz = tx_frequency_hz;
        value.mode = mode.into();
        value.report_sent = "-12".into();
        value.report_received = "-10".into();
        value.tx_power = "5".into();
        value.comments = "Simulated QSO".into();
        value.name = "".into();
        value.time_on = Some(timestamp(seconds.saturating_sub(45)));
        value.operator_call = Some(call.into());
        value.my_call = Some(call.into());
        value.my_grid = Some(grid.into());
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
            sim.frequency = Some(u64::MAX - 1500);
            assert!(script(&sim, 0).is_ok());
            sim.frequency = Some(u64::MAX - 1499);
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
