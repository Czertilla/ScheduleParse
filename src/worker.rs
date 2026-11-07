use chrono::Utc;

use crate::config::Config;
use crate::fetch;
use crate::ical;
use crate::state::SharedState;
use crate::xlsx;

/// Runs forever: fetch the share page, download the xlsx behind its Download
/// button, parse it into events, rebuild the ICS document and publish it to
/// `state`. Sleeps for `config.poll_interval` between attempts regardless of
/// success or failure, so a persistently unreachable source just keeps the
/// last known-good calendar being served instead of crashing the process.
pub async fn run(config: Config, client: reqwest::Client, state: SharedState) {
    loop {
        match refresh_once(&config, &client).await {
            Ok((ics, event_count)) => {
                tracing::info!(event_count, "calendar refreshed");
                let mut snapshot = state.write().await;
                snapshot.ics = ics;
                snapshot.event_count = event_count;
                snapshot.generated_at = Some(Utc::now());
                snapshot.last_error = None;
            }
            Err(err) => {
                tracing::warn!(error = %err, "calendar refresh failed, keeping last known-good calendar");
                let mut snapshot = state.write().await;
                snapshot.last_error = Some(err.to_string());
            }
        }
        tokio::time::sleep(config.poll_interval).await;
    }
}

async fn refresh_once(config: &Config, client: &reqwest::Client) -> anyhow::Result<(String, usize)> {
    let download_url = fetch::resolve_download_url(client, &config.source_url).await?;
    let bytes = fetch::download_bytes(client, &download_url).await?;
    let events = xlsx::parse_schedule(&bytes)?;
    let ics = ical::build_calendar(&events, &config.calendar_name, config.tz_offset_hours);
    Ok((ics, events.len()))
}
