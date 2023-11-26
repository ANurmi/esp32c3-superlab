use chrono::{Datelike, TimeZone, Timelike, Utc};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UtcDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub nanoseconds: u32,
}

impl From<chrono::DateTime<Utc>> for UtcDateTime {
    fn from(dt: chrono::DateTime<Utc>) -> Self {
        Self {
            year: dt.year(),
            month: dt.month(),
            day: dt.day(),
            hour: dt.hour(),
            minute: dt.minute(),
            second: dt.second(),
            nanoseconds: dt.nanosecond(),
        }
    }
}

impl From<UtcDateTime> for chrono::DateTime<Utc> {
    fn from(value: UtcDateTime) -> Self {
        Utc.with_ymd_and_hms(
            value.year,
            value.month,
            value.day,
            value.hour,
            value.minute,
            value.second,
        )
        .unwrap()
    }
}
