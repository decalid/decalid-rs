use chrono::{DateTime, Utc};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Calendar {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CalendarSource {
    pub id: i64,
    pub calendar_id: i64,
    pub caldav_url: Option<String>,
    pub sync_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CalendarShare {
    pub id: i64,
    pub calendar_id: i64,
    pub url_slug: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Event {
    pub id: i64,
    pub current_version_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EventVersion {
    pub id: i64,
    pub event_id: i64,
    pub version: i32,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub dtstart: Option<DateTime<Utc>>,
    pub dtend: Option<DateTime<Utc>>,
    pub duration: Option<String>,
    pub rrule: Option<String>,
    pub rdate: Option<String>,
    pub exdate: Option<String>,
    pub status: Option<String>,
    pub organizer: Option<String>,
    pub location: Option<String>,
    pub url: Option<String>,
    pub class: Option<String>,
    pub priority: Option<i32>,
    pub transp: Option<String>,
    pub sequence: Option<i32>,
    pub raw_data: String,
    pub created_at: DateTime<Utc>,
    pub last_retrieved_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EventUid {
    pub id: i64,
    pub event_id: i64,
    pub uid: String,
    pub sync_domain: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EventAttendee {
    pub id: i64,
    pub event_version_id: i64,
    pub attendee: String,
    pub role: Option<String>,
    pub partstat: Option<String>,
    pub rsvp: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EventAlarm {
    pub id: i64,
    pub event_version_id: i64,
    pub action: String,
    pub trigger: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Freebusy {
    pub id: i64,
    pub event_version_id: i64,
    pub fbtype: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

