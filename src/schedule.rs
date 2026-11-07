use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

/// A single scheduled event extracted from the spreadsheet.
#[derive(Debug, Clone)]
pub struct ScheduleEvent {
    pub title: String,
    pub speaker: Option<String>,
    pub format: Option<String>,
    pub stream_url: Option<String>,
    pub recording_url: Option<String>,
    pub date: NaiveDate,
    /// `None` means the row had no (parseable) time range, so the event is all-day.
    pub time_range: Option<(NaiveTime, NaiveTime)>,
}

impl ScheduleEvent {
    pub fn starts_at(&self, tz_offset_hours: i32) -> DateTime<Utc> {
        let time = self
            .time_range
            .map(|(start, _)| start)
            .unwrap_or_else(|| NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        to_utc(self.date, time, tz_offset_hours)
    }

    pub fn ends_at(&self, tz_offset_hours: i32) -> DateTime<Utc> {
        let time = self
            .time_range
            .map(|(_, end)| end)
            .unwrap_or_else(|| NaiveTime::from_hms_opt(23, 59, 0).unwrap());
        to_utc(self.date, time, tz_offset_hours)
    }

    pub fn is_all_day(&self) -> bool {
        self.time_range.is_none()
    }

    /// Deterministic identifier used to derive a stable UID, so that
    /// re-fetching the same source data does not spawn duplicate events in
    /// subscribers' calendars.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}|{:?}|{}",
            self.date,
            self.title,
            self.time_range,
            self.speaker.as_deref().unwrap_or("")
        )
    }
}

fn to_utc(date: NaiveDate, time: NaiveTime, tz_offset_hours: i32) -> DateTime<Utc> {
    use chrono::{FixedOffset, TimeZone};
    let offset = FixedOffset::east_opt(tz_offset_hours * 3600)
        .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
    let naive = date.and_time(time);
    offset
        .from_local_datetime(&naive)
        .single()
        .unwrap_or_else(|| offset.from_utc_datetime(&naive))
        .with_timezone(&Utc)
}
