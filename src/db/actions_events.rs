use log::info;

use crate::{db::models::EventVersion, events::model::ParsedEvent};

use super::{models::Event, Db};

pub struct EventsDb<'a> {
    pub(super) db: &'a Db,
    pub(super) calendar_id: i64,
}

impl<'a> EventsDb<'a> {
    pub async fn count(&self) -> Result<u32, sqlx::Error> {
        sqlx::query_scalar::<_, i32>(
            "SELECT COUNT(ev.id) FROM event_versions ev JOIN events e ON ev.event_id = e.id AND e.current_version_id = ev.id WHERE e.calendar_id = ?"
        )
        .bind(self.calendar_id)
        .fetch_one(&self.db.0)
        .await
        .map(|c| c as u32)
    }

    pub async fn list(&self, limit: u32, offset: u32) -> Result<Vec<EventVersion>, sqlx::Error> {
        sqlx::query_as::<_, EventVersion>(
            "SELECT ev.*, u.uid FROM event_versions ev JOIN events e ON ev.event_id = e.id JOIN event_uids u ON u.event_id = e.id AND e.current_version_id = ev.id WHERE e.calendar_id = ? ORDER BY ev.created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(self.calendar_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.db.0)
        .await
    }

    pub async fn create(&self, event: ParsedEvent) -> Result<EventVersion, sqlx::Error> {
        // Check if we already have an event with the same UID
        // TODO: This may not always work with Google Calendar for unspecified reasons by abc.xyz
        let event_uid = event.uid.as_deref();
        let event_record = {
            let event_record = sqlx::query_as::<_, Event>(
                "SELECT e.*, v.version as last_version FROM events e INNER JOIN event_uids u ON e.id = u.event_id INNER JOIN event_versions v ON e.current_version_id = v.id WHERE u.uid = ? AND sync_domain = ?"
            )
            .bind(event_uid)
            .bind(self.calendar_id)
            .fetch_optional(&self.db.0)
            .await?;

            match event_record {
                Some(record) => record,
                None => sqlx::query_as::<_, Event>(
                    "INSERT INTO events (calendar_id, created_at, updated_at) VALUES (?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING *"
                )
                .bind(self.calendar_id)
                .fetch_one(&self.db.0)
                .await?
            }
        };
        let next_version = event_record.last_version.unwrap_or(0) + 1;
        log::debug!("Got event = {event:?}, next version will be {next_version}");

        // Serialize the event to ICS format
        let event_raw_data = event.serialize();
        // Start a transaction
        let mut transaction = self.db.0.begin().await?;

        info!("Insert into event_versions");
        // Create a new event version
        let event_version = sqlx::query_as::<_, EventVersion>(
            "INSERT INTO event_versions (
                version,
                event_id,
                summary,
                description,
                dtstart,
                dtend,
                rrule,
                raw_data,
                is_all_day,
                last_repeat,
                created_at,
                last_retrieved_at
            ) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) 
             RETURNING *",
        )
        .bind(next_version)
        .bind(event_record.id)
        .bind(event.summary)
        .bind(event.description)
        .bind(event.dtstart)
        .bind(event.dtend)
        .bind(event.rrule)
        .bind(&event_raw_data)
        .bind(event._all_day)
        .bind(event._last_repeat)
        .fetch_one(&mut *transaction)
        .await?;

        info!("Update event version id");
        // Update the event with the current version
        sqlx::query("UPDATE events SET current_version_id = ? WHERE id = ?")
            .bind(event_version.id)
            .bind(event_record.id)
            .execute(&mut *transaction)
            .await?;

        // TODO: OPTIMIZE INTO UPSERT LATER
        if sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM event_uids WHERE uid = ?")
            .bind(event_uid)
            .fetch_one(&mut *transaction)
            .await?
            == 0
        {
            // Create event UID if not exists
            if let Some(uid_value) = event_uid {
                sqlx::query(
                    "INSERT INTO event_uids (event_id, uid, sync_domain, created_at) 
                 VALUES (?, ?, ?, CURRENT_TIMESTAMP)",
                )
                .bind(event_record.id)
                .bind(uid_value)
                .bind(self.calendar_id.to_string()) // Using calendar_id as sync_domain
                .execute(&mut *transaction)
                .await?;
            }
        }

        // Rollback the transaction
        transaction.commit().await?;

        Ok(event_version)
    }

    /// Find an event by its UID in the specified calendar
    pub async fn find_by_uid(&self, uid: &str) -> Result<Option<EventVersion>, sqlx::Error> {
        sqlx::query_as::<_, EventVersion>(
            "SELECT ev.* FROM events e
             JOIN event_uids eu ON e.id = eu.event_id
             JOIN event_versions ev ON e.current_version_id = ev.id
             WHERE e.calendar_id = ? AND eu.uid = ?",
        )
        .bind(self.calendar_id)
        .bind(uid)
        .fetch_optional(&self.db.0)
        .await
    }

    /// Update an existing event with new data
    pub async fn update(
        &self,
        event_id: i64,
        current_version: i32,
        event: ParsedEvent,
    ) -> Result<EventVersion, sqlx::Error> {
        // Start a transaction
        let mut transaction = self.db.0.begin().await?;

        // Serialize the event to ICS format
        let event_raw_data = event.serialize();

        // Create a new event version with incremented version number
        let new_version = current_version + 1;

        // Create a new event version
        let event_version = sqlx::query_as::<_, EventVersion>(
            "INSERT INTO event_versions (
                version,
                event_id,
                summary,
                description,
                dtstart,
                dtend,
                rrule,
                raw_data,
                is_all_day,
                last_repeat,
                created_at,
                last_retrieved_at
            ) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) 
             RETURNING *",
        )
        .bind(new_version)
        .bind(event_id)
        .bind(event.summary)
        .bind(event.description)
        .bind(event.dtstart)
        .bind(event.dtend)
        .bind(event.rrule)
        .bind(&event_raw_data)
        .bind(event._all_day)
        .bind(event._last_repeat)
        .fetch_one(&mut *transaction)
        .await?;

        // Update the event with the new current version
        sqlx::query(
            "UPDATE events SET current_version_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(event_version.id)
        .bind(event_id)
        .execute(&mut *transaction)
        .await?;

        // Commit the transaction
        transaction.commit().await?;

        Ok(event_version)
    }
}
