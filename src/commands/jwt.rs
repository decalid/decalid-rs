use anyhow::Result;

use decalid::{auth::jwt::generate_token, db::Db, config::Config};

pub async fn login_new_device(
    config: &Config,
    db: &Db,
    user_id: i64,
    device_description: &str,
    override_expiration: Option<u64>,
) -> Result<String> {
    let device = db
        .users()
        .create_user_device(user_id, device_description)
        .await?;

    let expiration = override_expiration.unwrap_or(config.auth.jwt_expiration_seconds);
    let config = config.auth.with_expiration(expiration);

    let token = generate_token(&config, &device.device_id)?;

    println!("Token: {}", token);
    println!("Device ID: {}", device.device_id);
    println!("Expiration: {}", expiration);

    Ok(token)
}

pub async fn logout_device(
    db: &Db,
    user_id: i64,
    device_id: &str,
) -> Result<()> {
    db.users().delete_user_device(user_id, device_id).await?;

    Ok(())
}