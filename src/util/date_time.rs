//! A type for dates.
//!
//! This module uses [`time`] under the hood.

use std::time::SystemTime;

use anyhow::Result;
use serde::{Deserialize, Serialize};
pub use time::Month;
use time::{
    Date, OffsetDateTime, PrimitiveDateTime, Time,
    macros::{format_description, time},
};

time::serde::format_description!(
    format,
    OffsetDateTime,
    "[year]-[month]-[day] [hour]:[minute][optional [:[second]]]"
);

/// An UTC date.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Serialize)]
pub struct DateTime(#[serde(with = "format")] OffsetDateTime);

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
    ) -> Result<Self> {
        Ok(Self(OffsetDateTime::new_utc(
            Date::from_calendar_date(year, month, day)?,
            Time::from_hms_nano(hour, minute, second, nanosecond)?,
        )))
    }
}

impl std::fmt::Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use time::format_description::well_known::Iso8601;
        let s = self.0.format(&Iso8601::DEFAULT).unwrap();
        write!(f, "{}", s)
    }
}

impl From<SystemTime> for DateTime {
    fn from(value: std::time::SystemTime) -> Self {
        Self(value.into())
    }
}

impl TryFrom<&str> for DateTime {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        use time::format_description::well_known::{Iso8601, Rfc2822, Rfc3339};

        let dt = PrimitiveDateTime::parse(
            value,
            format_description!("[year]-[month]-[day] [hour]:[minute][optional [:[second]]]"),
        )
        .map(|dt| dt.assume_utc())
        .or_else(|_| {
            Date::parse(value, format_description!("[year]-[month]-[day]"))
                .map(|d| d.with_time(time!(0:00)).assume_utc())
        })
        .or_else(|_| OffsetDateTime::parse(value, &Iso8601::DEFAULT))
        .or_else(|_| OffsetDateTime::parse(value, &Rfc3339))
        .or_else(|_| OffsetDateTime::parse(value, &Rfc2822))?;

        Ok(Self(dt))
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;
        impl serde::de::Visitor<'_> for Visitor {
            type Value = DateTime;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a datetime")
            }

            fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                DateTime::try_from(v).map_err(E::custom)
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::{DateTime, Month};

    #[test]
    fn parse() {
        // yyyy-mm-dd hh:mm:ss
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 23, 20, 50, 0).unwrap(),
            DateTime::try_from("1985-04-12 23:20:50").unwrap()
        );

        // yyyy-mm-dd hh:mm
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 23, 20, 0, 0).unwrap(),
            DateTime::try_from("1985-04-12 23:20").unwrap()
        );

        // yyyy-mm-dd
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 0, 0, 0, 0).unwrap(),
            DateTime::try_from("1985-04-12").unwrap()
        );

        // ISO 8601
        assert_eq!(
            DateTime::new(1997, Month::November, 12, 15, 55, 6, 0).unwrap(),
            DateTime::try_from("1997-11-12T09:55:06.000000000-06:00").unwrap()
        );

        // RFC 3339
        assert_eq!(
            DateTime::new(1985, Month::April, 12, 23, 20, 50, 520_000_000).unwrap(),
            DateTime::try_from("1985-04-12T23:20:50.52Z").unwrap()
        );

        // RFC 2822
        assert_eq!(
            DateTime::new(1993, Month::June, 12, 13, 25, 19, 0).unwrap(),
            DateTime::try_from("Sat, 12 Jun 1993 13:25:19 GMT").unwrap()
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
