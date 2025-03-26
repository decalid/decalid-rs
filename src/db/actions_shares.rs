use super::{
    models::{Calendar, CalendarShareRoot, EventVersion, SharedVirtualCalendar},
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
        min_utc: Option<chrono::DateTime<chrono::Utc>>,
        max_utc: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<EventVersion>, sqlx::Error> {
        let mut sql = "SELECT ev.* FROM events e \
             INNER JOIN event_versions ev ON e.current_version_id = ev.id \
             INNER JOIN share_collections sc ON sc.calendar_id = e.calendar_id \
             WHERE sc.id = ? AND sc.root_id = ?"
            .to_string();

        if let Some(min_utc) = min_utc {
            sql.push_str(" AND ev.dtstart >= ?");
        }
        if let Some(max_utc) = max_utc {
            sql.push_str(" AND ev.dtend <= ?");
        }

        let events = {
            let mut query = sqlx::query_as::<_, EventVersion>(&sql)
            .bind(calendar_id)
            .bind(&self.share_id);
            if let Some(min_utc) = min_utc {
                query = query.bind(min_utc);
            }
            if let Some(max_utc) = max_utc {
                query = query.bind(max_utc);
            }
            query.fetch_all(&self.db.0)
            .await?
        };

        println!(
            "Asked for events from {} to {} and got {}",
            min_utc.map_or("min".to_string(), |dt| dt.to_string()),
            max_utc.map_or("min".to_string(), |dt| dt.to_string()),
            events.len()
        );

        Ok(events)
    }

    pub(crate) async fn get_share(&self) -> Result<CalendarShareRoot, sqlx::Error> {
        let share =
            sqlx::query_as::<_, CalendarShareRoot>("SELECT * FROM share_roots WHERE id = ?")
                .bind(&self.share_id)
                .fetch_one(&self.db.0)
                .await?;

        Ok(share)
    }
    pub(crate) async fn get_shared_calendar(
        &self,
        calendar_id: &str,
    ) -> Result<SharedVirtualCalendar, sqlx::Error> {
        let share = sqlx::query_as::<_, SharedVirtualCalendar>(
            "SELECT * FROM share_virtualcalendars WHERE id = ? AND root_id = ?",
        )
        .bind(calendar_id)
        .bind(&self.share_id)
        .fetch_one(&self.db.0)
        .await?;

        Ok(share)
    }
}
