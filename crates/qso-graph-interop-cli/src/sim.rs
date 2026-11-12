// SPDX-License-Identifier: GPL-3.0-or-later
//! UDP simulator policy and scheduling; the codec remains sans I/O.
use crate::{
    Sim,
    alltxt::{self, Cycle, invalid},
    numeric_udp::NumericUdp,
};
use qso_graph_interop::wsjtx::{self, Body, Packet, QTime};
use std::io::{self, Read};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

static STOP: OnceLock<Arc<AtomicBool>> = OnceLock::new();
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn target(s: &str) -> std::result::Result<SocketAddr, String> {
    let parsed = s.parse().or_else(|_| {
        let (ip, port) = s.rsplit_once(':').ok_or(())?;
        if !ip.contains(':') {
            return Err(());
        }
        format!("[{ip}]:{port}").parse().map_err(|_| ())
    });
    let a: SocketAddr =
        parsed.map_err(|_| "use IP:PORT, the address QSO Graph Desktop listens on".to_owned())?;
    if a.port() == 0 {
        return Err("0 is not a port".into());
    }
    Ok(a)
}
pub fn call(s: &str) -> std::result::Result<String, String> {
    let v = s.to_uppercase();
    if v.len() > 13
        || !v
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'/')
    {
        return Err(format!("{s:?} is not a callsign"));
    }
    Ok(v)
}
pub fn grid(s: &str) -> std::result::Result<String, String> {
    let b = s.as_bytes();
    let valid = s.is_empty()
        || matches!(b.len(), 4 | 6)
            && b[..2].iter().all(|b| (b'A'..=b'R').contains(b))
            && b[2..4].iter().all(u8::is_ascii_digit)
            && b[4..].iter().all(|b| (b'A'..=b'X').contains(b));
    if !valid {
        return Err(format!("{s:?} is not a 4- or 6-character grid"));
    }
    Ok(s.into())
}
pub fn id(s: &str) -> std::result::Result<String, String> {
    if !(1..=64).contains(&s.chars().count()) || s.contains('\n') || s.len() > 256 {
        return Err(format!("{s:?} is not an Id"));
    }
    Ok(s.into())
}
pub fn start(s: &str) -> std::result::Result<String, String> {
    if !s.is_empty()
        && (s.len() != 13
            || s.as_bytes()[6] != b'_'
            || !s
                .bytes()
                .enumerate()
                .all(|(i, b)| i == 6 || b.is_ascii_digit()))
    {
        return Err(format!("{s:?} is not YYMMDD_HHMMSS"));
    }
    Ok(s.into())
}
pub fn speed(s: &str) -> std::result::Result<f64, String> {
    let n: f64 = s.parse().map_err(|_| format!("{s:?} is not a number"))?;
    if !(0.0 < n && n <= 1000.0) {
        return Err("speed is more than 0 and at most 1000".into());
    }
    Ok(n)
}
pub fn read(path: &std::path::PathBuf) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(alltxt::MAX_FILE as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > alltxt::MAX_FILE {
        return Err(invalid("input exceeds 64 MiB"));
    }
    Ok(bytes)
}
pub fn delay(ms: u64, speed: f64) -> io::Result<Duration> {
    Duration::try_from_secs_f64(ms as f64 / 1000.0 / speed)
        .map_err(|_| invalid("speed makes playback duration too large"))
}
fn deadline(start: Instant, ms: u64, speed: f64) -> io::Result<Instant> {
    start
        .checked_add(delay(ms, speed)?)
        .ok_or_else(|| invalid("playback deadline is too large"))
}
pub fn stopped() -> bool {
    STOP.get().is_some_and(|s| s.load(Ordering::Relaxed))
}
pub fn install_stop() -> Result<()> {
    let stop = STOP.get_or_init(|| Arc::new(AtomicBool::new(false)));
    signal_hook::flag::register(signal_hook::SIGINT, Arc::clone(stop))?;
    signal_hook::flag::register(signal_hook::SIGTERM, Arc::clone(stop))?;
    Ok(())
}

