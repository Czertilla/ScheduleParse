use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use icalendar::{Calendar, Component, Event, EventLike};

use crate::schedule::ScheduleEvent;

/// Builds an RFC 5545 calendar document from the parsed schedule rows.
///
/// UIDs are derived deterministically from each event's content so that
/// re-fetching the same source data on the next worker tick does not create
/// duplicate entries in subscribers' calendars - only genuine changes (a
/// moved time, a renamed block) produce a different UID, which calendar
/// apps treat as the old event disappearing and a new one appearing.
pub fn build_calendar(events: &[ScheduleEvent], calendar_name: &str, tz_offset_hours: i32) -> String {
    let mut calendar = Calendar::new();
    calendar.name(calendar_name);

    for ev in events {
        let mut component = Event::new();
        component.summary(&ev.title);
        component.uid(&stable_uid(ev));

        if ev.is_all_day() {
            component.all_day(ev.date);
        } else {
            component.starts(ev.starts_at(tz_offset_hours));
            component.ends(ev.ends_at(tz_offset_hours));
        }

        if let Some(format) = &ev.format {
            component.location(format);
        }

        let mut description = String::new();
        if let Some(speaker) = &ev.speaker {
            description.push_str("Спикер: ");
            description.push_str(speaker);
            description.push('\n');
        }
        if let Some(format) = &ev.format {
            description.push_str("Формат: ");
            description.push_str(format);
            description.push('\n');
        }
        if let Some(url) = &ev.stream_url {
            description.push_str("Ссылка на встречу: ");
            description.push_str(url);
            description.push('\n');
        }
        if let Some(url) = &ev.recording_url {
            description.push_str("Запись: ");
            description.push_str(url);
            description.push('\n');
        }
        if !description.is_empty() {
            component.description(description.trim_end());
        }
        if let Some(url) = &ev.stream_url {
            component.url(url);
        }

        calendar.push(component.done());
    }

    calendar.done().to_string()
}

fn stable_uid(ev: &ScheduleEvent) -> String {
    let mut hasher = DefaultHasher::new();
    ev.identity_key().hash(&mut hasher);
    format!("{:016x}@skedule-parse", hasher.finish())
}
