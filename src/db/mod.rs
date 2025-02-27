pub mod actions;
pub mod actions_events;
pub mod actions_shares;
pub mod actions_timezone;
pub mod actions_users;
pub mod models;

pub struct Db(pub(crate) sqlx::SqlitePool);


impl Db {
    pub fn new(pool: sqlx::SqlitePool) -> Self {
        Db(pool)
    }
    
    #[cfg(test)]
    pub(crate) async fn new_in_memory() -> Self {
        Db(sqlx::sqlite::SqlitePool::connect("sqlite::memory:").await.unwrap())
    }
}
