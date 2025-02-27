use log::info;

use crate::{db::models::EventVersion, events::model::ParsedEvent};

use super::{models::Event, Db};


pub struct EventsDb<'a> {
    pub(super) db: &'a Db,
    pub(super) calendar_id: i64,
}

impl<'a> EventsDb<'a> {
    pub async fn create(&self, event: ParsedEvent) -> Result<EventVersion, sqlx::Error> {
        let event_record = sqlx::query_as::<_, Event>(
            "INSERT INTO events (calendar_id, created_at, updated_at) VALUES (?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) RETURNING *"
        )
        .bind(self.calendar_id)
        .fetch_one(&self.db.0)
        .await?;

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
             VALUES (1, ?, ?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) 
             RETURNING *"
        )
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

        // Create event UID
        if let Some(uid_value) = event.uid {
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
             WHERE e.calendar_id = ? AND eu.uid = ?"
        )
        .bind(self.calendar_id)
        .bind(uid)
        .fetch_optional(&self.db.0)
        .await
    }

    /// Update an existing event with new data
    pub async fn update(&self, event_id: i64, current_version: i32, event: ParsedEvent) -> Result<EventVersion, sqlx::Error> {
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
             RETURNING *"
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
        sqlx::query("UPDATE events SET current_version_id = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(event_version.id)
            .bind(event_id)
            .execute(&mut *transaction)
            .await?;

        // Commit the transaction
        transaction.commit().await?;

        Ok(event_version)
    }
}
