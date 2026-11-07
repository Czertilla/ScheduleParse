use std::io::Cursor;

use anyhow::{Context, Result};
use calamine::{Data, DataType, Reader, Xlsx};
use chrono::NaiveTime;

use crate::schedule::ScheduleEvent;

/// Column indices discovered from a header row. The spreadsheet repeats a
/// header row before every week's block, so this is re-detected as we walk
/// down the sheet rather than assumed to be fixed.
struct ColumnMap {
    title: usize,
    speaker: Option<usize>,
    date: usize,
    time: Option<usize>,
    format: Option<usize>,
    stream: Option<usize>,
    recording: Option<usize>,
}

/// Parses every sheet of the workbook and returns all schedule rows found.
pub fn parse_schedule(bytes: &[u8]) -> Result<Vec<ScheduleEvent>> {
    let cursor = Cursor::new(bytes);
    let mut workbook: Xlsx<_> =
        calamine::open_workbook_from_rs(cursor).context("opening xlsx workbook")?;

    let sheet_names = workbook.sheet_names().to_owned();
    let mut events = Vec::new();
    for sheet_name in sheet_names {
        let range = workbook
            .worksheet_range(&sheet_name)
            .with_context(|| format!("reading sheet '{sheet_name}'"))?;

        let mut columns: Option<ColumnMap> = None;
        for row in range.rows() {
            if let Some(map) = detect_header(row) {
                columns = Some(map);
                continue;
            }
            let Some(map) = &columns else { continue };
            if let Some(event) = parse_row(row, map) {
                events.push(event);
            }
        }
    }
    Ok(events)
}

fn detect_header(row: &[Data]) -> Option<ColumnMap> {
    let mut title = None;
    let mut speaker = None;
    let mut date = None;
    let mut time = None;
    let mut format = None;
    let mut stream = None;
    let mut recording = None;

    for (i, cell) in row.iter().enumerate() {
        let text = cell
            .as_string()
            .unwrap_or_default()
            .to_lowercase()
            .replace('\n', " ");
        let text = text.trim();
        if text.is_empty() {
            continue;
        }
        if text.contains("дата") {
            date = Some(i);
        } else if text.contains("время") {
            time = Some(i);
        } else if text.contains("формат") {
            format = Some(i);
        } else if text.contains("запис") {
            recording = Some(i);
        } else if text.contains("трансляц") || text.contains("встреч") {
            stream = Some(i);
        } else if text.contains("спикер") || text.contains("лектор") || text.contains("преподава")
        {
            speaker = Some(i);
        } else if text.contains("наименование")
            || text.contains("образователь")
            || text.contains("дисциплин")
            || text.contains("курс")
            || text.contains("модул")
        {
            title = Some(i);
        }
    }

    match (title, date) {
        (Some(title), Some(date)) => Some(ColumnMap {
            title,
            speaker,
            date,
            time,
            format,
            stream,
            recording,
        }),
        _ => None,
    }
}

fn parse_row(row: &[Data], map: &ColumnMap) -> Option<ScheduleEvent> {
    let date = row.get(map.date)?.as_date()?;

    let title = row
        .get(map.title)
        .and_then(|c| c.as_string())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())?;

    let speaker = map
        .speaker
        .and_then(|i| row.get(i))
        .and_then(|c| c.as_string())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let format = map
        .format
        .and_then(|i| row.get(i))
        .and_then(|c| c.as_string())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let stream_url = map
        .stream
        .and_then(|i| row.get(i))
        .and_then(|c| c.as_string())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let recording_url = map
        .recording
        .and_then(|i| row.get(i))
        .and_then(|c| c.as_string())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let time_range = map
        .time
        .and_then(|i| row.get(i))
        .and_then(|c| c.as_string())
        .and_then(|s| parse_time_range(&s));

    Some(ScheduleEvent {
        title,
        speaker,
        format,
        stream_url,
        recording_url,
        date,
        time_range,
    })
}

/// Parses strings like `"18:00-19:30"`, `" 20:00–21:30"` or
/// `"12:00-18:00\n(время уточняется)"` into a `(start, end)` pair, ignoring
/// any trailing note after the first line.
fn parse_time_range(raw: &str) -> Option<(NaiveTime, NaiveTime)> {
    let first_line = raw.lines().next().unwrap_or("").trim();
    if first_line.is_empty() {
        return None;
    }
    let normalized = first_line.replace(['–', '—'], "-");
    let mut parts = normalized.splitn(2, '-');
    let start = parts.next()?.trim();
    let end = parts.next()?.trim();
    let start = parse_hhmm(start)?;
    let end = parse_hhmm(end)?;
    Some((start, end))
}

fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    let s = s.trim();
    NaiveTime::parse_from_str(s, "%H:%M").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_range() {
        assert_eq!(
            parse_time_range("18:00-19:30"),
            Some((
                NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(19, 30, 0).unwrap()
            ))
        );
    }

    #[test]
    fn parses_en_dash_with_leading_space() {
        assert_eq!(
            parse_time_range(" 20:00–21:30"),
            Some((
                NaiveTime::from_hms_opt(20, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(21, 30, 0).unwrap()
            ))
        );
    }

    #[test]
    fn parses_range_with_trailing_note() {
        assert_eq!(
            parse_time_range("12:00-18:00\n(время уточняется)"),
            Some((
                NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
                NaiveTime::from_hms_opt(18, 0, 0).unwrap()
            ))
        );
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_time_range(""), None);
        assert_eq!(parse_time_range("уточняется"), None);
    }
}
