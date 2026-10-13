//! Typed HL7 dates, times, and timestamps.
//!
//! The HL7 `DT`, `TM`, and `TS`/`DTM` datatypes carry variable precision
//! (`2024` vs `20240101` vs `20240101120000`), fractional seconds, and UTC
//! offsets. These types preserve all three, so values round-trip exactly:
//!
//! ```
//! use vyna_hl7_core::datetime::Hl7DateTime;
//!
//! let ts: Hl7DateTime = "20240101120000.123+0800".parse().unwrap();
//! assert_eq!(ts.encode(), "20240101120000.123+0800");
//! assert_eq!(ts.to_utc().to_rfc3339(), "2024-01-01T04:00:00.123+00:00");
//! ```
//!
//! Equality is structural: values with different precision are not equal,
//! because in HL7 `2024` (a year) and `20240101` (a day) mean different
//! things.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};

use crate::error::{Error, Result};

/// Precision of an [`Hl7Date`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DatePrecision {
    /// `YYYY`.
    Year,
    /// `YYYYMM`.
    Month,
    /// `YYYYMMDD`.
    Day,
}

/// Precision of an [`Hl7Time`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimePrecision {
    /// `HH`.
    Hour,
    /// `HHMM`.
    Minute,
    /// `HHMMSS`, optionally with fractional seconds.
    Second,
}

/// Precision of an [`Hl7DateTime`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TsPrecision {
    /// `YYYY`.
    Year,
    /// `YYYYMM`.
    Month,
    /// `YYYYMMDD`.
    Day,
    /// `YYYYMMDDHH`.
    Hour,
    /// `YYYYMMDDHHMM`.
    Minute,
    /// `YYYYMMDDHHMMSS`, optionally with fractional seconds.
    Second,
}

/// An HL7 `DT` value: a date with known precision.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hl7Date {
    date: NaiveDate,
    precision: DatePrecision,
}

impl Hl7Date {
    /// Parse `YYYY[MM[DD]]`.
    pub fn parse(s: &str) -> Result<Self> {
        let (date, precision) = match s.len() {
            4 => (parse_ymd(s, 1, 1, "DT")?, DatePrecision::Year),
            6 => {
                let month = parse_two(s, 4, "DT month")?;
                (parse_ymd(&s[0..4], month, 1, "DT")?, DatePrecision::Month)
            }
            8 => {
                let month = parse_two(s, 4, "DT month")?;
                let day = parse_two(s, 6, "DT day")?;
                (parse_ymd(&s[0..4], month, day, "DT")?, DatePrecision::Day)
            }
            len => {
                return Err(Error::parse(format!(
                    "DT must be YYYY, YYYYMM, or YYYYMMDD (4, 6, or 8 digits), got {len} characters"
                )));
            }
        };
        Ok(Self { date, precision })
    }

    /// The calendar date (month/day default to 1 below the precision).
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// How precise the original value was.
    pub fn precision(&self) -> DatePrecision {
        self.precision
    }

    /// Encode back to HL7, preserving precision.
    pub fn encode(&self) -> String {
        match self.precision {
            DatePrecision::Year => self.date.format("%Y").to_string(),
            DatePrecision::Month => self.date.format("%Y%m").to_string(),
            DatePrecision::Day => self.date.format("%Y%m%d").to_string(),
        }
    }
}

impl FromStr for Hl7Date {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl fmt::Display for Hl7Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

/// An HL7 `TM` value: a time of day with precision, fractional seconds, and
/// an optional UTC offset.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hl7Time {
    time: NaiveTime,
    precision: TimePrecision,
    /// Fractional-second digits (0 when absent), preserved for round-trips.
    frac_digits: u8,
    /// Minutes east of UTC (e.g. `+0800` is 480).
    offset_minutes: Option<i32>,
}

