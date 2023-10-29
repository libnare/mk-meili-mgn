use std::time::{Duration, SystemTime};
use chrono::{DateTime, Utc};

const TIME2000: i64 = 946_684_800_000;

pub(crate) fn parse(id: String) -> DateTime<Utc> {
    let time = i64::from_str_radix(&id[..8], 36).unwrap() + TIME2000;
    let system_time = SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_millis(time as u64))
        .ok_or("Failed to parse AID: Invalid Date").unwrap();

    DateTime::<Utc>::from(system_time)
}