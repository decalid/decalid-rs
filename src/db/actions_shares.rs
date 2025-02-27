use super::{
    models::{Calendar, EventVersion},
    Db,
};

pub struct SharesDb<'a> {
    pub(super) db: &'a Db,
    pub(super) share_id: &'a str,
}

impl<'a> SharesDb<'a> {
    pub async fn admin_create(&self, owner_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO share_roots (id, owner_id) VALUES (?, ?)")
            .bind(&self.share_id)
            .bind(owner_id)
            .execute(&self.db.0)
            .await?;
        Ok(())
    }

    pub async fn attach_calendar(
        &self,
        calendar_id: i64,
        description: &str,
    ) -> Result<String, sqlx::Error> {
        use nanoid::nanoid;
        let new_random_id = nanoid!(10); // Generates a 10-character random ID
        sqlx::query("INSERT INTO share_collections (id, root_id, calendar_id, description) VALUES (?, ?, ?, ?)")
            .bind(&new_random_id)
            .bind(&self.share_id)
            .bind(calendar_id)
            .bind(description)
            .execute(&self.db.0)
            .await?;
        Ok(new_random_id)
    }

    pub async fn check_exists(&self) -> Result<bool, sqlx::Error> {
        let (count, _) =
            sqlx::query_as::<_, (i32, i32)>("SELECT COUNT(*), 1 FROM share_roots WHERE id = ?")
                .bind(&self.share_id)
                .fetch_one(&self.db.0)
                .await?;
        Ok(count == 1)
    }

    pub async fn get_calendars(&self) -> Result<Vec<Calendar>, sqlx::Error> {
        let calendars = sqlx::query_as::<_, Calendar>(
            "SELECT sc.id as share_id, sc.description as share_description, c.* FROM share_collections sc \
             INNER JOIN calendars c ON sc.calendar_id = c.id \
             WHERE sc.root_id = ?"
        )
        .bind(&self.share_id)
        .fetch_all(&self.db.0)
        .await?;

        Ok(calendars)
    }

    pub async fn get_calendar(&self, calendar_id: &str) -> Result<Calendar, sqlx::Error> {
        let calendar = sqlx::query_as::<_, Calendar>(
            "SELECT sc.id as share_id, sc.description as share_description, c.* FROM share_collections sc \
             INNER JOIN calendars c ON sc.calendar_id = c.id \
             WHERE sc.root_id = ? AND sc.id = ?"
        )
        .bind(&self.share_id)
        .bind(calendar_id)
        .fetch_one(&self.db.0)
        .await?;

        Ok(calendar)
    }

    pub async fn check_exists_calendar(&self, calendar_id: &str) -> Result<bool, sqlx::Error> {
        let (count, _) = sqlx::query_as::<_, (i32, i32)>(
            "SELECT COUNT(*), 1 FROM share_collections WHERE id = ? AND root_id = ?",
        )
        .bind(calendar_id)
        .bind(&self.share_id)
        .fetch_one(&self.db.0)
        .await?;
        Ok(count == 1)
    }

    pub async fn get_calendar_events(
        &self,
        calendar_id: &str,
        min_utc: chrono::DateTime<chrono::Utc>,
        max_utc: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<EventVersion>, sqlx::Error> {
        let events = sqlx::query_as::<_, EventVersion>(
            "SELECT ev.* FROM events e \
             INNER JOIN event_versions ev ON e.current_version_id = ev.id \
             INNER JOIN share_collections sc ON sc.calendar_id = e.calendar_id \
             WHERE sc.id = ? AND sc.root_id = ? AND ev.dtstart >= ? AND ev.dtend <= ?",
        )
        .bind(calendar_id)
        .bind(&self.share_id)
        .bind(min_utc)
        .bind(max_utc)
        .fetch_all(&self.db.0)
        .await?;

        println!(
            "Asked for events from {} to {} and got {}",
            min_utc,
            max_utc,
            events.len()
        );

        Ok(events)
    }
}