impl Hl7Time {
    /// Parse `HH[MM[SS[.S...]]][+/-ZZZZ]`.
    pub fn parse(s: &str) -> Result<Self> {
        let (main, offset) = split_offset(s, "TM")?;
        let (digits, frac) = split_fraction(main, "TM")?;
        let (hour, minute, second, precision) = match digits.len() {
            2 => (parse_two(digits, 0, "TM hour")?, 0, 0, TimePrecision::Hour),
            4 => (
                parse_two(digits, 0, "TM hour")?,
                parse_two(digits, 2, "TM minute")?,
                0,
                TimePrecision::Minute,
            ),
            6 => (
                parse_two(digits, 0, "TM hour")?,
                parse_two(digits, 2, "TM minute")?,
                parse_two(digits, 4, "TM second")?,
                TimePrecision::Second,
            ),
            len => {
                return Err(Error::parse(format!(
                    "TM must be HH, HHMM, or HHMMSS (2, 4, or 6 digits), got {len} characters"
                )));
            }
        };
        if frac.is_some() && precision != TimePrecision::Second {
            return Err(Error::parse(
                "TM fractional seconds require seconds precision (HHMMSS.S)".to_string(),
            ));
        }
        let nanos = frac_to_nanos(frac)?;
        let time = NaiveTime::from_hms_nano_opt(hour, minute, second, nanos).ok_or_else(|| {
            Error::parse(format!(
                "TM has out-of-range fields: '{}'",
                truncate_value(s)
            ))
        })?;
        Ok(Self {
            time,
            precision,
            frac_digits: frac.map(|f| f.len() as u8).unwrap_or(0),
            offset_minutes: offset,
        })
    }

    /// Number of fractional-second digits (0 when the value has none).
    pub fn frac_digits(&self) -> u8 {
        self.frac_digits
    }

    /// The time of day (fields below the precision read as zero).
    pub fn time(&self) -> NaiveTime {
        self.time
    }

    /// How precise the original value was.
    pub fn precision(&self) -> TimePrecision {
        self.precision
    }

    /// Minutes east of UTC, when the value carries an offset.
    pub fn offset_minutes(&self) -> Option<i32> {
        self.offset_minutes
    }

    /// Encode back to HL7, preserving precision, fraction, and offset.
    pub fn encode(&self) -> String {
        let mut out = match self.precision {
            TimePrecision::Hour => self.time.format("%H").to_string(),
            TimePrecision::Minute => self.time.format("%H%M").to_string(),
            TimePrecision::Second => {
                let mut base = self.time.format("%H%M%S").to_string();
                if self.frac_digits > 0 {
                    base.push('.');
                    base.push_str(&format_nanos(self.time.nanosecond(), self.frac_digits));
                }
                base
            }
        };
        if let Some(offset) = self.offset_minutes {
            out.push_str(&format_offset(offset));
        }
        out
    }
}

impl FromStr for Hl7Time {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl fmt::Display for Hl7Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

/// An HL7 `TS`/`DTM` value: a timestamp with precision, fractional seconds,
/// and an optional UTC offset.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hl7DateTime {
    datetime: NaiveDateTime,
    precision: TsPrecision,
    /// Fractional-second digits (0 when absent), preserved for round-trips.
    frac_digits: u8,
    /// Minutes east of UTC (e.g. `+0800` is 480).
    offset_minutes: Option<i32>,
}

impl Hl7DateTime {
    /// Parse `YYYY[MM[DD[HH[MM[SS[.S...]]]]]][+/-ZZZZ]`.
    pub fn parse(s: &str) -> Result<Self> {
        let (main, offset) = split_offset(s, "TS")?;
        let (digits, frac) = split_fraction(main, "TS")?;
        let (datetime, precision) = match digits.len() {
            4 => (
                parse_ymd(digits, 1, 1, "TS")?.and_hms_opt(0, 0, 0),
                TsPrecision::Year,
            ),
            6 => {
                let month = parse_two(digits, 4, "TS month")?;
                (
                    parse_ymd(&digits[0..4], month, 1, "TS")?.and_hms_opt(0, 0, 0),
                    TsPrecision::Month,
                )
            }
            8 => {
                let date = parse_full_date(digits, "TS")?;
                (date.and_hms_opt(0, 0, 0), TsPrecision::Day)
            }
            10 | 12 | 14 => {
                let date_prefix = digits.get(0..8).ok_or_else(|| {
                    Error::parse(format!(
                        "TS date part is not 8 digits: '{}'",
                        truncate_value(digits)
                    ))
                })?;
                let date = parse_full_date(date_prefix, "TS")?;
                let hour = parse_two(digits, 8, "TS hour")?;
                let minute = if digits.len() >= 12 {
                    parse_two(digits, 10, "TS minute")?
                } else {
                    0
                };
                let second = if digits.len() == 14 {
                    parse_two(digits, 12, "TS second")?
                } else {
                    0
                };
                let precision = match digits.len() {
                    10 => TsPrecision::Hour,
                    12 => TsPrecision::Minute,
                    _ => TsPrecision::Second,
                };
                let nanos = frac_to_nanos(frac)?;
                (
                    date.and_hms_nano_opt(hour, minute, second, nanos),
                    precision,
                )
            }
            len => {
                return Err(Error::parse(format!(
                    "TS must be 4, 6, 8, 10, 12, or 14 digits, got {len} characters"
                )));
            }
        };
        if frac.is_some() && precision != TsPrecision::Second {
            return Err(Error::parse(
                "TS fractional seconds require seconds precision (YYYYMMDDHHMMSS.S)".to_string(),
            ));
        }
        let datetime = datetime.ok_or_else(|| {
            Error::parse(format!(
                "TS has out-of-range fields: '{}'",
                truncate_value(s)
            ))
        })?;
        Ok(Self {
            datetime,
            precision,
            frac_digits: frac.map(|f| f.len() as u8).unwrap_or(0),
            offset_minutes: offset,
        })
    }