pub struct Player {
    pub status: wsjtx::Status,
    pub window: Vec<wsjtx::Decode>,
    pub id: String,
    schema: u32,
    staged: String,
    pub close: bool,
    window_bytes: usize,
}
impl Player {
    pub fn new(s: &Sim) -> Self {
        let mut status = wsjtx::Status::default();
        status.mode = "FT8".into();
        status.tx_mode = "FT8".into();
        status.dx_call = "".into();
        status.report = "".into();
        status.rx_df_hz = 1500;
        status.tx_df_hz = 1500;
        status.de_call = Some(s.call.as_str().into());
        status.de_grid = Some(s.grid.as_str().into());
        status.dx_grid = Some("".into());
        status.tx_watchdog = Some(false);
        status.sub_mode = Some("".into());
        status.fast_mode = Some(false);
        status.special_operation_mode = Some(0);
        status.frequency_tolerance_hz = Some(u32::MAX);
        status.tr_period_seconds = Some(15);
        status.configuration_name = Some("Default".into());
        status.tx_message = Some("".into());
        Self {
            status,
            window: Vec::new(),
            id: s.id.clone(),
            schema: 3,
            staged: String::new(),
            close: false,
            window_bytes: 0,
        }
    }
    fn packet(&self, body: Body) -> Packet {
        let mut p = Packet::new(self.id.as_str(), body);
        p.schema = self.schema;
        p
    }
    fn heartbeat(&self) -> Packet {
        let mut h = wsjtx::Heartbeat::default();
        h.max_schema = Some(3);
        h.version = Some("qg-interop".into());
        h.revision = Some(env!("CARGO_PKG_VERSION").into());
        self.packet(Body::Heartbeat(h))
    }
    pub fn play(&mut self, c: &Cycle, ms: u64) -> io::Result<Vec<Packet>> {
        let added: usize = c.decodes.iter().map(|d| d.message.len() + 128).sum();
        if self.window_bytes + added > alltxt::MAX_FILE {
            return Err(invalid("Replay window exceeds 64 MiB"));
        }
        if self.window.len() + c.decodes.len() > alltxt::MAX_DECODES {
            return Err(invalid("Replay window exceeds 100000 decodes"));
        }
        self.window_bytes += added;
        self.status.dial_frequency_hz = c.dial_hz;
        let (mode, submode) = alltxt::mode_fields(&c.mode);
        self.status.mode = mode.into();
        self.status.tx_mode = mode.into();
        self.status.sub_mode = Some(submode.into());
        self.status.decoding = true;
        let period = alltxt::period_ms(&c.mode)?;
        // FT4's 7.5 s is not representable in the header's integer period.
        self.status.tr_period_seconds = Some(if period.is_multiple_of(1000) {
            (period / 1000) as u32
        } else {
            u32::MAX
        });
        let mut packets = vec![
            self.heartbeat(),
            self.packet(Body::Status(self.status.clone())),
        ];
        for l in &c.decodes {
            let mut d = wsjtx::Decode::default();
            d.new = true;
            d.time = QTime::new((ms % 86_400_000) as u32);
            d.snr_db = l.snr;
            d.delta_time_seconds = l.dt;
            d.delta_frequency_hz = l.df;
            d.mode = wsjtx::mode_character(alltxt::mode_fields(&l.mode).0).into();
            d.message = l.message.as_str().into();
            d.low_confidence = Some(l.low_confidence);
            d.off_air = Some(false);
            packets.push(self.packet(Body::Decode(d.clone())));
            self.window.push(d);
        }
        self.status.decoding = false;
        packets.push(self.packet(Body::Status(self.status.clone())));
        Ok(packets)
    }
    /// The connected numeric UDP socket admits only the complete configured peer address.
    /// Header v3.0.2 lines 329–341 (Replay), 166–189 (changed Status),
    /// 344–381 (HaltTx/FreeText), 488–508 (Configure), 235–253 (Clear).
    pub fn react(&mut self, bytes: &[u8]) -> Vec<Packet> {
        let Ok(wsjtx::Outcome::Message(p)) = wsjtx::decode(bytes) else {
            return vec![];
        };
        if p.id.as_str() != Some(&self.id) {
            return vec![];
        }
        let before = self.status.clone();
        match p.body {
            Body::Replay => {
                let mut out: Vec<_> = self
                    .window
                    .iter()
                    .map(|d| {
                        let mut d = d.clone();
                        d.new = false;
                        self.packet(Body::Decode(d))
                    })
                    .collect();
                out.push(self.packet(Body::Status(self.status.clone())));
                return out;
            }
            Body::Clear(c) => {
                if matches!(c.window, None | Some(0 | 2)) {
                    self.window.clear();
                    self.window_bytes = 0;
                }
            }
            Body::Close => self.close = true,
            Body::Heartbeat(h) => {
                self.schema = p.schema.min(h.max_schema.unwrap_or(2)).clamp(1, 3);
                return vec![];
            }
            Body::HaltTx(h) => {
                self.status.tx_enabled = false;
                if !h.auto_tx_only {
                    self.status.transmitting = false;
                }
            }
            Body::FreeText(t) => {
                if let Some(text) = t.text.as_str() {
                    if !text.is_empty() || !t.send {
                        self.staged = text.into();
                    }
                    // Staging alone does not change the currently transmitted message.
                    if t.send && self.status.transmitting {
                        self.status.tx_message = Some(self.staged.as_str().into());
                    }
                    self.status.tx_watchdog = Some(false);
                }
            }
            Body::Configure(c) => {
                if let Some(mode) = c
                    .mode
                    .as_str()
                    .filter(|m| matches!(*m, "FT8" | "FT4" | "JT4" | "JT65"))
                {
                    self.status.mode = mode.into();
                    self.status.tx_mode = mode.into();
                    self.status.sub_mode = Some("".into());
                    let period = alltxt::period_ms(mode).unwrap_or(15000);
                    self.status.tr_period_seconds = Some(if period.is_multiple_of(1000) {
                        (period / 1000) as u32
                    } else {
                        u32::MAX
                    });
                }
                // Fixed-period modes do not expose the period, fast-mode, submode or tolerance controls.
                if c.rx_df_hz != u32::MAX && c.rx_df_hz <= 5000 {
                    self.status.rx_df_hz = c.rx_df_hz;
                }
                if let Some(call) = c.dx_call.as_str().filter(|s| !s.is_empty()) {
                    self.status.dx_call = call.into();
                }
                if let Some(grid) = c.dx_grid.as_str().filter(|s| !s.is_empty()) {
                    self.status.dx_grid = Some(grid.into());
                }
                self.status.tx_watchdog = Some(false);
            }
            Body::Location(l) => {
                if let Some(g) = l
                    .location
                    .as_str()
                    .filter(|g| !g.is_empty() && grid(g).is_ok())
                {
                    self.status.de_grid = Some(g.into());
                }
            }
            _ => {}
        }
        if before != self.status {
            vec![self.packet(Body::Status(self.status.clone()))]
        } else {
            vec![]
        }
    }
}
fn send(socket: &NumericUdp, packets: Vec<Packet>) -> Result<()> {
    for p in packets {
        socket.send(&wsjtx::encode(&p)?)?;
    }
    Ok(())
}
fn wait(socket: &NumericUdp, until: Instant, mut player: Option<&mut Player>) -> Result<()> {
    let mut bytes = [0; 65_535];
    while !stopped() && !player.as_ref().is_some_and(|p| p.close) {
        let remaining = until.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        socket.set_read_timeout(Some(
            remaining
                .min(Duration::from_millis(50))
                .max(Duration::from_millis(1)),
        ))?;
        match socket.recv_from(&mut bytes) {
            Ok((n, _)) => {
                if let Some(p) = player.as_deref_mut() {
                    send(socket, p.react(&bytes[..n]))?;
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
pub fn wait_capture(socket: &NumericUdp, until: Instant) -> Result<()> {
    wait(socket, until, None)
}

pub fn run(s: Sim) -> Result<()> {
    if s.n1mm {
        return crate::n1mm_sim::run(&s);
    }
    if s.capture.is_some() {
        return crate::capture::run(&s);
    }
    if s.dx_call.is_some() || s.mode.is_some() || s.frequency.is_some() {
        let seconds = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let packets = crate::script(&s, seconds)?;
        install_stop()?;
        let socket = NumericUdp::sender(s.to)?;
        return send(&socket, packets);
    }
    let name = s
        .file
        .as_ref()
        .map_or_else(|| "the sample".to_owned(), |p| p.display().to_string());
    let bytes = match &s.file {
        Some(p) => read(p).map_err(|e| {
            format!(
                "cannot read {}: {}",
                p.display(),
                e.to_string()
                    .split(" (os error")
                    .next()
                    .unwrap_or("read failed")
            )
        })?,
        None => include_bytes!("../../qso-graph-interop/tests/fixtures/ALL-sample.TXT").to_vec(),
    };
    let log = alltxt::parse(&bytes, &s.from)?;
    if log.cycles.is_empty() {
        return Err(format!(
            "no received decodes in {name}{}{}",
            if s.from.is_empty() {
                String::new()
            } else {
                format!(" from {}", s.from)
            },
            if log.skipped_modes.is_empty() {
                String::new()
            } else {
                format!(" ({})", log.skipped_summary())
            }
        )
        .into());
    }
    let mut total = 0_u64;
    for c in &log.cycles {
        total = total
            .checked_add(alltxt::period_ms(&c.mode)?)
            .ok_or_else(|| invalid("playback duration overflow"))?;
        for d in &c.decodes {
            alltxt::period_ms(&d.mode)?;
        }
    }
    deadline(Instant::now(), total, s.speed)?;
    println!(
        "{} cycles, {} decodes from {name}{}",
        log.cycles.len(),
        log.decodes,
        if log.skipped == 0 {
            String::new()
        } else {
            format!(" ({})", log.skipped_summary())
        }
    );
    install_stop()?;
    let socket = NumericUdp::sender(s.to)?;
    let mut player = Player::new(&s);
    let period = alltxt::period_ms(&log.cycles[0].mode)?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    let mut stamp = now / period * period;
    let begun = Instant::now();
    let mut elapsed = 0_u64;
    let result = (|| -> Result<()> {
        loop {
            for c in &log.cycles {
                if stopped() || player.close {
                    return Ok(());
                }
                send(&socket, player.play(c, stamp)?)?;
                let period = alltxt::period_ms(&c.mode)?;
                stamp = stamp
                    .checked_add(period)
                    .ok_or_else(|| invalid("cycle timestamp overflow"))?;
                elapsed = elapsed
                    .checked_add(period)
                    .ok_or_else(|| invalid("playback duration overflow"))?;
                wait(
                    &socket,
                    deadline(begun, elapsed, s.speed)?,
                    Some(&mut player),
                )?;
            }
            if !s.repeat {
                break;
            }
        }
        Ok(())
    })();
    // Cleanup is attempted even when playback or a reaction fails.
    let clear = send(
        &socket,
        vec![player.packet(Body::Clear(wsjtx::Clear::default()))],
    );
    let close = send(&socket, vec![player.packet(Body::Close)]);
    result.and(clear).and(close)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    fn player() -> Player {
        let cli =
            crate::Cli::parse_from(["qg-interop", "sim", "--wsjtx", "--to", "127.0.0.1:2237"]);
        let crate::Command::Sim(s) = cli.command else {
            panic!("sim");
        };
        Player::new(&s)
    }
    fn react(p: &mut Player, b: Body) -> Vec<Packet> {
        p.react(&wsjtx::encode(&Packet::new("WSJT-X", b)).unwrap())
    }
    #[test]
    fn replay_clear_close_and_no_invented_acknowledgments() {
        let mut p = player();
        let log =
            alltxt::parse(b"260407_120000 14.074 Rx FT8 -1 0.1 1500 CQ W1AAA AA00", "").unwrap();
        p.play(&log.cycles[0], 86_399_000).unwrap();
        let out = react(&mut p, Body::Replay);
        assert_eq!(out.len(), 2);
        let Body::Decode(d) = &out[0].body else {
            panic!("decode");
        };
        assert!(!d.new);
        assert_eq!(d.time.0, 86_399_000);
        assert!(matches!(&out[1].body,Body::Status(s) if !s.decoding));
        assert!(
            p.react(&wsjtx::encode(&Packet::new("other", Body::Replay)).unwrap())
                .is_empty()
        );
        let mut clear = wsjtx::Clear::default();
        clear.window = Some(1);
        assert!(react(&mut p, Body::Clear(clear)).is_empty());
        assert_eq!(p.window.len(), 1);
        assert!(react(&mut p, Body::Clear(wsjtx::Clear::default())).is_empty());
        assert!(p.window.is_empty());
        assert_eq!(react(&mut p, Body::Replay).len(), 1);
        assert!(react(&mut p, Body::HaltTx(wsjtx::HaltTx::default())).is_empty());
        assert!(react(&mut p, Body::Close).is_empty());
        assert!(p.close);
    }
    #[test]
    fn heartbeat_negotiates_without_an_invented_immediate_reply() {
        let mut p = player();
        let mut heartbeat = wsjtx::Heartbeat::default();
        heartbeat.max_schema = Some(2);
        assert!(react(&mut p, Body::Heartbeat(heartbeat)).is_empty());
        let log =
            alltxt::parse(b"260407_120000 14.074 Rx FT8 -1 0.1 1500 CQ W1AAA AA00", "").unwrap();
        assert!(
            p.play(&log.cycles[0], 0)
                .unwrap()
                .iter()
                .all(|p| p.schema == 2)
        );
    }

    #[test]
    fn changed_status_halt_text_and_configure() {
        let mut p = player();
        p.status.tx_enabled = true;
        p.status.transmitting = true;
        let mut halt = wsjtx::HaltTx::default();
        halt.auto_tx_only = true;
        assert_eq!(react(&mut p, Body::HaltTx(halt)).len(), 1);
        assert!(!p.status.tx_enabled);
        assert!(p.status.transmitting);
        assert_eq!(
            react(&mut p, Body::HaltTx(wsjtx::HaltTx::default())).len(),
            1
        );
        assert!(!p.status.transmitting);
        let mut text = wsjtx::FreeText::default();
        text.text = "SYNTHETIC".into();
        text.send = false;
        assert!(react(&mut p, Body::FreeText(text.clone())).is_empty());
        // Sending empty text selects the staged text only when actually transmitting.
        p.status.transmitting = true;
        text.text = "".into();
        text.send = true;
        assert_eq!(react(&mut p, Body::FreeText(text)).len(), 1);
        assert_eq!(
            p.status.tx_message.as_ref().unwrap().as_str(),
            Some("SYNTHETIC")
        );
        let mut c = wsjtx::Configure::default();
        c.mode = "FT4".into();
        c.frequency_tolerance_hz = u32::MAX;
        c.rx_df_hz = 1700;
        c.dx_call = "W1AAA".into();
        c.dx_grid = "AA00".into();
        c.tr_period_seconds = u32::MAX;
        p.status.sub_mode = Some("C".into());
        assert_eq!(react(&mut p, Body::Configure(c.clone())).len(), 1);
        assert_eq!(p.status.mode.as_str(), Some("FT4"));
        assert_eq!(p.status.sub_mode.as_ref().unwrap().as_str(), Some(""));
        assert_eq!(p.status.rx_df_hz, 1700);
        assert_eq!(p.status.tr_period_seconds, Some(u32::MAX));
        assert!(react(&mut p, Body::Configure(c)).is_empty());
        let mut invalid = wsjtx::Configure::default();
        invalid.mode = "UNKNOWN".into();
        invalid.frequency_tolerance_hz = u32::MAX;
        invalid.rx_df_hz = u32::MAX;
        invalid.tr_period_seconds = u32::MAX;
        assert!(react(&mut p, Body::Configure(invalid)).is_empty());
        assert_eq!(p.status.mode.as_str(), Some("FT4"));
    }
    #[test]
    fn validation_boundaries() {
        assert_eq!(call("w1aaa/p").unwrap(), "W1AAA/P");
        assert!(call("").is_ok());
        assert!(grid("").is_ok());
        assert!(grid("AA00XX").is_ok());
        assert!(id(&"x".repeat(64)).is_ok());
        assert!(id(&"x".repeat(65)).is_err());
        assert!(start("999999_999999").is_ok());
        assert!(speed("inf").is_err());
        assert!(speed("1000").is_ok());
        assert!(target("[::1]:2237").is_ok());
        assert!(target("::1:2237").is_ok());
    }
}
