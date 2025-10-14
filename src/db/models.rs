use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::transformation;

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(unused)]
#[derive(Clone, Debug, sqlx::FromRow)]
pub struct UserDevice {
    pub device_id: String,
    pub user_id: i64,
    pub device_description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}



#[allow(unused)]
#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct Calendar {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub color: Option<String>,

    pub timezone_id: Option<i64>, // Reference to VTIMEZONE
    
    #[sqlx(default)]
    pub etag: Option<String>,
    #[sqlx(default)]
    pub timezone: String,

    // For the JOIN with share_collections
    #[sqlx(default)]
    pub share_id: Option<String>,
    #[sqlx(default)]
    pub share_description: Option<String>,
    
    pub created_at: DateTime<Utc>,
}

#[allow(unused)]
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::FromRow)]
pub struct CalendarSource {
    pub id: i64,
    pub calendar_id: i64,
    pub caldav_url: Option<String>,
    pub sync_info: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CalendarSource {
    pub fn parse(&self) -> Result<ParsedCalendarSource, anyhow::Error> {
        // sync_info is a JSON string
        let sync_info: SyncInfo = self.sync_info.as_ref().map(|info| serde_json::from_str(info)).transpose()?.unwrap_or_default();

        Ok(ParsedCalendarSource {
            id: self.id,
            calendar_id: self.calendar_id,
            caldav_url: self.caldav_url.clone(),
            sync_info,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
    pub fn parsed(self) -> Result<ParsedCalendarSource, anyhow::Error> {
        // sync_info is a JSON string
        let sync_info: SyncInfo = self.sync_info.as_ref().map(|info| serde_json::from_str(info)).transpose()?.unwrap_or_default();

        Ok(ParsedCalendarSource {
            id: self.id,
            calendar_id: self.calendar_id,
            caldav_url: self.caldav_url,
            sync_info,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

#[derive(Debug)]
pub struct ParsedCalendarSource {
    pub id: i64,
    pub calendar_id: i64,
    pub caldav_url: Option<String>,
    pub sync_info: SyncInfo,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Default, PartialEq)]
pub enum SyncInfo {
    #[default]
    None,
    ICalSyncInfo{
        last_successful_sync: Option<DateTime<Utc>>,
        last_etag: Option<String>,
        last_modfified: Option<DateTime<Utc>>,
    },
    CalDavSyncInfo{
        last_successful_sync: Option<DateTime<Utc>>,
        sync_token: Option<String>,
    },
}


#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct CalendarShareRoot {
    pub id: String,
    pub owner_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct SharedVirtualCalendar {
    pub id: String,
    pub root_id: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "filter_status")]
pub enum FilterStatus {
    #[sqlx(rename = "PRV")]
    Private,
    #[sqlx(rename = "REV")]
    UnderReview,
    #[sqlx(rename = "RJ")]
    ReviewRejected,
    #[sqlx(rename = "PUB")]
    Published,
}

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct Filter {
    pub id: i64,
    pub name: String,
    pub filter: FilterStatus,
    pub status: String,
    pub body: String,
    pub creator_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
impl Filter {
    pub(crate) async fn apply(&self, filtered_calendar_events: &[EventVersion]) -> anyhow::Result<Vec<EventVersion>> {
        let mut engine = transformation::DslEngine::new();
        engine.compile(&self.body)?;
        engine.filter_events(filtered_calendar_events.iter().cloned())
    }
}

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct Event {
    pub id: i64,
    pub calendar_id: i64,
    pub current_version_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    #[sqlx(default)]
    pub last_version: Option<i64>,
}

#[allow(unused)]
#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct EventVersion {
    #[sqlx(default)]
    pub uid: Option<String>,

    pub id: i64,
    pub event_id: i64,
    pub version: i32,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub dtstart: Option<DateTime<Utc>>,
    pub dtend: Option<DateTime<Utc>>,
    pub duration: Option<String>,
    pub rrule: Option<String>,
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
    pub is_all_day: bool,
    pub last_repeat: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub last_retrieved_at: DateTime<Utc>,
    pub sync_status: Option<String>, // For tracking sync/reconciliation status
    pub conflict_with: Option<i64>,  // Reference to conflicting version if any
}

impl From<crate::events::model::ParsedEvent> for EventVersion {
    fn from(event: crate::events::model::ParsedEvent) -> Self {
        let raw_data = event.serialize();
        Self {
            uid: event.uid,
            id: 0,
            event_id: 0,
            version: 0,
            summary: event.summary,
            description: event.description,
            dtstart: event.dtstart.map(|x| x.to_utc()),
            dtend: event.dtend.map(|x| x.to_utc()),
            duration: event.duration,
            rrule: event.rrule,
            exdate: None,
            status: event.status,
            organizer: event.organizer,
            location: event.location,
            url: event.url,
            class: event.class,
            priority: event.priority.map(|x| x.parse::<i32>().ok()).flatten(),
            transp: event.transp,
            sequence: None,
            raw_data,
            is_all_day: event._all_day,
            last_repeat: event._last_repeat.map(|x| x.to_utc()),
            created_at: Utc::now(),
            last_retrieved_at: Utc::now(),
            sync_status: None,
            conflict_with: None       
        }
    }
}

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct EventUid {
    pub id: i64,
    pub event_id: i64,
    pub uid: String,
    pub sync_domain: String,
    pub created_at: DateTime<Utc>,
}

#[allow(unused)]
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

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct EventAlarm {
    pub id: i64,
    pub event_version_id: i64,
    pub action: String,
    pub trigger: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct Freebusy {
    pub id: i64,
    pub event_version_id: i64,
    pub fbtype: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct Timezone {
    pub id: i64,
    pub tzid: String,
    pub raw_data: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct TimezoneRule {
    pub id: i64,
    pub timezone_id: i64,
    pub rule_type: String, // "STANDARD" or "DAYLIGHT"
    pub dtstart: DateTime<Utc>,
    pub tzoffsetfrom: String,
    pub tzoffsetto: String,
    pub rrule: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[allow(unused)]
#[derive(Debug, sqlx::FromRow)]
pub struct ReconciliationLog {
    pub id: i64,
    pub event_id: i64,
    pub source_version_id: i64,
    pub target_version_id: i64,
    pub resolution: String, // "AUTO", "MANUAL", "CONFLICT"
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}