    /// Number of fractional-second digits (0 when the value has none).
    pub fn frac_digits(&self) -> u8 {
        self.frac_digits
    }

    /// The local date and time (fields below the precision read as zero).
    pub fn datetime(&self) -> NaiveDateTime {
        self.datetime
    }

    /// How precise the original value was.
    pub fn precision(&self) -> TsPrecision {
        self.precision
    }

    /// Minutes east of UTC, when the value carries an offset.
    pub fn offset_minutes(&self) -> Option<i32> {
        self.offset_minutes
    }

    /// The instant in UTC. Values without an offset are taken as already UTC.
    pub fn to_utc(&self) -> DateTime<Utc> {
        match self.offset_minutes {
            Some(minutes) => {
                let offset = FixedOffset::east_opt(minutes * 60)
                    .unwrap_or(FixedOffset::east_opt(0).unwrap());
                offset
                    .from_local_datetime(&self.datetime)
                    .single()
                    .map(|dt| dt.to_utc())
                    .unwrap_or_else(|| DateTime::from_naive_utc_and_offset(self.datetime, Utc))
            }
            None => DateTime::from_naive_utc_and_offset(self.datetime, Utc),
        }
    }

    /// Encode back to HL7, preserving precision, fraction, and offset.
    pub fn encode(&self) -> String {
        let mut out = match self.precision {
            TsPrecision::Year => self.datetime.format("%Y").to_string(),
            TsPrecision::Month => self.datetime.format("%Y%m").to_string(),
            TsPrecision::Day => self.datetime.format("%Y%m%d").to_string(),
            TsPrecision::Hour => self.datetime.format("%Y%m%d%H").to_string(),
            TsPrecision::Minute => self.datetime.format("%Y%m%d%H%M").to_string(),
            TsPrecision::Second => {
                let mut base = self.datetime.format("%Y%m%d%H%M%S").to_string();
                if self.frac_digits > 0 {
                    base.push('.');
                    base.push_str(&format_nanos(self.datetime.nanosecond(), self.frac_digits));
                }
                base
            }
        };
        if let Some(offset) = self.offset_minutes {
            out.push_str(&format_offset(offset));
        }
        out
    }
}

impl FromStr for Hl7DateTime {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl fmt::Display for Hl7DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

/// Parse a 2-digit field at a byte offset.
fn parse_two(s: &str, at: usize, what: &str) -> Result<u32> {
    s.get(at..at + 2)
        .filter(|part| part.bytes().all(|b| b.is_ascii_digit()))
        .and_then(|part| part.parse::<u32>().ok())
        .ok_or_else(|| {
            Error::parse(format!(
                "{what} is not two digits in '{}'",
                truncate_value(s)
            ))
        })
}

/// Parse `YYYY` plus month/day, rejecting impossible dates.
fn parse_ymd(year: &str, month: u32, day: u32, what: &str) -> Result<NaiveDate> {
    let year_num = year
        .parse::<i32>()
        .ok()
        .filter(|_| year.bytes().all(|b| b.is_ascii_digit()))
        .ok_or_else(|| {
            Error::parse(format!(
                "{what} year is not four digits in '{}'",
                truncate_value(year)
            ))
        })?;
    NaiveDate::from_ymd_opt(year_num, month, day).ok_or_else(|| {
        Error::parse(format!(
            "{what} has an impossible date: '{}-{month}-{day}'",
            truncate_value(year)
        ))
    })
}

/// Parse a full `YYYYMMDD` date.
fn parse_full_date(s: &str, what: &str) -> Result<NaiveDate> {
    let month = parse_two(s, 4, &format!("{what} month"))?;
    let day = parse_two(s, 6, &format!("{what} day"))?;
    parse_ymd(&s[0..4], month, day, what)
}

/// Split a trailing `[+/-ZZZZ]` offset. Returns the main part and the offset
/// in minutes east of UTC.
fn split_offset<'a>(s: &'a str, what: &str) -> Result<(&'a str, Option<i32>)> {
    let at = s.find(['+', '-']);
    let Some(at) = at else {
        return Ok((s, None));
    };
    let (main, offset) = s.split_at(at);
    if main.is_empty() {
        return Err(Error::parse(format!(
            "{what} value is missing before the offset: '{}'",
            truncate_value(s)
        )));
    }
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let digits = &offset[1..];
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::parse(format!(
            "{what} offset must be [+/-]HHMM (4 digits), got '{}'",
            truncate_value(offset)
        )));
    }
    let hours: i32 = digits[0..2].parse().unwrap();
    let minutes: i32 = digits[2..4].parse().unwrap();
    if hours > 23 || minutes > 59 {
        return Err(Error::parse(format!(
            "{what} offset is out of range: '{}'",
            truncate_value(offset)
        )));
    }
    Ok((main, Some(sign * (hours * 60 + minutes))))
}

