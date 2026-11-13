// SPDX-License-Identifier: GPL-3.0-or-later
//! UDP listener and timed simulator. All destinations are numeric socket addresses.
#![forbid(unsafe_code)]

use qso_graph_interop_cli::alltxt;
mod capture;
mod fldigi_poll;
mod fldigi_sim;
mod loopback_tcp;
mod n1mm_sim;
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
        after_help = "Examples:\n  qg-interop listen --wsjtx\n  qg-interop listen --n1mm\n  qg-interop listen --n1mm 127.0.0.1:12060 --json"
    )]
    Listen(Listen),
    /// Simulate UDP traffic or serve fldigi XML-RPC on loopback.
    #[command(
        after_help = "Examples:\n  qg-interop sim --fldigi\n  qg-interop sim --wsjtx --to 127.0.0.1:2237\n  qg-interop sim --n1mm --to 127.0.0.1:12060 --radios 2 --speed 60\n  qg-interop sim --wsjtx --to 127.0.0.1:2237 --call N0CALL --dx-call Q1AW --mode FT4 --frequency 7074000"
    )]
    Sim(Box<Sim>),
    /// Decode one complete native body from a file or standard input.
    #[command(
        after_help = "Examples:\n  qg-interop decode --wsjtx capture.bin\n  qg-interop decode --n1mm capture.xml\n  qg-interop decode --n1mm -\n  qg-interop decode --fldigi response.xml --request-method rig.get_frequency"
    )]
    Decode(Decode),
    /// Poll fldigi XML-RPC state and RX text on numeric loopback only.
    #[command(
        after_help = "Examples:\n  qg-interop poll --fldigi\n  qg-interop poll --fldigi 127.0.0.1:7362 --interval 500 --count 3"
    )]
    Poll(Poll),
}

#[derive(Args)]
struct Listen {
    /// Select the WSJT-X protocol family.
    #[arg(long, required_unless_present = "n1mm", conflicts_with = "n1mm")]
    wsjtx: bool,
    /// Select N1MM Logger+ broadcasts (default port 12060).
    #[arg(long)]
    n1mm: bool,
    /// Numeric local address to bind (IPv6 uses brackets).
    address: Option<SocketAddr>,
    /// Emit specification NDJSON (also the default output).
    #[arg(long)]
    json: bool,
    /// Exit after this many datagrams (0 listens indefinitely).
    #[arg(long, default_value_t = 0)]
    count: u64,
}

#[derive(Args)]
struct Decode {
    #[arg(long, required_unless_present_any = ["n1mm", "fldigi"], conflicts_with_all = ["n1mm", "fldigi"])]
    wsjtx: bool,
    /// Select the N1MM Logger+ protocol family.
    #[arg(long)]
    n1mm: bool,
    /// Select fldigi XML-RPC bodies.
    #[arg(long, conflicts_with = "n1mm")]
    fldigi: bool,
    /// Associated method, required when decoding an XML-RPC response.
    #[arg(long, requires = "fldigi", conflicts_with = "request_file")]
    request_method: Option<String>,
    /// Original XML-RPC request body (also supplies text.get_rx range arguments).
    #[arg(long, requires = "fldigi")]
    request_file: Option<PathBuf>,
    /// File containing one native body; '-' reads standard input.
    #[arg(default_value = "-")]
    file: PathBuf,
}

