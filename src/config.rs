use std::time::Duration;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    /// URL of the Nextcloud public share page (e.g. https://cloud.mirea.ru/index.php/s/XXXX).
    pub source_url: String,
    /// How often the worker re-fetches the source and rebuilds the calendar.
    pub poll_interval: Duration,
    /// Address the HTTP server binds to, e.g. 0.0.0.0:8080.
    pub bind_addr: String,
    /// Path the calendar is served at, e.g. /calendar.ics.
    pub calendar_path: String,
    /// Fixed UTC offset (in hours) the schedule's dates/times are expressed in.
    pub tz_offset_hours: i32,
    /// Timeout for outbound HTTP requests (share page + xlsx download).
    pub http_timeout: Duration,
    /// Calendar display name (NAME / X-WR-CALNAME).
    pub calendar_name: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // .env is optional: in production config is usually supplied via real
        // environment variables (docker-compose env_file, systemd, etc).
        let _ = dotenvy::dotenv();

        let source_url = std::env::var("SOURCE_URL")
            .context("SOURCE_URL is not set (the Nextcloud public share page URL)")?;

        let poll_interval_secs = env_parse("POLL_INTERVAL_SECS", 300u64)?;
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let calendar_path =
            std::env::var("CALENDAR_PATH").unwrap_or_else(|_| "/calendar.ics".to_string());
        let calendar_path = if calendar_path.starts_with('/') {
            calendar_path
        } else {
            format!("/{calendar_path}")
        };
        let tz_offset_hours = env_parse("TZ_OFFSET_HOURS", 3i32)?; // Europe/Moscow by default
        let http_timeout_secs = env_parse("HTTP_TIMEOUT_SECS", 30u64)?;
        let calendar_name =
            std::env::var("CALENDAR_NAME").unwrap_or_else(|_| "Расписание".to_string());

        Ok(Self {
            source_url,
            poll_interval: Duration::from_secs(poll_interval_secs),
            bind_addr,
            calendar_path,
            tz_offset_hours,
            http_timeout: Duration::from_secs(http_timeout_secs),
            calendar_name,
        })
    }
}

fn env_parse<T>(key: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(val) => val
            .parse::<T>()
            .map_err(|e| anyhow::anyhow!("invalid value for {key}: {e}")),
        Err(_) => Ok(default),
    }
}