/// Split fractional seconds (`.S...`, 1-9 digits).
fn split_fraction<'a>(s: &'a str, what: &str) -> Result<(&'a str, Option<&'a str>)> {
    let Some(at) = s.find('.') else {
        return Ok((s, None));
    };
    let (main, frac) = (&s[..at], &s[at + 1..]);
    if main.is_empty() {
        return Err(Error::parse(format!(
            "{what} value is missing before '.': '{}'",
            truncate_value(s)
        )));
    }
    if frac.is_empty() || frac.len() > 9 || !frac.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Error::parse(format!(
            "{what} fractional seconds must be 1-9 digits, got '{}'",
            truncate_value(frac)
        )));
    }
    if frac.contains('.') {
        return Err(Error::parse(format!(
            "{what} has more than one decimal point: '{}'",
            truncate_value(s)
        )));
    }
    Ok((main, Some(frac)))
}

/// Scale fraction digits to nanoseconds.
fn frac_to_nanos(frac: Option<&str>) -> Result<u32> {
    match frac {
        None => Ok(0),
        Some(f) => {
            let mut nanos = f.parse::<u32>().map_err(|_| {
                Error::parse(format!(
                    "fractional seconds overflow: '{}'",
                    truncate_value(f)
                ))
            })?;
            for _ in f.len()..9 {
                nanos *= 10;
            }
            Ok(nanos)
        }
    }
}

/// Format nanoseconds back to the original digit count.
fn format_nanos(nanos: u32, digits: u8) -> String {
    let full = format!("{nanos:09}");
    full[..digits as usize].to_string()
}

/// Format minutes-east as `[+/-]HHMM`.
fn format_offset(minutes: i32) -> String {
    let sign = if minutes < 0 { '-' } else { '+' };
    let abs = minutes.abs();
    format!("{sign}{:02}{:02}", abs / 60, abs % 60)
}

