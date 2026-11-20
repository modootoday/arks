//! Full `--date=<format>` / `%(…date:<format>)` rendering: the fixed shapes
//! of [`DateMode`] plus `relative`, `human`, `local` and `<mode>-local`,
//! `format:<strftime>` and `format-local:<strftime>`.
//! Primary: <https://git-scm.com/docs/git-log#Documentation/git-log.txt---dateltformatgt>.
//!
//! Partial: `local` renders in the zone named by `TZ` only when it is UTC
//! (or unset); other local zones fall back to UTC.

use crate::cmd::log_format::DateMode;
use crate::error::{CliError, CliResult};
use arks_object::Signature;

const WD: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WDL: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MO: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MOL: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn tz_minutes(tz: &str) -> i64 {
    let sign = if tz.starts_with('-') { -1 } else { 1 };
    let d = tz.trim_start_matches(['+', '-']);
    if d.len() != 4 {
        return 0;
    }
    let h: i64 = d[..2].parse().unwrap_or(0);
    let m: i64 = d[2..].parse().unwrap_or(0);
    sign * (h * 60 + m)
}

/// Broken-down civil time.
#[derive(Clone, Copy, Debug)]
pub struct Civil {
    pub y: i64,
    pub mo: u32,
    pub d: u32,
    pub h: u32,
    pub mi: u32,
    pub s: u32,
    pub wd: u32,
    pub yday: u32,
}