#[derive(Args)]
struct Poll {
    #[arg(long, required = true)]
    fldigi: bool,
    /// Numeric loopback address; fldigi's default XML-RPC port is 7362.
    #[arg(default_value = "127.0.0.1:7362", value_parser = loopback_address)]
    address: SocketAddr,
    /// Delay between complete polling cycles, in milliseconds.
    #[arg(long, default_value = "1000", value_parser = clap::value_parser!(u64).range(1..=86_400_000))]
    interval: u64,
    /// Exit after N cycles (0 polls indefinitely).
    #[arg(long, default_value_t = 0)]
    count: u64,
}
fn loopback_address(s: &str) -> Result<SocketAddr, String> {
    let address: SocketAddr = s.parse().map_err(|_| {
        "fldigi XML-RPC uses plain HTTP; supply a numeric loopback address, without DNS".to_owned()
    })?;
    if !address.ip().is_loopback() || address.port() == 0 {
        return Err("fldigi XML-RPC uses plain HTTP; only loopback addresses with a nonzero port are allowed".into());
    }
    Ok(address)
}
#[derive(Args)]
struct Sim {
    #[arg(long, required_unless_present_any = ["capture", "n1mm", "fldigi"], conflicts_with_all = ["qgi", "n1mm", "fldigi"])]
    wsjtx: bool,
    /// Serve fldigi 4.2.13 XML-RPC (default 127.0.0.1:7362).
    #[arg(long, conflicts_with_all = ["n1mm", "capture", "qgi", "to", "file", "from", "id", "dx_call", "mode", "frequency", "repeat", "count", "station", "radios", "rate"])]
    fldigi: bool,
    /// Numeric loopback listener address for --fldigi; no DNS.
    #[arg(requires = "fldigi", value_parser = loopback_address)]
    address: Option<SocketAddr>,
    /// Simulate N1MM Logger+ contest broadcasts and QSY input.
    #[arg(long, conflicts_with_all = ["capture", "qgi", "file", "from", "grid", "id", "dx_call", "mode", "frequency"])]
    n1mm: bool,
    /// Synthetic N1MM computer label (1–64 printable ASCII characters).
    #[arg(long, requires = "n1mm", conflicts_with_all = ["wsjtx", "capture"], default_value = "SYNTHETIC-01", value_parser = n1mm_sim::station)]
    station: String,
    /// N1MM radios: 1, or 2 for SO2R.
    #[arg(long, requires = "n1mm", conflicts_with_all = ["wsjtx", "capture"], default_value = "1", value_parser = clap::value_parser!(u8).range(1..=2))]
    radios: u8,
    /// New N1MM contacts per simulated hour (1–3600).
    #[arg(long, requires = "n1mm", conflicts_with_all = ["wsjtx", "capture"], default_value = "120", value_parser = clap::value_parser!(u32).range(1..=3600))]
    rate: u32,
    /// Deterministic N1MM or fldigi traffic seed.
    #[arg(long, conflicts_with_all = ["wsjtx", "capture"], default_value = "1")]
    seed: u64,
    /// Stop after N new N1MM contacts (1–100000), including with --loop.
    #[arg(long, requires = "n1mm", conflicts_with_all = ["wsjtx", "capture"], value_parser = clap::value_parser!(u64).range(1..=100_000))]
    count: Option<u64>,
    /// Play native.data from an NDJSON capture.
    #[arg(long, conflicts_with_all = ["file", "from", "dx_call", "mode", "frequency"])]
    capture: Option<PathBuf>,
    /// Send each captured event as qgi UDP event carriage.
    #[arg(long, requires = "capture")]
    qgi: bool,
    /// Numeric destination; no implicit broadcast or discovery.
    #[arg(long, required_unless_present = "fldigi", value_parser = sim::target)]
    to: Option<SocketAddr>,
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
        Command::Poll(options) => fldigi_poll::run(options),
    }
}

