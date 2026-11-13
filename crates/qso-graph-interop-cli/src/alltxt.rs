// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounded ALL.TXT reader. No I/O; suitable for arbitrary-byte fuzz input.
use std::io;

pub const MAX_FILE: usize = 64 * 1024 * 1024;
pub const MAX_LINE: usize = 16 * 1024;
pub const MAX_DECODES: usize = 100_000;

#[derive(Debug, PartialEq)]
pub struct Line {
    pub stamp: String,
    pub dial_hz: u64,
    pub mode: String,
    pub snr: i32,
    pub dt: f64,
    pub df: u32,
    pub message: String,
    pub low_confidence: bool,
    pub received: bool,
}
#[derive(Debug)]
pub struct Cycle {
    pub stamp: String,
    pub dial_hz: u64,
    pub mode: String,
    pub decodes: Vec<Line>,
}
#[derive(Debug, Default)]
pub struct Log {
    pub cycles: Vec<Cycle>,
    pub skipped: usize,
    pub skipped_modes: Vec<String>,
    pub decodes: usize,
}
impl Log {
    pub fn skipped_summary(&self) -> String {
        let modes = self
            .skipped_modes
            .iter()
            .map(|mode| {
                mode.chars()
                    .flat_map(char::escape_default)
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{} lines skipped: Tx, not a decode, or a mode without an established period{}",
            self.skipped,
            if modes.is_empty() {
                String::new()
            } else {
                format!(": {modes}")
            }
        )
    }
}
pub fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// Python's %y pivot is 1969–2068. Validate the date without a locale or time-zone lookup.
pub fn valid_stamp(s: &str) -> bool {
    if s.len() != 13
        || s.as_bytes()[6] != b'_'
        || !s
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 6 || b.is_ascii_digit())
    {
        return false;
    }
    let n = |a, b| s[a..b].parse::<u32>().unwrap_or(u32::MAX);
    let y = n(0, 2);
    let y = if y >= 69 { 1900 + y } else { 2000 + y };
    let month = n(2, 4);
    let days = match month {
        4 | 6 | 9 | 11 => 30,
        2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
        2 => 28,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    (1..=days).contains(&n(4, 6)) && n(7, 9) < 24 && n(9, 11) < 60 && n(11, 13) < 60
}
fn decimal(s: &str, signed: bool, fraction: bool) -> bool {
    let s = if signed {
        s.strip_prefix('-').unwrap_or(s)
    } else {
        s
    };
    let mut parts = s.split('.');
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits(parts.next().unwrap_or(""))
        && match parts.next() {
            Some(p) => fraction && digits(p) && parts.next().is_none(),
            None => true,
        }
}
pub fn parse_line(raw: &str) -> Option<Line> {
    if raw.len() > MAX_LINE || raw.starts_with(char::is_whitespace) {
        return None;
    }
    let mut rest = raw;
    let mut fields = Vec::with_capacity(7);
    for _ in 0..7 {
        let end = rest.find(char::is_whitespace)?;
        fields.push(&rest[..end]);
        rest = rest[end..].trim_start();
    }
    let message = rest.trim_end();
    if !valid_stamp(fields[0])
        || !decimal(fields[1], false, true)
        || !matches!(fields[2], "Rx" | "Tx")
        || !decimal(fields[4], true, false)
        || !decimal(fields[5], true, true)
        || !decimal(fields[6], false, false)
        || message.is_empty()
    {
        return None;
    }
    let frequency = fields[1].parse::<f64>().ok()? * 1_000_000.0;
    if !frequency.is_finite() || frequency < 0.0 || frequency >= u64::MAX as f64 {
        return None;
    }
    let dt = fields[5].parse::<f64>().ok()?;
    if !dt.is_finite() {
        return None;
    }
    let low = message.strip_suffix('?').and_then(|s| {
        s.char_indices()
            .next_back()
            .filter(|(_, c)| c.is_whitespace())
            .map(|(i, _)| &s[..i])
    });
    Some(Line {
        stamp: fields[0].into(),
        dial_hz: frequency.round_ties_even() as u64,
        mode: fields[3].into(),
        snr: fields[4].parse().ok()?,
        dt,
        df: fields[6].parse().ok()?,
        message: low.unwrap_or(message).into(),
        low_confidence: low.is_some(),
        received: fields[2] == "Rx",
    })
}

pub fn parse(bytes: &[u8], start: &str) -> io::Result<Log> {
    if bytes.len() > MAX_FILE {
        return Err(invalid("ALL.TXT exceeds 64 MiB"));
    }
    let text = String::from_utf8_lossy(bytes);
    let mut log = Log::default();
    // Match str.splitlines(), including old-Mac and Unicode line separators.
    let text = text.replace("\r\n", "\n");
    for raw in text.split([
        '\n', '\r', '\u{b}', '\u{c}', '\u{1c}', '\u{1d}', '\u{1e}', '\u{85}', '\u{2028}',
        '\u{2029}',
    ]) {
        if raw.len() > MAX_LINE {
            return Err(invalid("ALL.TXT line exceeds 16 KiB"));
        }
        if raw.trim().is_empty() {
            continue;
        }
        let Some(line) = parse_line(raw).filter(|l| l.received) else {
            log.skipped += 1;
            continue;
        };
        if line.stamp.as_str() < start {
            continue;
        }
        // These modes have known periods but use separate log/UDP layouts, not Decode.
        if matches!(line.mode.as_str(), "WSPR" | "Echo") {
            log.skipped += 1;
            continue;
        }
        if period_ms(&line.mode).is_err() {
            log.skipped += 1;
            log.skipped_modes.push(line.mode);
            continue;
        }
        if log.decodes == MAX_DECODES {
            return Err(invalid("ALL.TXT exceeds 100000 decodes"));
        }
        log.decodes += 1;
        if let Some(last) = log
            .cycles
            .last_mut()
            .filter(|c| c.stamp == line.stamp && c.dial_hz == line.dial_hz)
        {
            last.decodes.push(line);
        } else {
            log.cycles.push(Cycle {
                stamp: line.stamp.clone(),
                dial_hz: line.dial_hz,
                mode: line.mode.clone(),
                decodes: vec![line],
            });
        }
    }
    log.skipped_modes.sort_unstable();
    log.skipped_modes.dedup();
    Ok(log)
}

/// Released Q65 labels record both period and submode, for example Q65-60C.
/// WSJT-X v3.0.2 widgets/mainwindow.cpp: 11381, 13077–13085, 13144, 15236.
pub fn q65(mode: &str) -> Option<(u64, &str)> {
    let label = mode.strip_prefix("Q65-")?;
    let (period, submode) = label.split_at_checked(label.len().checked_sub(1)?)?;
    let (ms, last) = match period {
        "15" => (15_000, "C"),
        "30" => (30_000, "D"),
        "60" => (60_000, "E"),
        "120" => (120_000, "F"),
        "300" => (300_000, "E"),
        _ => return None,
    };
    (submode >= "A" && submode <= last).then_some((ms, submode))
}

/// Translate released log labels to native mode and submode fields.
pub fn mode_fields(mode: &str) -> (&str, &str) {
    if let Some((_, submode)) = q65(mode) {
        ("Q65", submode)
    } else if matches!(mode, "FT8_SH" | "FT8_SF") {
        ("FT8", "")
    } else {
        (mode, "")
    }
}

/// Established periods, including Q65's period-bearing ALL.TXT label.
/// WSPR and Echo have separate log layouts and are not ordinary Decode records.
/// Evidence and selectable-period exclusions: docs/PROTOCOL.md, simulator timing table.
pub fn period_ms(mode: &str) -> io::Result<u64> {
    match mode {
        "FT8" | "FT8_SH" | "FT8_SF" => Ok(15_000),
        "FT4" => Ok(7_500),
        "JT4" | "JT65" => Ok(60_000),
        "WSPR" => Ok(120_000),
        "Echo" => Ok(3_000),
        _ if let Some((period, _)) = q65(mode) => Ok(period),
        _ => Err(invalid(&format!(
            "cannot establish cycle length for {mode} from ALL.TXT; no period is recorded"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn python_reader_edges() {
        let a = "260407_233915    14.074 Rx FT8    -12  -0.2 1234 CQ Q1AAA DN13  ?   ";
        let l = parse_line(a).unwrap();
        assert_eq!(
            (l.dial_hz, l.snr, l.dt, l.df),
            (14_074_000, -12, -0.2, 1234)
        );
        assert_eq!(l.message, "CQ Q1AAA DN13 ");
        assert!(l.low_confidence);
        for bad in [
            "260229_120000 14 Rx FT8 1 0 1 CQ",
            "260407_240000 14 Rx FT8 1 0 1 CQ",
            "260407_120000 14 Rx FT8 +1 0 1 CQ",
            "260407_120000 1e2 Rx FT8 1 0 1 CQ",
            "260407_120000 14 Rx FT8 1 NaN 1 CQ",
            " 260407_120000 14 Rx FT8 1 0 1 CQ",
            "260407_120000 14 Rx FT8 1 0 1 ",
        ] {
            assert!(parse_line(bad).is_none(), "{bad}");
        }
        assert!(valid_stamp("000229_000000"));
        assert!(!valid_stamp("990229_000000"));
        assert!(
            !parse_line("260407_120000 14 Rx FT8 1 0 1 CQ?")
                .unwrap()
                .low_confidence
        );
    }
    #[test]
    fn grouping_filter_and_counts_match_python() {
        let text = "\nheader\n260407_120000 14 Tx FT8 1 0 1 CQ\n260407_120000 14 Rx FT8 1 0 1 OLD\n260407_120015 14 Rx FT8 1 0 1 A\r\n260407_120015 14 Rx FT4 2 0 2 B\n260407_120015 7 Rx FT8 3 0 3 C\n260407_120000 14 Rx FT8 4 0 4 OLD\n260407_120015 14 Rx FT8 5 0 5 D";
        let l = parse(text.as_bytes(), "260407_120015").unwrap();
        assert_eq!((l.skipped, l.decodes, l.cycles.len()), (2, 4, 3));
        assert_eq!(l.cycles[0].mode, "FT8");
        assert_eq!(l.cycles[0].decodes[1].mode, "FT4");
        assert_eq!(parse(b"\xff\n", "").unwrap().skipped, 1);
        assert!(parse(&vec![b'x'; MAX_LINE + 1], "").is_err());
        assert!(parse(&vec![0; MAX_FILE + 1], "").is_err());
    }
    #[test]
    fn embedded_sample() {
        let l = parse(include_bytes!("../data/ALL-sample.TXT"), "").unwrap();
        assert_eq!((l.cycles.len(), l.decodes), (7, 120));
        assert!(period_ms("Q65").is_err());
        assert_eq!(period_ms("FT4").unwrap(), 7500);
    }

    #[test]
    fn released_periods_and_missing_settings() {
        for (mode, expected) in [
            ("FT8", 15_000),
            ("FT8_SH", 15_000),
            ("FT8_SF", 15_000),
            ("FT4", 7500),
            ("JT4", 60_000),
            ("JT65", 60_000),
            ("WSPR", 120_000),
            ("Echo", 3000),
            ("Q65-15C", 15_000),
            ("Q65-30D", 30_000),
            ("Q65-60E", 60_000),
            ("Q65-120F", 120_000),
            ("Q65-300E", 300_000),
        ] {
            assert_eq!(period_ms(mode).unwrap(), expected, "{mode}");
        }
        for mode in [
            "JT9", "MSK144", "Q65", "FST4", "FST4W", "FreqCal", "UNKNOWN", "Q65-15D", "Q65-30E",
            "Q65-60F", "Q65-300F", "Q65-5A", "Q65-060A", "Q65-60", "Q65-60a", "Q65-", "Q65-60é",
        ] {
            assert!(period_ms(mode).is_err(), "{mode}");
        }
        // Separate native layouts must never be emitted as ordinary Decode messages.
        let log = parse(
            b"260407_120000 14 Rx WSPR -1 0.1 1500 Q1AAA AA00 30\n260407_120000 14 Rx Echo -1 0.1 1500 Q1AAA\n",
            "",
        )
        .unwrap();
        assert_eq!((log.skipped, log.decodes), (2, 0));
        assert!(log.skipped_modes.is_empty());
    }

    #[test]
    fn skip_modes_before_grouping_and_after_start_filter() {
        let log = parse(
            b"260407_115959 14 Rx OLD -1 0 1 CQ\n260407_120000 14 Tx TXMODE -1 0 1 CQ\n260407_120000 14 Rx Q65 -1 0 1 CQ\n260407_120000 14 Rx FT8 -1 0 1 FIRST\n260407_120000 14 Rx JT9 -1 0 1 CQ\n260407_120000 14 Rx FT8 -1 0 1 SECOND\n260407_120000 14 Rx Q65 -1 0 1 CQ\n260407_120015 14 Rx \x1b[31m -1 0 1 CQ\n",
            "260407_120000",
        )
        .unwrap();
        assert_eq!((log.skipped, log.decodes, log.cycles.len()), (5, 2, 1));
        assert_eq!(log.cycles[0].decodes[1].message, "SECOND");
        assert_eq!(
            log.skipped_summary(),
            "5 lines skipped: Tx, not a decode, or a mode without an established period: \\u{1b}[31m, JT9, Q65"
        );
    }
}
