use anyhow::Result;
use sqlx::SqlitePool;

use crate::models::User;


pub async fn create_user(pool: &SqlitePool, username: &str) -> Result<()> {
    let user = sqlx::query_as::<_, User>("INSERT INTO users (username) VALUES (?) RETURNING *")
        .bind(username)
        .fetch_one(pool)
        .await?;

    println!("User created: {:?}", user);

    Ok(())
}

pub async fn list_users(pool: &SqlitePool) -> Result<()> {
    let users = sqlx::query_as::<_, User>("SELECT * FROM users")
        .fetch_all(pool)
        .await?;

    println!("Users:");
    for user in users {
        println!("{:?}", user);
    }

    Ok(())
}