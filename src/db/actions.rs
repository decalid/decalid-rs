use anyhow::Result;

use super::{
    Db,
    models::{Calendar, CalendarSource, User},
};

pub struct AdminDb<'a> {
    pub(super) db: &'a Db,
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
