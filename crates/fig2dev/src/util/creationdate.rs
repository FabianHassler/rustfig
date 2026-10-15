//! The creation date some drivers write into their output, honouring
//! `SOURCE_DATE_EPOCH` for reproducible builds.
//!
//! Differences from C: without `SOURCE_DATE_EPOCH`, C formats the current
//! time in the local time zone; this port has no time-zone data and uses
//! UTC. On wasm32 there is neither an environment nor a clock
//! (`SystemTime::now` panics there), so no date is given and the drivers
//! leave it out, as C does when `strftime` fails.
//
// Ported from fig2dev creationdate.c (xfig licence, see LICENSE-xfig).

use crate::gendev::Ctx;

/// Room C reserves for the date (`CREATION_TIME_LEN`).
pub const CREATION_TIME_LEN: usize = 36;

/// The latest time `gmtime` would format here: 9999-12-31 23:59:59 UTC.
const MAX_EPOCH: u64 = 253_402_300_799;

/// A broken-down UTC time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateTime {
    pub year: i64,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl DateTime {
    /// The UTC time `secs` seconds after 1970-01-01 (`gmtime`).
    pub fn from_epoch(secs: u64) -> DateTime {
        let days = (secs / 86_400) as i64;
        let rem = secs % 86_400;
        // Howard Hinnant's civil_from_days.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = yoe + era * 400 + i64::from(month <= 2);
        DateTime {
            year,
            month,
            day,
            hour: (rem / 3600) as u32,
            minute: (rem / 60 % 60) as u32,
            second: (rem % 60) as u32,
        }
    }

    /// `strftime("%F %H:%M:%S")`.
    pub fn iso(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// `strftime("D:%Y%m%d%H%M%S")`, the pdfmark form.
    pub fn pdfmark(&self) -> String {
        format!(
            "D:{:04}{:02}{:02}{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Parses `SOURCE_DATE_EPOCH` as C does (`strtoull`, base 10), giving the
/// message C prints on stderr for a bad value.
pub fn parse_source_date_epoch(s: &str) -> Result<u64, String> {
    const PRE: &str = "Environment variable SOURCE_DATE_EPOCH:";
    let t = s.trim_start();
    let t = t.strip_prefix('+').unwrap_or(t);
    let digits = t.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Err(format!("{PRE} No digits were found: {s}"));
    }
    let (num, rest) = t.split_at(digits);
    let Ok(v) = num.parse::<u64>() else {
        return Err(format!("{PRE} strtoull: Result too large"));
    };
    if !rest.is_empty() {
        return Err(format!("{PRE} Trailing garbage: {rest}"));
    }
    if v > MAX_EPOCH {
        return Err(format!(
            "{PRE} value must be smaller than or equal to: {MAX_EPOCH} but was found to be: {v} "
        ));
    }
    Ok(v)
}

/// The time to use: `SOURCE_DATE_EPOCH` if set and valid (messages about a
/// bad value go to `ctx`), otherwise now; `None` where there is no clock.
pub fn creation_time(ctx: &mut Ctx) -> Option<DateTime> {
    if let Ok(s) = std::env::var("SOURCE_DATE_EPOCH") {
        match parse_source_date_epoch(&s) {
            Ok(v) => return Some(DateTime::from_epoch(v)),
            Err(msg) => ctx.warn(msg),
        }
    }
    now()
}

#[cfg(not(target_arch = "wasm32"))]
fn now() -> Option<DateTime> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(DateTime::from_epoch(secs.min(MAX_EPOCH)))
}

#[cfg(target_arch = "wasm32")]
fn now() -> Option<DateTime> {
    None
}

/// `creation_date`: "YYYY-MM-DD HH:MM:SS", or `None` (C returns 0).
pub fn creation_date(ctx: &mut Ctx) -> Option<String> {
    creation_time(ctx).map(|t| t.iso())
}

/// `creation_date_pdfmark`: "D:YYYYMMDDHHMMSS", or `None`.
pub fn creation_date_pdfmark(ctx: &mut Ctx) -> Option<String> {
    creation_time(ctx).map(|t| t.pdfmark())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_to_date() {
        assert_eq!(DateTime::from_epoch(0).iso(), "1970-01-01 00:00:00");
        assert_eq!(DateTime::from_epoch(951_782_400).iso(), "2000-02-29 00:00:00");
        assert_eq!(DateTime::from_epoch(1_700_000_000).iso(), "2023-11-14 22:13:20");
        assert_eq!(DateTime::from_epoch(1_700_000_000).pdfmark(), "D:20231114221320");
        assert_eq!(DateTime::from_epoch(MAX_EPOCH).iso(), "9999-12-31 23:59:59");
        assert!(DateTime::from_epoch(MAX_EPOCH).iso().len() < CREATION_TIME_LEN);
    }

    #[test]
    fn parse_epoch() {
        assert_eq!(parse_source_date_epoch("1700000000"), Ok(1_700_000_000));
        assert_eq!(parse_source_date_epoch(" +12"), Ok(12));
        assert!(parse_source_date_epoch("").unwrap_err().contains("No digits"));
        assert!(parse_source_date_epoch("12x")
            .unwrap_err()
            .contains("Trailing garbage: x"));
        assert!(parse_source_date_epoch("99999999999999999999999").is_err());
        assert!(parse_source_date_epoch("253402300800").is_err());
    }

    #[test]
    fn now_has_a_date() {
        let mut ctx = Ctx::default();
        let d = creation_date(&mut ctx).unwrap();
        assert_eq!(d.len(), 19);
    }
}
