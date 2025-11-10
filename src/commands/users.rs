use anyhow::Result;

use decalid::db::Db;

pub async fn create_user(db: &Db, username: &str) -> Result<()> {
    let user = db.admin().create_user(username).await?;
    println!("User created: {user:?}");
    Ok(())
}

pub async fn list_users(db: &Db) -> Result<()> {
    let users = db.admin().list_users().await?;

    println!("Users:");
    for user in users {
        println!("{user:?}");
    }

    Ok(())
}