/// Cap an echoed value at 32 chars plus length; short values stay verbatim.
fn truncate_value(value: &str) -> String {
    const MAX: usize = 32;
    let count = value.chars().count();
    if count <= MAX {
        value.to_string()
    } else {
        let head: String = value.chars().take(MAX).collect();
        format!("{head}... ({} chars total)", count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_parses_all_precisions_and_round_trips() {
        for (input, precision) in [
            ("2024", DatePrecision::Year),
            ("202403", DatePrecision::Month),
            ("20240315", DatePrecision::Day),
        ] {
            let date: Hl7Date = input.parse().unwrap();
            assert_eq!(date.precision(), precision);
            assert_eq!(date.encode(), input);
            assert_eq!(date.to_string(), input);
        }
        assert_eq!(
            "2024".parse::<Hl7Date>().unwrap().date(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap()
        );
    }

    #[test]
    fn date_rejects_impossible_values() {
        assert!("20241301".parse::<Hl7Date>().is_err());
        assert!("20240230".parse::<Hl7Date>().is_err());
        assert!("2024031".parse::<Hl7Date>().is_err());
        assert!("2024abcd".parse::<Hl7Date>().is_err());
        assert!("".parse::<Hl7Date>().is_err());
    }

    #[test]
    fn time_parses_all_precisions_and_round_trips() {
        for (input, precision) in [
            ("14", TimePrecision::Hour),
            ("1430", TimePrecision::Minute),
            ("143000", TimePrecision::Second),
            ("143000.123", TimePrecision::Second),
            ("143000+0800", TimePrecision::Second),
            ("143000.123456789-0530", TimePrecision::Second),
        ] {
            let time: Hl7Time = input.parse().unwrap();
            assert_eq!(time.precision(), precision, "{input}");
            assert_eq!(time.encode(), input, "{input}");
        }
        let time: Hl7Time = "143000.5-0530".parse().unwrap();
        assert_eq!(time.offset_minutes(), Some(-330));
        assert_eq!(
            time.time(),
            NaiveTime::from_hms_milli_opt(14, 30, 0, 500).unwrap()
        );
    }

    #[test]
    fn time_rejects_bad_values() {
        assert!("2530".parse::<Hl7Time>().is_err());
        assert!("1460".parse::<Hl7Time>().is_err());
        assert!("143060".parse::<Hl7Time>().is_err());
        assert!("14300".parse::<Hl7Time>().is_err());
        assert!("1430.5".parse::<Hl7Time>().is_err());
        assert!("143000+2500".parse::<Hl7Time>().is_err());
        assert!("143000+0860".parse::<Hl7Time>().is_err());
    }

    #[test]
    fn datetime_round_trips_full_form() {
        let ts: Hl7DateTime = "20240101120000.123+0800".parse().unwrap();
        assert_eq!(ts.precision(), TsPrecision::Second);
        assert_eq!(ts.offset_minutes(), Some(480));
        assert_eq!(ts.encode(), "20240101120000.123+0800");
        assert_eq!(ts.to_utc().to_rfc3339(), "2024-01-01T04:00:00.123+00:00");
    }

    #[test]
    fn datetime_tracks_coarse_precisions() {
        for (input, precision) in [
            ("2024", TsPrecision::Year),
            ("202403", TsPrecision::Month),
            ("20240315", TsPrecision::Day),
            ("2024031514", TsPrecision::Hour),
            ("202403151430", TsPrecision::Minute),
            ("20240315143000", TsPrecision::Second),
        ] {
            let ts: Hl7DateTime = input.parse().unwrap();
            assert_eq!(ts.precision(), precision, "{input}");
            assert_eq!(ts.encode(), input, "{input}");
            assert_eq!(ts.offset_minutes(), None);
        }
    }

    #[test]
    fn datetime_rejects_bad_values() {
        // Bad lengths and non-digits.
        assert!("202403151".parse::<Hl7DateTime>().is_err());
        assert!("2024031x".parse::<Hl7DateTime>().is_err());
        // Impossible fields.
        assert!("20241301120000".parse::<Hl7DateTime>().is_err());
        assert!("20240101250000".parse::<Hl7DateTime>().is_err());
        // Fraction requires seconds.
        assert!("20240315.5".parse::<Hl7DateTime>().is_err());
        assert!("202403151430.5".parse::<Hl7DateTime>().is_err());
        // Offsets are validated, not ignored.
        assert!("20240101120000+9999".parse::<Hl7DateTime>().is_err());
        assert!("20240101120000+ABCD".parse::<Hl7DateTime>().is_err());
        assert!("20240101120000+080".parse::<Hl7DateTime>().is_err());
    }

    #[test]
    fn precision_affects_equality() {
        let year: Hl7DateTime = "2024".parse().unwrap();
        let day: Hl7DateTime = "20240101".parse().unwrap();
        assert_ne!(year, day);
        let same: Hl7DateTime = "2024".parse().unwrap();
        assert_eq!(year, same);
    }

    #[test]
    fn non_ascii_values_return_errors_without_panicking() {
        // Byte length matches a valid precision but a char boundary does not.
        assert!("1234567é1".parse::<Hl7DateTime>().is_err());
        assert!("1234567é123".parse::<Hl7DateTime>().is_err());
        assert!("1234567é12345".parse::<Hl7DateTime>().is_err());
        assert!("2024é115120000".parse::<Hl7DateTime>().is_err());
        assert!("2024é1".parse::<Hl7Date>().is_err());
        assert!("20é40315".parse::<Hl7Date>().is_err());
        assert!("1é3000".parse::<Hl7Time>().is_err());
        assert!("é".parse::<Hl7DateTime>().is_err());
    }

    #[test]
    fn long_values_are_truncated_in_errors() {
        let long = format!("20240101120000-{}", "A".repeat(100));
        let err = long.parse::<Hl7DateTime>().unwrap_err().to_string();
        assert!(!err.contains(&"A".repeat(100)), "{err}");
        assert!(err.contains("chars total"), "{err}");

        let frac = format!("20240101120000.{}", "1".repeat(100));
        let err = frac.parse::<Hl7DateTime>().unwrap_err().to_string();
        assert!(!err.contains(&"1".repeat(100)), "{err}");
        assert!(err.contains("chars total"), "{err}");
    }
}
