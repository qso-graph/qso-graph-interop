// SPDX-License-Identifier: GPL-3.0-or-later
//! Scripted N1MM peer policy. Annex D's broadcast catalog, with no codec I/O.
use crate::{Sim, alltxt::invalid, numeric_udp::NumericUdp, sim};
use core::fmt::Display;
use core::mem;
use qso_graph_interop::{Codec, n1mm::N1mm};
use std::io;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const HOUR_MS: u64 = 3_600_000;
const DEFAULT_COUNT: u64 = 12;

pub fn station(s: &str) -> std::result::Result<String, String> {
    if !(1..=64).contains(&s.len())
        || !s.bytes().all(|b| (b' '..=b'~').contains(&b))
        || s.trim().is_empty()
    {
        return Err(
            "station must contain 1–64 printable ASCII characters, including a non-space".into(),
        );
    }
    Ok(s.into())
}

// Canonical UTF-8 XML uses Annex D D.4 names and D.5 declaration/escaping/order
// conventions. The library's field-built writer only constructs inbound commands.
fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#13;"),
            _ => out.push(c),
        }
    }
    out
}
fn leaf(name: &str, value: impl Display) -> String {
    format!("<{name}>{}</{name}>", escape(&value.to_string()))
}
fn document(root: &str, body: String) -> Vec<u8> {
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><{root}>{body}</{root}>").into_bytes()
}
// Fixed start is 2026-01-01 00:00:00 UTC. Logical time never reads the wall clock.
fn timestamp(ms: u64) -> io::Result<String> {
    let mut days = ms / 86_400_000;
    let mut year = 2026;
    loop {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let length = if leap { 366 } else { 365 };
        if days < length {
            break;
        }
        days -= length;
        year += 1;
        if year > 9999 {
            return Err(invalid("contest timestamp exceeds year 9999"));
        }
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let lengths = [
        31,
        if leap { 29 } else { 28 },
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
    let mut month = 1;
    for length in lengths {
        if days < length {
            break;
        }
        days -= length;
        month += 1;
    }
    let seconds = ms / 1000 % 86_400;
    Ok(format!(
        "{year:04}-{month:02}-{:02} {:02}:{:02}:{:02}",
        days + 1,
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    ))
}
fn khz(hz: u64) -> String {
    let s = format!("{}.{:03}", hz / 1000, hz % 1000);
    s.trim_end_matches('0').trim_end_matches('.').into()
}
fn band(hz: u64) -> &'static str {
    match hz {
        1_800_000..=2_000_000 => "1.8",
        3_500_000..=4_000_000 => "3.5",
        7_000_000..=7_300_000 => "7",
        10_100_000..=10_150_000 => "10",
        14_000_000..=14_350_000 => "14",
        18_068_000..=18_168_000 => "18",
        21_000_000..=21_450_000 => "21",
        24_890_000..=24_990_000 => "24",
        28_000_000..=29_700_000 => "28",
        50_000_000..=54_000_000 => "50",
        _ => "",
    }
}

struct Contact {
    id: u64,
    call: String,
    stamp: String,
    radio: u8,
    hz: u64,
    points: u64,
}
struct Contest {
    call: String,
    station: String,
    radios: u8,
    active: u8,
    transmitting: bool,
    hz: [u64; 2],
    random: u64,
    sent: u64,
    qsos: u64,
    points: u64,
}
impl Contest {
    fn new(s: &Sim) -> Self {
        Self {
            call: if s.call.is_empty() {
                "N0CALL".into()
            } else {
                s.call.clone()
            },
            station: s.station.clone(),
            radios: s.radios,
            active: 1,
            transmitting: false,
            hz: [14_025_000, 7_025_000],
            random: s.seed,
            sent: 0,
            qsos: 0,
            points: 0,
        }
    }
    fn common(&self) -> String {
        leaf("app", "N1MM") + &leaf("StationName", &self.station) + &leaf("mycall", &self.call)
    }
    fn app_info(&self) -> Vec<u8> {
        document(
            "AppInfo",
            leaf("app", "N1MM")
                + &leaf("dbname", "SYNTHETIC")
                + &leaf("contestnr", 1)
                + &leaf("contestname", "SYNTHETIC")
                + &leaf("StationName", &self.station)
                + &leaf("mycall", &self.call),
        )
    }
    fn radios(&self) -> Vec<Vec<u8>> {
        (1..=self.radios)
            .map(|n| {
                let hz = self.hz[usize::from(n - 1)];
                document(
                    "RadioInfo",
                    self.common()
                        + &leaf("RadioNr", n)
                        + &leaf("Freq", hz / 10)
                        + &leaf("TXFreq", hz / 10)
                        + &leaf("Mode", "CW")
                        + &leaf("OpCall", &self.call)
                        + &leaf("IsRunning", if n == 1 { "True" } else { "False" })
                        + &leaf("FocusRadioNr", self.active)
                        + &leaf("IsStereo", if self.radios == 2 { "True" } else { "False" })
                        + &leaf("ActiveRadioNr", self.active)
                        + &leaf(
                            "IsTransmitting",
                            if self.transmitting && n == self.active {
                                "True"
                            } else {
                                "False"
                            },
                        )
                        + &leaf("RadioName", format!("SYNTHETIC-RADIO-{n}")),
                )
            })
            .collect()
    }
    fn contact(&self, root: &str, c: &Contact, old: Option<&str>) -> Vec<u8> {
        let mut body = self.common()
            + &leaf("contestname", "SYNTHETIC")
            + &leaf("contestnr", 1)
            + &leaf("timestamp", &c.stamp)
            + &leaf("band", band(c.hz))
            + &leaf("call", &c.call)
            + &leaf("ID", format!("{:032x}", c.id));
        if root != "contactdelete" {
            body += &(leaf("rxfreq", c.hz / 10)
                + &leaf("txfreq", c.hz / 10)
                + &leaf("operator", &self.call)
                + &leaf("mode", "CW")
                + &leaf("snt", "599")
                + &leaf("sntnr", format!("{:03}", c.id))
                + &leaf("rcv", "599")
                + &leaf("rcvnr", format!("{:03}", c.id + 10))
                + &leaf("points", c.points)
                + &leaf("radionr", c.radio)
                + &leaf("IsRunQSO", if c.radio == 1 { "True" } else { "False" })
                + &leaf("comment", "Synthetic contest contact"));
        }
        if let Some(old) = old {
            body += &(leaf("oldcall", old) + &leaf("oldtimestamp", &c.stamp));
        }
        document(root, body)
    }
    fn spot(&self, c: &Contact, action: &str) -> Vec<u8> {
        document(
            "spot",
            leaf("app", "N1MM")
                + &leaf("StationName", &self.station)
                + &leaf("dxcall", &c.call)
                + &leaf("frequency", khz(c.hz))
                + &leaf("spottercall", &self.call)
                + &leaf("timestamp", &c.stamp)
                + &leaf("action", action)
                + &leaf("mode", "CW")
                + &leaf("comment", "Synthetic spot"),
        )
    }
    fn score(&self, stamp: &str) -> Vec<u8> {
        document(
            "dynamicresults",
            leaf("contest", "SYNTHETIC")
                + &leaf("call", &self.call)
                + "<breakdown>"
                + &format!(
                    "<qso band=\"total\" mode=\"ALL\">{}</qso><point band=\"total\" mode=\"ALL\">{}</point>",
                    self.qsos, self.points
                )
                + "</breakdown>"
                + &leaf("score", self.points)
                + &leaf("timestamp", stamp),
        )
    }
    fn play(&mut self, ms: u64) -> Result<Vec<Vec<u8>>> {
        // Clear the previous transmitter before announcing the next radio. A consumer
        // maintaining the latest state for each RadioNr must never see two TX flags.
        self.transmitting = false;
        let mut out = self.radios();
        self.active = (self.sent % u64::from(self.radios)) as u8 + 1;
        self.transmitting = true;
        out.extend(self.radios());
        self.sent = self
            .sent
            .checked_add(1)
            .ok_or_else(|| invalid("contact counter overflow"))?;
        // Full-width deterministic LCG, including seed zero; no platform RNG or clock.
        self.random = self
            .random
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let suffix = self.random >> 32;
        let mut c = Contact {
            id: self.sent,
            call: format!(
                "W{}{}{}{}",
                suffix % 10,
                char::from(b'A' + (suffix / 10 % 26) as u8),
                char::from(b'A' + (suffix / 260 % 26) as u8),
                char::from(b'A' + (suffix / 6760 % 26) as u8)
            ),
            stamp: timestamp(ms)?,
            radio: self.active,
            hz: self.hz[usize::from(self.active - 1)],
            points: 2,
        };
        out.push(self.spot(&c, "add"));
        out.push(self.contact("contactinfo", &c, None));
        self.qsos += 1;
        self.points += c.points;
        out.push(self.score(&c.stamp));
        if self.sent.is_multiple_of(5) {
            let old = c.call.clone();
            c.call += "/P";
            c.points = 3;
            self.points += 1;
            out.push(self.contact("contactreplace", &c, Some(&old)));
            out.push(self.score(&c.stamp));
            // Remove the spot under its original key.
            let changed = mem::replace(&mut c.call, old);
            out.push(self.spot(&c, "delete"));
            c.call = changed;
        } else {
            out.push(self.spot(&c, "delete"));
        }
        if self.sent.is_multiple_of(7) {
            out.push(self.contact("contactdelete", &c, None));
            self.qsos -= 1;
            self.points -= c.points;
            out.push(self.score(&c.stamp));
        }
        Ok(out)
    }
    /// Annex D D.5, S1 "Setting N1MM's Frequency": QSY changes N1MM and
    /// the connected radio's frequency. Spectrum affects the display only.
    /// The connected NumericUdp socket admits only the configured target.
    fn react(&mut self, bytes: &[u8]) {
        let Ok(out) = N1mm::default().decode(bytes, &Default::default()) else {
            return;
        };
        let Some(event) = out.events.first() else {
            return;
        };
        let f = &event.value()["fields"];
        if f["operation"] != "set_frequency" {
            return;
        }
        let (Some(n), Some(hz)) = (f["radio"]["number"].as_u64(), f["frequency_hz"].as_u64())
        else {
            return;
        };
        // Simulator policy: tune at RadioInfo's native 10 Hz resolution, dropping
        // the single-Hz remainder. No precision beyond that report is claimed.
        if (1..=u64::from(self.radios)).contains(&n) {
            self.hz[n as usize - 1] = hz / 10 * 10;
        }
    }
}
fn send(socket: &NumericUdp, packets: Vec<Vec<u8>>) -> Result<()> {
    for bytes in packets {
        // Validate complete canonical datagrams with the shared library before I/O.
        let out = N1mm::default().decode(&bytes, &Default::default())?;
        if out.events.len() != 1 || !out.skips.is_empty() {
            return Err(invalid("invalid simulator datagram").into());
        }
        socket.send(&bytes)?;
    }
    Ok(())
}
fn deadline(start: Instant, ms: u64, speed: f64) -> io::Result<Instant> {
    start
        .checked_add(sim::delay(ms, speed)?)
        .ok_or_else(|| invalid("playback deadline is too large"))
}
fn wait(socket: &NumericUdp, until: Instant, contest: &mut Contest) -> Result<()> {
    let mut bytes = [0; 65_535];
    while !sim::stopped() {
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
            Ok((n, _)) => contest.react(&bytes[..n]),
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
pub fn run(s: &Sim) -> Result<()> {
    let limit = s
        .count
        .or(if s.repeat { None } else { Some(DEFAULT_COUNT) });
    let end_ms = limit.unwrap_or(DEFAULT_COUNT) * HOUR_MS / u64::from(s.rate);
    deadline(Instant::now(), end_ms, s.speed)?;
    timestamp(end_ms)?;
    sim::install_stop()?;
    let socket = NumericUdp::sender(s.to)?;
    let mut contest = Contest::new(s);
    let start = Instant::now();
    send(&socket, vec![contest.app_info()])?;
    send(&socket, contest.radios())?;
    send(&socket, vec![contest.score(&timestamp(0)?)])?;
    let result = (|| -> Result<()> {
        let mut tick = 1000_u64;
        loop {
            let qso = contest
                .sent
                .checked_add(1)
                .and_then(|n| n.checked_mul(HOUR_MS))
                .ok_or_else(|| invalid("contest duration overflow"))?
                / u64::from(s.rate);
            let next = tick.min(qso);
            wait(&socket, deadline(start, next, s.speed)?, &mut contest)?;
            if sim::stopped() {
                break;
            }
            if next == tick {
                contest.transmitting = false;
                send(&socket, contest.radios())?;
                tick = tick
                    .checked_add(1000)
                    .ok_or_else(|| invalid("contest duration overflow"))?;
            }
            if next == qso {
                send(&socket, contest.play(next)?)?;
                if limit.is_some_and(|n| contest.sent >= n) {
                    break;
                }
            }
        }
        Ok(())
    })();
    contest.transmitting = false;
    // No invented Close or command acknowledgment; leave both transmit flags clear.
    let cleanup = send(&socket, contest.radios());
    result.and(cleanup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    fn contest(radios: &str) -> Contest {
        let cli = crate::Cli::parse_from([
            "qg-interop",
            "sim",
            "--n1mm",
            "--to",
            "127.0.0.1:12060",
            "--radios",
            radios,
        ]);
        let crate::Command::Sim(s) = cli.command else {
            panic!("sim");
        };
        Contest::new(&s)
    }
    #[test]
    fn utc_rollovers_calendar_and_frequency_units() {
        assert_eq!(timestamp(0).unwrap(), "2026-01-01 00:00:00");
        assert_eq!(timestamp(86_399_999).unwrap(), "2026-01-01 23:59:59");
        assert_eq!(timestamp(86_400_000).unwrap(), "2026-01-02 00:00:00");
        assert_eq!(timestamp(31 * 86_400_000).unwrap(), "2026-02-01 00:00:00");
        assert_eq!(
            timestamp((365 + 365 + 31 + 28) * 86_400_000).unwrap(),
            "2028-02-29 00:00:00"
        );
        assert_eq!(
            timestamp((365 + 365 + 31 + 29) * 86_400_000).unwrap(),
            "2028-03-01 00:00:00"
        );
        assert!(timestamp(u64::MAX).is_err());
        for (hz, expected) in [
            (0, "0"),
            (137_500, "137.5"),
            (475_000, "475"),
            (14_053_284, "14053.284"),
            (1, "0.001"),
        ] {
            assert_eq!(khz(hz), expected);
        }
        assert_eq!(band(21_050_000), "21");
        assert_eq!(band(0), "");
    }
    #[test]
    fn commands_use_shared_decoder_and_never_generate_a_reply() {
        let mut c = contest("2");
        let before = c.radios();
        for bytes in [b"malformed".as_slice(), b"<Spectrum><Name>SYNTHETIC</Name></Spectrum>",
            b"<radio_setfrequency><radionr>3</radionr><frequency>21000</frequency></radio_setfrequency>",
            b"<radio_setfrequency><radionr>1</radionr><frequency>-1</frequency></radio_setfrequency>",
            b"<radio_setfrequency><radionr>1</radionr><frequency>1</frequency><frequency>2</frequency></radio_setfrequency>",
            b"<radio_setfrequency><radionr>1</radionr><frequency>NaN</frequency></radio_setfrequency>"] {
            c.react(bytes);
            assert_eq!(c.radios(), before);
        }
        c.react(b"<radio_setfrequency><radionr>2</radionr><frequency>14053.284</frequency></radio_setfrequency>");
        assert_eq!(c.hz, [14_025_000, 14_053_280]);
        assert_eq!(c.active, 1);
        let mut one = contest("1");
        one.react(b"<radio_setfrequency><radionr>2</radionr><frequency>475</frequency></radio_setfrequency>");
        assert_eq!(one.hz, [14_025_000, 7_025_000]);
        one.react(b"<radio_setfrequency><radionr>1</radionr><frequency>475</frequency></radio_setfrequency>");
        assert_eq!(one.hz[0], 475_000);
    }
}