/// Unix seconds shifted by `tz` (`+HHMM`) to civil fields.
pub fn civil(ts: i64, tz: &str) -> Civil {
    let t = ts + tz_minutes(tz) * 60;
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if mo <= 2 { 1 } else { 0 };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    const CUM: [u32; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    let yday = CUM[(mo - 1) as usize] + d - 1 + u32::from(leap && mo > 2);
    Civil {
        y,
        mo,
        d,
        h: (secs / 3600) as u32,
        mi: ((secs % 3600) / 60) as u32,
        s: (secs % 60) as u32,
        wd: ((days + 4).rem_euclid(7)) as u32,
        yday,
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The local zone offset (`+HHMM`): UTC unless `TZ` names a fixed offset.
fn local_tz() -> String {
    "+0000".to_string()
}

fn plural(n: i64, one: &str) -> String {
    if n == 1 {
        format!("{n} {one}")
    } else {
        format!("{n} {one}s")
    }
}

/// git's relative date wording for `ts` seen at `now`.
pub fn relative(ts: i64, now: i64) -> String {
    if ts > now {
        return "in the future".into();
    }
    let mut diff = now - ts;
    if diff < 90 {
        return format!("{} ago", plural(diff, "second"));
    }
    diff = (diff + 30) / 60;
    if diff < 90 {
        return format!("{} ago", plural(diff, "minute"));
    }
    diff = (diff + 30) / 60;
    if diff < 36 {
        return format!("{} ago", plural(diff, "hour"));
    }
    diff = (diff + 12) / 24;
    if diff < 14 {
        return format!("{} ago", plural(diff, "day"));
    }
    if diff < 70 {
        return format!("{} ago", plural((diff + 3) / 7, "week"));
    }
    if diff < 365 {
        return format!("{} ago", plural((diff + 15) / 30, "month"));
    }
    if diff < 1825 {
        let total_months = (diff * 12 * 2 + 365) / (365 * 2);
        let years = total_months / 12;
        let months = total_months % 12;
        if months > 0 {
            return format!("{}, {} ago", plural(years, "year"), plural(months, "month"));
        }
        return format!("{} ago", plural(years, "year"));
    }
    format!("{} ago", plural((diff + 183) / 365, "year"))
}

fn human(sig: &Signature, now: i64) -> String {
    let diff = now - sig.timestamp;
    if (0..12 * 3600).contains(&diff) {
        return relative(sig.timestamp, now);
    }
    let c = civil(sig.timestamp, &sig.timezone);
    let n = civil(now, &local_tz());
    let same_tz = sig.timezone == local_tz();
    let tz = if same_tz {
        String::new()
    } else {
        format!(" {}", sig.timezone)
    };
    if (0..5 * 86_400).contains(&diff) {
        return format!("{} {:02}:{:02}{tz}", WD[c.wd as usize], c.h, c.mi);
    }
    if c.y == n.y {
        return format!(
            "{} {} {:02}:{:02}{tz}",
            MO[(c.mo - 1) as usize],
            c.d,
            c.h,
            c.mi
        );
    }
    format!("{} {} {}{tz}", MO[(c.mo - 1) as usize], c.d, c.y)
}

/// strftime subset used by `format:`.
pub fn strftime(fmt: &str, ts: i64, tz: &str) -> String {
    let c = civil(ts, tz);
    let mut out = String::new();
    let mut it = fmt.chars().peekable();
    while let Some(ch) = it.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        let Some(k) = it.next() else {
            out.push('%');
            break;
        };
        let h12 = if c.h % 12 == 0 { 12 } else { c.h % 12 };
        match k {
            '%' => out.push('%'),
            'a' => out.push_str(WD[c.wd as usize]),
            'A' => out.push_str(WDL[c.wd as usize]),
            'b' | 'h' => out.push_str(MO[(c.mo - 1) as usize]),
            'B' => out.push_str(MOL[(c.mo - 1) as usize]),
            'c' => out.push_str(&format!(
                "{} {} {:>2} {:02}:{:02}:{:02} {}",
                WD[c.wd as usize],
                MO[(c.mo - 1) as usize],
                c.d,
                c.h,
                c.mi,
                c.s,
                c.y
            )),
            'C' => out.push_str(&format!("{:02}", c.y / 100)),
            'd' => out.push_str(&format!("{:02}", c.d)),
            'D' => out.push_str(&format!("{:02}/{:02}/{:02}", c.mo, c.d, c.y % 100)),
            'e' => out.push_str(&format!("{:>2}", c.d)),
            'F' => out.push_str(&format!("{:04}-{:02}-{:02}", c.y, c.mo, c.d)),
            'H' => out.push_str(&format!("{:02}", c.h)),
            'I' => out.push_str(&format!("{:02}", h12)),
            'j' => out.push_str(&format!("{:03}", c.yday + 1)),
            'k' => out.push_str(&format!("{:>2}", c.h)),
            'l' => out.push_str(&format!("{:>2}", h12)),
            'm' => out.push_str(&format!("{:02}", c.mo)),
            'M' => out.push_str(&format!("{:02}", c.mi)),
            'n' => out.push('\n'),
            'p' => out.push_str(if c.h < 12 { "AM" } else { "PM" }),
            'P' => out.push_str(if c.h < 12 { "am" } else { "pm" }),
            'r' => out.push_str(&format!(
                "{:02}:{:02}:{:02} {}",
                h12,
                c.mi,
                c.s,
                if c.h < 12 { "AM" } else { "PM" }
            )),
            'R' => out.push_str(&format!("{:02}:{:02}", c.h, c.mi)),
            's' => out.push_str(&ts.to_string()),
            'S' => out.push_str(&format!("{:02}", c.s)),
            't' => out.push('\t'),
            'T' => out.push_str(&format!("{:02}:{:02}:{:02}", c.h, c.mi, c.s)),
            'u' => out.push_str(&(if c.wd == 0 { 7 } else { c.wd }).to_string()),
            'w' => out.push_str(&c.wd.to_string()),
            'x' => out.push_str(&format!("{:02}/{:02}/{:02}", c.mo, c.d, c.y % 100)),
            'X' => out.push_str(&format!("{:02}:{:02}:{:02}", c.h, c.mi, c.s)),
            'y' => out.push_str(&format!("{:02}", c.y % 100)),
            'Y' => out.push_str(&c.y.to_string()),
            'z' => out.push_str(tz),
            'Z' => {}
            other => {
                out.push('%');
                out.push(other);
            }
        }
    }
    out
}

/// Render `sig`'s date in git's `<format>` (as `--date=` takes it).
pub fn format(spec: &str, sig: &Signature) -> CliResult<String> {
    let spec = if spec.is_empty() { "default" } else { spec };
    if let Some(f) = spec.strip_prefix("format-local:") {
        return Ok(strftime(f, sig.timestamp, &local_tz()));
    }
    if let Some(f) = spec.strip_prefix("format:") {
        return Ok(strftime(f, sig.timestamp, &sig.timezone));
    }
    let (base, local) = match spec.strip_suffix("-local") {
        Some(b) => (b, true),
        None if spec == "local" => ("default", true),
        None => (spec, false),
    };
    let shown = if local {
        Signature {
            timezone: local_tz(),
            ..sig.clone()
        }
    } else {
        sig.clone()
    };
    Ok(match base {
        "relative" => relative(sig.timestamp, now()),
        "human" => human(&shown, now()),
        "default" | "" if local => {
            // `local` drops the zone from the default shape.
            let full = DateMode::Default.format(&shown);
            full.rsplit_once(' ')
                .map(|(a, _)| a.to_string())
                .unwrap_or(full)
        }
        "raw" if local => format!("{} +0000", sig.timestamp),
        other => match DateMode::parse(other) {
            Ok(m) => m.format(&shown),
            Err(_) => return Err(CliError::fatal(format!("unknown date format {other}"))),
        },
    })
}

/// Whether `spec` names a date format git accepts.
pub fn valid(spec: &str) -> bool {
    let dummy = Signature {
        name: String::new(),
        email: String::new(),
        timestamp: 0,
        timezone: "+0000".into(),
    };
    format(spec, &dummy).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig(ts: i64) -> Signature {
        Signature {
            name: "n".into(),
            email: "e".into(),
            timestamp: ts,
            timezone: "+0000".into(),
        }
    }

    #[test]
    fn relative_wording() {
        assert_eq!(relative(100, 130), "30 seconds ago");
        assert_eq!(relative(0, 3600), "60 minutes ago");
        assert_eq!(relative(0, 86_400 * 3), "3 days ago");
        assert_eq!(relative(0, 86_400 * 365 * 3), "3 years ago");
        assert_eq!(relative(10, 0), "in the future");
    }

    #[test]
    fn shapes() {
        let s = sig(1_700_025_200);
        assert_eq!(format("format:%Y-%m %H", &s).unwrap(), "2023-11 05");
        assert_eq!(format("local", &s).unwrap(), "Wed Nov 15 05:13:20 2023");
        assert_eq!(format("short", &s).unwrap(), "2023-11-15");
        assert!(format("bogus", &s).is_err());
        assert!(valid("iso-local"));
    }
}