fn listen(options: Listen) -> Result<(), Box<dyn std::error::Error>> {
    let socket = NumericUdp::bind(listen_address(&options))?;
    let mut codec = selected_codec(options.n1mm);
    let mut bytes = [0_u8; 65_535];
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut seen = 0_u64;
    loop {
        let (n, peer) = socket.recv_from(&mut bytes)?;
        // The observer on the selected application's broadcast/server port supplies direction.
        match emit(codec.as_mut(), &bytes[..n], &mut output, "from_application") {
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
    codec: &mut dyn Codec,
    bytes: &[u8],
    output: &mut impl Write,
    direction: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = DecodeOptions::new(qso_graph_interop::Context::from_value(
        serde_json::json!({"direction": direction, "exchange": "unknown"}),
    )?);
    let outcome = codec.decode(bytes, &options)?;
    for event in outcome.events {
        output.write_all(&qso_graph_interop::json::write(
            &event,
            &qso_graph_interop::PublicationOptions::new(
                qso_graph_interop::PublicationMode::Authored,
            )
            .with_profile(codec),
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
    let mut codec: Box<dyn Codec> = if options.fldigi {
        Box::new(qso_graph_interop::fldigi::Fldigi::default())
    } else {
        selected_codec(options.n1mm)
    };
    let mut decode = DecodeOptions::default();
    decode = decode.with_structured(true);
    if let Some(method) = options.request_method {
        let metadata = match qso_graph_interop::fldigi::RequestAssociation::for_method(&method) {
            Ok(association) => association.into(),
            // API-2: malformed boundary metadata must retain native-parser error precedence.
            Err(_) => DecodeOptions::from_value(
                &serde_json::json!({"native":{"request":{"method":method}}}),
            )?
            .metadata(),
        };
        decode = decode.with_metadata(metadata);
    }
    if let Some(file) = options.request_file {
        let mut request = Vec::new();
        std::fs::File::open(file)?
            .take((qso_graph_interop::fldigi::MAX_BODY + 1) as u64)
            .read_to_end(&mut request)?;
        let request_view =
            codec.decode(&request, &DecodeOptions::default().with_structured(true))?;
        if request_view.events[0].value()["native"]["message"]["root"] != "methodCall" {
            return Err("--request-file must contain a request".into());
        }
        decode = decode.with_metadata(
            qso_graph_interop::fldigi::RequestAssociation::from_request_body(&request)?.into(),
        );
    }
    let input: Box<dyn Read> = if options.file.as_os_str() == "-" {
        Box::new(io::stdin().lock())
    } else {
        Box::new(std::fs::File::open(options.file)?)
    };
    let mut bytes = Vec::new();
    input
        .take((codec.limits().message_bytes + 1) as u64)
        .read_to_end(&mut bytes)?;
    if !options.fldigi {
        return emit(codec.as_mut(), &bytes, &mut io::stdout().lock(), "unknown");
    }
    let out = codec.decode(&bytes, &decode)?;
    if !decode.metadata().extensions().contains_key("request")
        && out.events[0].value()["native"]["message"]["root"] == "methodResponse"
    {
        return Err("fldigi responses require --request-method NAME or --request-file FILE".into());
    }
    for event in out.events {
        io::stdout()
            .lock()
            .write_all(&qso_graph_interop::json::write(
                &event,
                &qso_graph_interop::PublicationOptions::new(
                    qso_graph_interop::PublicationMode::Preserved,
                ),
            )?)?;
        println!();
    }
    Ok(())
}
fn selected_codec(n1mm: bool) -> Box<dyn Codec> {
    if n1mm {
        Box::new(qso_graph_interop::n1mm::N1mm::default())
    } else {
        Box::new(wsjtx::Wsjtx::default())
    }
}
fn listen_address(options: &Listen) -> SocketAddr {
    options.address.unwrap_or_else(|| {
        SocketAddr::from(([127, 0, 0, 1], if options.n1mm { 12060 } else { 2237 }))
    })
}

fn script(s: &Sim, seconds: u64) -> Result<Vec<Packet>, io::Error> {
    let make = |body| Packet::new(s.id.as_str(), body);
    let mode = s.mode.as_deref().unwrap_or("FT8");
    let dx_call = s.dx_call.as_deref().unwrap_or("Q1AW");
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
        assert_eq!(listen_address(&l).to_string(), "127.0.0.1:2237");
        let c = Cli::try_parse_from(["qg-interop", "listen", "--n1mm"]).unwrap();
        let Command::Listen(l) = c.command else {
            panic!("listen")
        };
        assert_eq!(listen_address(&l).to_string(), "127.0.0.1:12060");
        for args in [
            vec!["listen", "--n1mm", "example.com:12060"],
            vec!["listen", "--n1mm", "--wsjtx"],
            vec!["decode", "--n1mm", "--wsjtx"],
            vec!["decode"],
        ] {
            let mut argv = vec!["qg-interop"];
            argv.extend(args);
            assert!(Cli::try_parse_from(argv).is_err());
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
            "Q7ABC",
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
                    assert_eq!(v.de_call, Some(Utf8::from("Q7ABC")));
                }
                Body::Decode(v) => assert_eq!(v.mode, Utf8::from("+")),
                Body::QsoLogged(v) => assert_eq!(v.mode, Utf8::from("FT4")),
                _ => {}
            }
        }
    }
}
