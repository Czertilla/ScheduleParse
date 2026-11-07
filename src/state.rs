use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

/// Snapshot of the most recently generated calendar, shared between the
/// background worker (writer) and the HTTP server (reader).
#[derive(Debug, Clone, Default)]
pub struct CalendarSnapshot {
    pub ics: String,
    pub generated_at: Option<DateTime<Utc>>,
    pub event_count: usize,
    pub last_error: Option<String>,
}

pub type SharedState = Arc<RwLock<CalendarSnapshot>>;

pub fn new_shared_state() -> SharedState {
    Arc::new(RwLock::new(CalendarSnapshot::default()))
}
