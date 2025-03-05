use anyhow::Result;
use chrono::{DateTime, TimeZone};

use crate::events::model::DecalidEvent;

use super::{
    actions_events::EventsDb, actions_shares::SharesDb, actions_timezone::TimezoneDb, actions_users::UsersDb, models::{Calendar, CalendarSource, EventVersion, Filter, User}, Db
};

pub struct AdminDb<'a> {
    db: &'a Db,
}

impl<'a> AdminDb<'a> {
    pub async fn get_calendar_by_id(&self, calendar_id: i64) -> Result<Calendar, sqlx::Error> {
        sqlx::query_as::<_, Calendar>("SELECT * FROM calendars WHERE id = ?")
            .bind(calendar_id)
            .fetch_one(&self.db.0)
            .await
    }

    pub async fn create_user(&self, username: &str) -> Result<User, sqlx::Error> {
        sqlx::query_as::<_, User>("INSERT INTO users (username) VALUES (?) RETURNING *")
            .bind(username)
            .fetch_one(&self.db.0)
            .await
    }
    pub async fn list_users(&self) -> Result<Vec<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users")
            .fetch_all(&self.db.0)
            .await
    }

    pub async fn get_calendar_source_by_calendar_id(
        &self,
        calendar_id: i64,
    ) -> Result<Option<CalendarSource>, sqlx::Error> {
        sqlx::query_as::<_, CalendarSource>("SELECT * FROM calendar_sources WHERE calendar_id = ?")
            .bind(calendar_id)
            .fetch_optional(&self.db.0)
            .await
    }
}

impl Db {
    pub async fn close(self) {
        self.0.close().await
    }

    pub fn admin(&self) -> AdminDb {
        AdminDb { db: self }
    }
    pub fn events(&self, calendar_id: i64) -> EventsDb {
        EventsDb {
            db: self,
            calendar_id,
        }
    }

    pub fn shares<'a>(&'a self, share_id: &'a str) -> SharesDb<'a> {
        SharesDb { db: self, share_id }
    }

    pub fn users(&self) -> UsersDb {
        UsersDb { db: self }
    }

    pub fn timezones(&self) -> TimezoneDb {
        TimezoneDb { db: self }
    }

    pub async fn list_calendars(self: &Self, user_id: i64) -> Result<Vec<Calendar>, sqlx::Error> {
        sqlx::query_as::<_, Calendar>("SELECT * FROM calendars WHERE user_id = ?")
            .bind(user_id)
            .fetch_all(&self.0)
            .await
    }

    pub async fn create_calendar(
        self: &Self,
        user_id: i64,
        name: &str,
        color: Option<&str>,
    ) -> Result<Calendar, sqlx::Error> {
        sqlx::query_as::<_, Calendar>(
            "INSERT INTO calendars (user_id, name, color) VALUES (?, ?, ?) RETURNING *",
        )
        .bind(user_id)
        .bind(name)
        .bind(color)
        .fetch_one(&self.0)
        .await
    }

    /// Gets the current version of events between some dates.
    /// It does not return already-removed events
    pub async fn get_current_events_between_dates<Tz: TimeZone>(
        self: &Self,
        calendar_id: i64,
        min_date: DateTime<Tz>,
        max_date: DateTime<Tz>,
        max_results: i64,
    ) -> Result<Vec<DecalidEvent>>
    where
        <Tz as TimeZone>::Offset: Send,
        <Tz as TimeZone>::Offset: std::fmt::Display,
    {
        Ok(sqlx::query_as::<_, EventVersion>(
            "SELECT ev.* FROM events e INNER JOIN event_versions ev ON e.current_version_id = ev.id WHERE calendar_id = ? AND dtstart <= ? AND (last_repeat IS NULL OR last_repeat >= ?) LIMIT ?",
        )
        .bind(calendar_id)
        .bind(max_date)
        .bind(min_date)
        .bind(max_results)
        .fetch_all(&self.0)
        .await?.iter().map(DecalidEvent::from).collect())
    }

    /// Create or update a calendar source
    ///
    /// This links a calendar to an external CalDAV source
    pub async fn create_or_update_calendar_source(
        &self,
        calendar_id: i64,
        caldav_url: &str,
        sync_token: Option<&str>,
    ) -> Result<CalendarSource, sqlx::Error> {
        sqlx::query_as::<_, CalendarSource>(
            "INSERT INTO calendar_sources (calendar_id, caldav_url, sync_token) VALUES (?, ?, ?) 
             ON CONFLICT (calendar_id) DO UPDATE SET caldav_url = excluded.caldav_url, sync_token = excluded.sync_token, updated_at = CURRENT_TIMESTAMP
                     RETURNING *"
                )
                .bind(calendar_id)
                .bind(caldav_url)
                .bind(sync_token)
                .fetch_one(&self.0)
                .await
    }

    pub async fn get_filters(&self, share_id: &str, calendar_id: &str) -> Result<Vec<Filter>> {
        // Query to get filters associated with a virtual calendar
        // We join share_virtualcalendar_filters with filters table
        // to get the filter details
        let filters = sqlx::query_as::<_, Filter>(
            r#"SELECT f.* 
               FROM filters f
               JOIN share_virtualcalendar_filters svf ON f.id = svf.filter_id
               JOIN share_virtualcalendars sv ON svf.virtualcalendar_id = sv.id
               JOIN share_roots sr ON sv.root_id = sr.id
               WHERE sv.id = ? AND sr.id = ?
               ORDER BY f.created_at DESC"#
        )
        .bind(calendar_id)
        .bind(share_id)
        .fetch_all(&self.0)
        .await?
        ;

        Ok(filters)
    }
}
