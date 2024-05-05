//! A type for dates.
//!
//! This module uses [`time`] under the hood.

use std::time::SystemTime;

use thiserror::Error;
pub use time::Month;
use time::{
    macros::{format_description, time},
    Date, OffsetDateTime, PrimitiveDateTime, Time,
};

/// An UTC date.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct DateTime(OffsetDateTime);

impl DateTime {
    /// Create a date from the year, month, day, hour, minute, second, and
    /// nanosecond components.
    pub fn new(
        year: i32,
        month: Month,
        day: u8,
        hour: u8,
        minute: u8,
        second: u8,
        nanosecond: u32,
    ) -> Result<Self, DateTimeError> {
        Ok(Self(OffsetDateTime::new_utc(
            Date::from_calendar_date(year, month, day)?,
            Time::from_hms_nano(hour, minute, second, nanosecond)?,
        )))
    }

    /// Parse a date string.
    pub fn parse(input: &str) -> Result<Self, DateTimeError> {
        use time::format_description::well_known::{Iso8601, Rfc2822, Rfc3339};

        let dt = PrimitiveDateTime::parse(
            input,
            format_description!("[year]-[month]-[day] [hour]:[minute][optional [:[second]]]"),
        )
        .map(|dt| dt.assume_utc())
        .or_else(|_| {
            Date::parse(input, format_description!("[year]-[month]-[day]"))
                .map(|d| d.with_time(time!(0:00)).assume_utc())
        })
        .or_else(|_| OffsetDateTime::parse(input, &Iso8601::DEFAULT))
        .or_else(|_| OffsetDateTime::parse(input, &Rfc3339))
        .or_else(|_| OffsetDateTime::parse(input, &Rfc2822))
        .map_err(|_| DateTimeError::Parse)?;

        Ok(Self(dt))
    }
}

impl ToString for DateTime {
    fn to_string(&self) -> String {
        use time::format_description::well_known::Iso8601;
        self.0.format(&Iso8601::DEFAULT).unwrap()
    }
}

impl From<SystemTime> for DateTime {
    fn from(value: std::time::SystemTime) -> Self {
        Self(value.into())
    }
}

/// Date error.
#[derive(Debug, Error)]
pub enum DateTimeError {
    /// Component out of range error.
    #[error(transparent)]
    ComponentRange(#[from] time::error::ComponentRange),
    /// Parse error.
    #[error("failed to parse date and time")]
    Parse,
    /// [`time`] error.
    #[error(transparent)]
    Time(#[from] time::Error),
}

#[cfg(test)]
mod tests {
    use super::{DateTime, Month};

    #[test]
    fn parse() {
        // yyyy-mm-dd hh:mm:ss
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 23, 20, 50, 0).unwrap(),
            DateTime::parse("1985-04-12 23:20:50").unwrap()
        );

        // yyyy-mm-dd hh:mm
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 23, 20, 0, 0).unwrap(),
            DateTime::parse("1985-04-12 23:20").unwrap()
        );

        // yyyy-mm-dd
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 0, 0, 0, 0).unwrap(),
            DateTime::parse("1985-04-12").unwrap()
        );

        // ISO 8601
        assert_eq!(
            DateTime::new(1997, Month::November, 12, 15, 55, 6, 0).unwrap(),
            DateTime::parse("1997-11-12T09:55:06.000000000-06:00").unwrap()
        );

        // RFC 3339
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 23, 20, 50, 520_000_000).unwrap(),
            DateTime::parse("1985-04-12T23:20:50.52Z").unwrap()
        );

        // RFC 2822
        assert_eq!(
            DateTime::new(1993, Month::June, 12, 13, 25, 19, 0).unwrap(),
            DateTime::parse("Sat, 12 Jun 1993 13:25:19 GMT").unwrap()
        );
    }

    #[test]
    fn to_string() {
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 23, 20, 50, 520_000_000)
                .unwrap()
                .to_string(),
            "1985-04-12T23:20:50.520000000Z"
        );
    }
}
