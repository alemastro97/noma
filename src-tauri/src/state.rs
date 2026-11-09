use std::sync::Mutex;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use rusqlite::Connection;

/// Shared application state managed by Tauri.
pub struct AppState {
    pub db: Mutex<Connection>,
}

impl AppState {
    pub fn new(db: Connection) -> Self {
        Self { db: Mutex::new(db) }
    }
}

/// The real clock, injected by the command layer so the data layer stays pure.
pub fn now() -> DateTime<Utc> {
    Utc::now()
}

/// The system time zone, falling back to UTC when it cannot be determined.
pub fn local_tz() -> Tz {
    iana_time_zone::get_timezone()
        .ok()
        .and_then(|name| name.parse::<Tz>().ok())
        .unwrap_or(chrono_tz::UTC)
}
