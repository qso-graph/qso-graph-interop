// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded fldigi 4.2.13 stand-in. Source derivations and simulator policy: docs/PROTOCOL.md.
#![forbid(unsafe_code)]
use crate::{
    Sim,
    loopback_tcp::{LoopbackListener, LoopbackTcp},
    sim,
};
use qso_graph_interop::fldigi::MAX_BODY;
use serde_json::Value;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};
mod modes;
mod request;
const MAX_HEADER: usize = 16_384;
const MAX_CONNECTIONS: usize = 16;
// Keeps even base64 + structured JSON responses inside the codec's event budget.
const MAX_BUFFER: usize = 65_536;
fn bad() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid bounded fldigi HTTP request",
    )
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn value(tag: &str, s: impl core::fmt::Display) -> String {
    format!("<{tag}>{}</{tag}>", escape(&s.to_string()))
}
fn response(s: &str) -> Vec<u8> {
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><methodResponse><params><param><value>{s}</value></param></params></methodResponse>").into_bytes()
}
fn fault(s: &str) -> Vec<u8> {
    format!("<?xml version=\"1.0\" encoding=\"utf-8\"?><methodResponse><fault><value><struct><member><name>faultCode</name><value><int>-1</int></value></member><member><name>faultString</name><value>{}</value></member></struct></value></fault></methodResponse>", value("string", s)).into_bytes()
}
fn bytes(s: &[u8]) -> String {
    // Canonical base64 alphabet; no dependency or platform encoder.
    const ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for b in s.chunks(3) {
        let n = (u32::from(b[0]) << 16)
            | (u32::from(*b.get(1).unwrap_or(&0)) << 8)
            | u32::from(*b.get(2).unwrap_or(&0));
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            out.push(if i > b.len() {
                '='
            } else {
                ABC[((n >> shift) & 63) as usize] as char
            });
        }
    }
    value("base64", out)
}
#[derive(Debug)]
struct State {
    frequency: f64,
    carrier: i64,
    mode: &'static str,
    rig_mode: String,
    call: String,
    grid: String,
    trx: &'static str,
    queue: Vec<u8>,
    sent: Vec<u8>,
    rx: Vec<u8>,
    raw_rx: Vec<u8>,
    script: Vec<u8>,
    script_pos: usize,
    credit: f64,
    speed: f64,
    last: Instant,
}
impl State {
    fn new(s: &Sim) -> Self {
        let call = if s.call.is_empty() { "Q1AAA" } else { &s.call };
        let grid = if s.grid.is_empty() { "FN31" } else { &s.grid };
        let other = format!("Q{}BBB", s.seed % 9 + 1);
        Self {
            frequency: 14_070_000.0, carrier: 1500, mode: "BPSK31", rig_mode: "USB".into(),
            call: call.into(), grid: grid.into(), trx: "rx", queue: vec![], sent: vec![],
            rx: vec![], raw_rx: vec![],
            script: format!("CQ CQ DE {other} {other} FN32\n{call} DE {other} UR RST 599 599 QTH FN32\n{other} DE {call} TNX QSO 73 {grid}\n").into_bytes(),
            script_pos: 0, credit: 0.0, speed: s.speed, last: Instant::now(),
        }
    }
    fn rate(&self) -> f64 {
        // Approximate printable bytes/second, deliberately not a DSP timing claim.
        match self.mode {
            "BPSK31" | "QPSK31" => 3.125,
            "BPSK63" | "QPSK63" => 6.25,
            "BPSK125" | "QPSK125" => 12.5,
            "BPSK250" => 25.0,
            "RTTY" => 6.0,
            "CW" => 2.0,
            "MFSK16" => 3.9,
            "MFSK32" => 7.8,
            _ => 3.0,
        }
    }
    fn advance(&mut self, elapsed: Duration) {
        self.credit += elapsed.as_secs_f64() * self.speed * self.rate();
        let n = self.credit.floor() as usize;
        self.credit -= n as f64;
        if self.trx == "tx" {
            let n = n.min(self.queue.len());
            let room = MAX_BUFFER - self.sent.len();
            self.sent.extend(self.queue.drain(..n).take(room));
            // An empty main.tx never counts as sending anything. It waits for text or RX/abort.
            if n > 0 && self.queue.is_empty() {
                self.trx = "rx";
                self.credit = 0.0;
            }
        } else if self.trx == "rx" {
            let n = n
                .min(self.script.len() - self.script_pos)
                .min(MAX_BUFFER - self.rx.len());
            for &b in &self.script[self.script_pos..self.script_pos + n] {
                self.rx.push(b);
                // fldigi add_rx_char clears its raw buffer on the next byte after 4096.
                if self.raw_rx.len() == 4096 {
                    self.raw_rx.clear();
                }
                self.raw_rx.push(b);
            }
            self.script_pos += n;
        }
    }
    fn tick(&mut self) {
        let now = Instant::now();
        self.advance(now.duration_since(self.last));
        self.last = now;
    }
    fn handle(&mut self, body: &[u8]) -> Vec<u8> {
        let (method, fields) = request::parse(body);
        let result = fields.and_then(|f| self.execute(&method, &f));
        match result {
            Ok(s) => {
                let body = response(&s);
                if body.len() > MAX_BODY {
                    fault("simulator response limit")
                } else {
                    body
                }
            }
            Err(s) => fault(&s),
        }
    }
    fn execute(&mut self, method: &str, f: &Value) -> Result<String, String> {
        let string = |key: &str| f[key].as_str().ok_or_else(|| "type error".to_owned());
        let int = |key: &str| f[key].as_i64().ok_or_else(|| "type error".to_owned());
        let s = match method {
            "fldigi.version" => value("string", "4.2.13"),
            "rig.get_frequency" | "main.get_frequency" => value("double", self.frequency),
            "rig.set_frequency" | "main.set_frequency" => {
                let n = f["frequency_hz"]
                    .as_f64()
                    .filter(|n| *n >= 0.0 && *n <= 9_007_199_254_740_991.0)
                    .ok_or("type error")?;
                let old = self.frequency;
                if n >= 1.0 {
                    self.frequency = n.trunc();
                }
                value("double", old)
            }
            "modem.get_carrier" => value("int", self.carrier),
            "modem.set_carrier" => {
                let old = self.carrier;
                self.carrier = int("audio_offset_hz")?.max(1);
                value("int", old)
            }
            "modem.get_name" => value("string", self.mode),
            "modem.get_mode" => value(
                "string",
                modes::MODES
                    .iter()
                    .find(|m| m.0 == self.mode)
                    .expect("known mode")
                    .1,
            ),
            "modem.get_submode" => value(
                "string",
                modes::MODES
                    .iter()
                    .find(|m| m.0 == self.mode)
                    .expect("known mode")
                    .2,
            ),
            "modem.set_by_name" => {
                let name = string("mode")?;
                if let Some(m) = modes::MODES.iter().find(|m| m.0 == name) {
                    let old = self.mode;
                    self.mode = m.0;
                    value("string", old)
                } else {
                    value("string", "No such modem")
                }
            }
            "rig.get_mode" => value("string", &self.rig_mode),
            "rig.set_mode" => {
                self.rig_mode = string("mode")?.into();
                String::new()
            }
            "main.get_trx_state" => value("string", if self.trx == "rx" { "RX" } else { "TX" }),
            "main.get_trx_status" => value("string", self.trx),
            "main.tx" => {
                if self.trx != "tx" {
                    self.credit = 0.0;
                }
                self.trx = "tx";
                String::new()
            }
            "main.rx" | "main.abort" => {
                self.trx = "rx";
                self.credit = 0.0;
                String::new()
            }
            "main.tune" => {
                self.trx = "tune";
                String::new()
            }
            "text.add_tx" => {
                let text = string("text")?;
                if text.len() > MAX_BUFFER - self.queue.len() {
                    return Err("simulator TX buffer limit".into());
                }
                self.queue.extend_from_slice(text.as_bytes());
                String::new()
            }
            "text.clear_tx" => {
                self.queue.clear();
                String::new()
            }
            "text.clear_rx" => {
                self.rx.clear();
                String::new()
            }
            "text.get_rx_length" => value("int", self.rx.len()),
            "rx.get_data" => {
                let s = bytes(&self.raw_rx);
                self.raw_rx.clear();
                s
            }
            "text.get_rx" => {
                let start =
                    int("rx_start")?.clamp(0, self.rx.len().saturating_sub(1) as i64) as usize;
                let n = int("rx_length")?.clamp(-1, (self.rx.len() - start) as i64);
                // Released -1 uses the entire widget length as memcpy size even for nonzero
                // starts (an upstream over-read). Return only defined available bytes here.
                let n = if n == -1 {
                    self.rx.len() - start
                } else {
                    n as usize
                };
                bytes(&self.rx[start..start + n])
            }
            "log.get_call" => value("string", &self.call),
            "log.set_call" => {
                self.call = string("dx_call")?.into();
                String::new()
            }
            "log.get_locator" => value("string", &self.grid),
            "log.set_locator" => {
                self.grid = string("dx_grid")?.into();
                String::new()
            }
            "log.get_frequency" => value("string", format!("{:.6}", self.frequency / 1_000_000.0)),
            "log.get_band" => value("string", modes::band(self.frequency)),
            "log.get_sideband" => value("string", &self.rig_mode),
            "log.get_rst_in" | "log.get_rst_out" => value("string", "599"),
            "log.get_time_on" | "log.get_time_off" => value("string", "0000"),
            "log.get_date_on" | "log.get_date_off" => value("string", "20260101"),
            "logbook.last_record" => value("string", "NONE"),
            "logbook.all_records" => value(
                "string",
                "Fldigi logbook records\n<ADIF_VER:5>3.1.2\n<EOH>\n<EOH>\n",
            ),
            "log.get_name"
            | "log.get_serial_number"
            | "log.get_serial_number_sent"
            | "log.get_exchange"
            | "log.get_state"
            | "log.get_province"
            | "log.get_country"
            | "log.get_qth"
            | "log.get_notes"
            | "log.get_az" => String::new(),
            _ => return Err(format!("{method}: unknown method name")),
        };
        Ok(s)
    }
}
struct Framing {
    end: usize,
    length: usize,
    close: bool,
}
fn framing(input: &[u8]) -> io::Result<Option<Framing>> {
    let Some(end) = input
        .windows(4)
        .position(|b| b == b"\r\n\r\n")
        .map(|n| n + 4)
    else {
        return if input.len() >= MAX_HEADER {
            Err(bad())
        } else {
            Ok(None)
        };
    };
    if end > MAX_HEADER {
        return Err(bad());
    }
    let header = std::str::from_utf8(&input[..end]).map_err(|_| bad())?;
    let mut lines = header.split("\r\n");
    let first = lines.next().ok_or_else(bad)?;
    let parts: Vec<_> = first.split(' ').collect();
    if parts.len() != 3
        || parts[0] != "POST"
        || !parts[1].starts_with('/')
        || !matches!(parts[2], "HTTP/1.1" | "HTTP/1.0")
    {
        return Err(bad());
    }
    let mut length = None;
    let mut close = parts[2] == "HTTP/1.0";
    for line in lines.filter(|s| !s.is_empty()) {
        let (k, v) = line.split_once(':').ok_or_else(bad)?;
        if k.is_empty()
            || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !v.bytes().all(|b| b == 9 || (32..=126).contains(&b))
        {
            return Err(bad());
        }
        let v = v.trim();
        if k.eq_ignore_ascii_case("content-length") {
            if length.is_some() || v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad());
            }
            let n = v.parse::<usize>().map_err(|_| bad())?;
            if n == 0 || n > MAX_BODY {
                return Err(bad());
            }
            length = Some(n);
        }
        if k.eq_ignore_ascii_case("transfer-encoding")
            || (k.eq_ignore_ascii_case("content-encoding") && !v.eq_ignore_ascii_case("identity"))
        {
            return Err(bad());
        }
        if k.eq_ignore_ascii_case("connection") {
            close = v.eq_ignore_ascii_case("close");
        }
    }
    Ok(Some(Framing {
        end,
        length: length.ok_or_else(bad)?,
        close,
    }))
}
struct Connection {
    socket: LoopbackTcp,
    input: Vec<u8>,
    output: Vec<u8>,
    written: usize,
    close: bool,
}
impl Connection {
    fn step(&mut self, state: &mut State) -> io::Result<bool> {
        if self.output.is_empty() {
            let ready = framing(&self.input)?.is_some_and(|f| self.input.len() >= f.end + f.length);
            let mut buf = [0; 4096];
            if !ready {
                match self.socket.read(&mut buf) {
                    Ok(0) => return Ok(false),
                    Ok(n) => self.input.extend_from_slice(&buf[..n]),
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(e),
                }
            }
            if self.input.len() > MAX_HEADER + MAX_BODY {
                return Err(bad());
            }
            if let Some(frame) = framing(&self.input)?
                && self.input.len() >= frame.end + frame.length
            {
                let body = state.handle(&self.input[frame.end..frame.end + frame.length]);
                self.input.drain(..frame.end + frame.length);
                self.close = frame.close;
                self.output = format!("HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: {}\r\n\r\n", body.len(), if self.close { "close" } else { "keep-alive" }).into_bytes();
                self.output.extend(body);
            }
        }
        if !self.output.is_empty() {
            match self.socket.write(&self.output[self.written..]) {
                Ok(0) => return Ok(false),
                Ok(n) => self.written += n,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(e),
            }
            if self.written == self.output.len() {
                if self.close {
                    return Ok(false);
                }
                self.output.clear();
                self.written = 0;
                self.socket.restart_deadline();
            }
        }
        Ok(true)
    }
}
pub fn run(s: &Sim) -> Result<(), Box<dyn std::error::Error>> {
    let address = s.address.unwrap_or_else(|| ([127, 0, 0, 1], 7362).into());
    let listener = LoopbackListener::bind(address)?;
    sim::install_stop()?;
    let mut state = State::new(s);
    let mut connections: Vec<Connection> = vec![];
    eprintln!("fldigi simulator listening on {address}");
    while !sim::stopped() {
        state.tick();
        // Bound accept work as well as active streams; a flood cannot starve existing peers.
        for _ in 0..MAX_CONNECTIONS {
            match listener.accept() {
                Ok(socket) if connections.len() < MAX_CONNECTIONS => connections.push(Connection {
                    socket,
                    input: vec![],
                    output: vec![],
                    written: 0,
                    close: false,
                }),
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }
        connections.retain_mut(|c| c.step(&mut state).unwrap_or(false));
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cli, Command};
    use clap::Parser;
    use qso_graph_interop::{
        Codec, Event,
        codec::{DecodeOptions, EncodeOptions},
        fldigi::Fldigi,
    };
    use serde_json::json;
    fn state() -> State {
        let c = Cli::try_parse_from(["qg-interop", "sim", "--fldigi"]).unwrap();
        let Command::Sim(s) = c.command else {
            panic!("sim")
        };
        State::new(&s)
    }
    fn command(fields: Value) -> Vec<Vec<u8>> {
        Fldigi::default().encode_commands(&Event::from_value(json!({"spec":"0.1.0","family":"fldigi","mapping":1,"kind":"control","context":{"direction":"unknown","exchange":"unknown"},"fields":fields})).unwrap(), &EncodeOptions::default()).unwrap().requests().iter().map(|r| r.bytes().to_vec()).collect()
    }
    fn request(method: &str, args: &str) -> Vec<u8> {
        format!("<methodCall><methodName>{method}</methodName><params>{args}</params></methodCall>")
            .into_bytes()
    }
    fn checked(method: &str, body: &[u8], expected_kind: &str) -> bool {
        let mut codec = Fldigi::default();
        let e = codec
            .decode(
                body,
                &DecodeOptions::default().with_metadata(
                    qso_graph_interop::fldigi::RequestAssociation::from_value(
                        json!({"method":method,"arguments":[0,-1]}),
                    )
                    .unwrap()
                    .into(),
                ),
            )
            .unwrap()
            .events
            .remove(0);
        e.value()["kind"] == expected_kind
    }
    #[test]
    fn every_annex_e_method_has_the_released_response_type_and_wrong_type_is_detected() {
        // Independent catalog: specification rows, not the private library or server dispatch.
        let annex = include_str!("../../../spec/annex-e-fldigi.md");
        let table = annex
            .split("## E.4 Complete mapping catalog")
            .nth(1)
            .unwrap()
            .split("### Remaining pinned methods")
            .next()
            .unwrap();
        let mut count = 0;
        for row in table.lines().filter(|l| l.starts_with("| `")) {
            let cols: Vec<_> = row.split('|').collect();
            let method = cols[1].trim().trim_matches('`');
            let sig = cols[2].trim().trim_matches('`');
            if !sig.contains(':') {
                continue;
            }
            let mut s = state();
            s.advance(Duration::from_secs(120));
            let (_, tag) = sig.split_once(':').unwrap();
            let args = match tag {
                "n" => "",
                "s" => "<param><value><string>BPSK63</string></value></param>",
                "d" => "<param><value><double>7070000</double></value></param>",
                "i" => "<param><value><int>1500</int></value></param>",
                "ii" => {
                    "<param><value><int>0</int></value></param><param><value><int>-1</int></value></param>"
                }
                _ => panic!("signature: {sig}"),
            };
            let body = s.handle(&request(method, args));
            let kind = if sig.starts_with("n:") {
                "native"
            } else if method == "text.get_rx" || method == "rx.get_data" {
                "decode"
            } else if cols[4].contains("radio_state") {
                "radio_state"
            } else {
                "result"
            };
            assert!(
                checked(method, &body, kind),
                "{method}: {}",
                String::from_utf8_lossy(&body)
            );
            let expected_tag = match sig.split_once(':').unwrap().0 {
                "s" => "string",
                "d" => "double",
                "i" => "int",
                "6" => "base64",
                "n" => "",
                _ => panic!("{sig}"),
            };
            if !expected_tag.is_empty() {
                assert!(
                    String::from_utf8_lossy(&body).contains(&format!("<{expected_tag}>"))
                        || (expected_tag == "string"
                            && String::from_utf8_lossy(&body).contains("<value></value>")),
                    "{method}"
                );
            }
            if sig.starts_with("n:") {
                assert!(String::from_utf8_lossy(&body).contains("<value></value>"));
            }
            count += 1;
        }
        assert_eq!(count, 50);
        assert!(checked(
            "rig.get_frequency",
            &response("<double>14070000</double>"),
            "radio_state"
        ));
        assert!(!checked(
            "rig.get_frequency",
            &response("<string>14070000</string>"),
            "radio_state"
        ));
    }
    #[test]
    fn fields_commands_change_old_value_state_and_send_queued_text_before_receiving() {
        let mut s = state();
        let reqs =
            command(json!({"operation":"set_frequency","frequency_hz":7070000,"rx_offset_hz":850}));
        assert_eq!(reqs.len(), 2);
        assert_eq!(s.handle(&reqs[0]), response("<double>14070000</double>"));
        assert_eq!(s.handle(&reqs[1]), response("<int>1500</int>"));
        assert_eq!(s.frequency, 7070000.0);
        assert_eq!(s.carrier, 850);
        let text = "CQ Q1AAA Q1AAA";
        let reqs = command(json!({"operation":"send_text","text":text,"transmit":true}));
        assert_eq!(reqs.len(), 2);
        s.handle(&reqs[0]);
        assert_eq!(s.trx, "rx");
        s.handle(&reqs[1]);
        assert_eq!(s.trx, "tx");
        assert!(s.sent.is_empty());
        s.advance(Duration::from_secs(1));
        assert_eq!(&s.sent, b"CQ ");
        assert_eq!(s.trx, "tx");
        s.advance(Duration::from_secs(10));
        assert_eq!(&s.sent, text.as_bytes());
        assert_eq!(s.trx, "rx");
        // Planted main.tx without text must fail this same sent-text invariant.
        let mut empty = state();
        empty.handle(&reqs[1]);
        empty.advance(Duration::from_secs(20));
        assert_ne!(empty.sent, text.as_bytes());
        assert_eq!(empty.trx, "tx");
        s.handle(&reqs[0]);
        s.handle(&reqs[1]);
        s.advance(Duration::from_secs(1));
        let sent = s.sent.clone();
        s.handle(&command(json!({"operation":"halt_tx"}))[0]);
        s.advance(Duration::from_secs(20));
        assert_eq!(s.trx, "rx");
        assert_eq!(s.sent, sent);
        s.handle(&request("main.tx", ""));
        s.handle(&request("main.rx", ""));
        s.advance(Duration::from_secs(20));
        assert_eq!(s.sent, sent);
    }
    #[test]
    fn offsets_raw_drain_clear_and_mode_rates_are_consistent() {
        let mut s = state();
        s.advance(Duration::from_secs(120));
        let rx = s.rx.clone();
        assert!(rx.starts_with(b"CQ CQ DE Q2BBB"));
        let args = |start: i64, n: i64| {
            format!(
                "<param><value><int>{start}</int></value></param><param><value><int>{n}</int></value></param>"
            )
        };
        for (start, n) in [(0, 5), (5, 7), (2, 4), (0, 0)] {
            assert_eq!(
                s.handle(&request("text.get_rx", &args(start as i64, n as i64))),
                response(&bytes(&rx[start..start + n]))
            );
        }
        assert_eq!(
            s.handle(&request("text.get_rx", &args(0, -1))),
            response(&bytes(&rx))
        );
        assert_eq!(s.handle(&request("rx.get_data", "")), response(&bytes(&rx)));
        assert_eq!(s.handle(&request("rx.get_data", "")), response(&bytes(b"")));
        assert_eq!(s.rx, rx);
        s.handle(&request("text.clear_rx", ""));
        assert!(s.rx.is_empty());
        assert_eq!(
            s.handle(&request("text.get_rx", &args(0, 0))),
            response(&bytes(b""))
        );
        let old_rate = s.rate();
        s.execute("modem.set_by_name", &json!({"mode":"BPSK125"}))
            .unwrap();
        assert_eq!(s.rate(), 4.0 * old_rate);
        assert_eq!(
            s.execute("modem.get_mode", &json!({})).unwrap(),
            value("string", "PSK")
        );
        assert_eq!(
            s.execute("modem.get_submode", &json!({})).unwrap(),
            value("string", "PSK125")
        );
        assert_eq!(
            s.execute("modem.set_by_name", &json!({"mode":"made up"}))
                .unwrap(),
            value("string", "No such modem")
        );
        assert_eq!(s.mode, "BPSK125");
    }
    #[test]
    fn empty_logbook_markers_and_band_buckets_match_released_source() {
        let mut s = state();
        assert_eq!(
            s.handle(&request("logbook.last_record", "")),
            response("<string>NONE</string>")
        );
        assert_eq!(
            s.handle(&request("logbook.all_records", "")),
            response(&value(
                "string",
                "Fldigi logbook records\n<ADIF_VER:5>3.1.2\n<EOH>\n<EOH>\n"
            ))
        );
        for (hz, label) in [
            (14_070_000.0, "20m"),
            (7_070_000.0, "40m"),
            (28_999_999.0, "10m"),
            (29_999_999.0, "10m"),
            (30_000_000.0, "other"),
            (222_000_000.0, "1.25m"),
            (24_000_000_000.0, "1.25cm"),
        ] {
            s.frequency = hz;
            assert_eq!(
                s.handle(&request("log.get_band", "")),
                response(&value("string", label))
            );
        }
        // Planted empty-string placeholders would lose the released logbook markers.
        assert_ne!(response(""), s.handle(&request("logbook.last_record", "")));
    }
    #[test]
    fn malformed_xml_unknown_methods_and_bounded_framing() {
        let mut s = state();
        assert_eq!(s.handle(b"broken"), fault(": unknown method name"));
        assert_eq!(
            s.handle(b"<methodCall><methodName>rig.get_frequency</methodName>"),
            response("<double>14070000</double>")
        );
        // Bundled parsing dispatches recognizable methods without requiring outer XML.
        assert_eq!(s.handle(b"<methodName>rig.set_frequency</methodName><params><param><value><double>7070000.9</double></value></param>"), response("<double>14070000</double>"));
        assert_eq!(s.frequency, 7070000.0);
        assert_eq!(
            s.handle(&request("modem.set_carrier", "")),
            fault("type error: expected an array")
        );
        s.handle(&request(
            "log.set_call",
            "<param><value><string>Q1AAA &amp;amp; &#13;</string></value></param>",
        ));
        assert_eq!(s.call, "Q1AAA &amp; &#13;");
        assert_eq!(
            s.handle(&request(
                "modem.set_carrier",
                "<param><value><int/></value></param>"
            )),
            response("<int>1500</int>")
        );
        assert_eq!(s.carrier, 1);
        assert_eq!(
            s.handle(&request("unknown.method", "")),
            fault("unknown.method: unknown method name")
        );
        assert_eq!(
            s.handle(&request(
                "modem.set_carrier",
                "<param><value><string>wrong</string></value></param>"
            )),
            fault("type error")
        );
        for header in [
            "Content-Length: 0",
            "Content-Length: 262145",
            "X: x",
            "Content-Length: 1\r\nContent-Length: 1",
            "Content-Length: 1\r\nTransfer-Encoding: chunked",
            "Content-Length: -1",
            "Content-Length: 1\r\nContent-Encoding: gzip",
        ] {
            assert!(
                framing(format!("POST /RPC2 HTTP/1.1\r\n{header}\r\n\r\n").as_bytes()).is_err()
            );
        }
        assert!(framing(&vec![b'x'; MAX_HEADER]).is_err());
        assert_eq!(
            framing(b"POST /RPC2 HTTP/1.1\r\nContent-Length: 262144\r\n\r\n")
                .unwrap()
                .unwrap()
                .length,
            MAX_BODY
        );
        s.queue.resize(MAX_BUFFER, b'x');
        assert!(s.execute("text.add_tx", &json!({"text":"x"})).is_err());
        s.script = vec![b'x'; 5000];
        s.script_pos = 0;
        s.advance(Duration::from_secs(2000));
        assert_eq!(s.raw_rx.len(), 904);
    }
}
