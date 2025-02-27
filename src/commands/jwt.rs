use crate::{auth, config::{AuthConfig, Config}, db::Db};

use anyhow::Result;

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

    Ok(auth::jwt::generate_token(&config, &device.device_id)?)
}
