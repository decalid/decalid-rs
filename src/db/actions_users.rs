use anyhow::Result;

use super::{models::{User, UserDevice}, Db};

pub struct UsersDb<'a> {
    pub(super) db: &'a Db,
}

const DEVICE_PREFIX: &str = "d-";

impl<'a> UsersDb<'a> {
    /// Get a user by its ID
    pub async fn get_by_id(&self, user_id: i64) -> Result<User> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one(&self.db.0)
            .await?;
        
        Ok(user)
    }

    pub async fn get_by_username(&self, username: &str) -> Result<User> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_one(&self.db.0)
            .await?;
        
        Ok(user)
    }
    
    pub(crate) async fn create_user_device(&self, user_id: i64, device_description: &str) -> Result<UserDevice> {
        let mut retries = 0;
        let mut device = Err(anyhow::anyhow!("Never tried"));
        while retries < 10 {
            
            let device_id = format!("{}-{}", DEVICE_PREFIX, nanoid::nanoid!(10));
            device = sqlx::query_as::<_, UserDevice>("INSERT INTO user_devices (device_id, user_id, device_description) VALUES (?, ?, ?) RETURNING *")
            .bind(device_id)
            .bind(user_id)
            .bind(device_description)
            .fetch_one(&self.db.0)
            .await.map_err(|e| anyhow::anyhow!("Failed to create user device: {:?}", e));

            if device.is_ok() {
                break;
            }
            retries += 1;
        }

        device
    }

    pub(crate) async fn get_user_device(&self, device_id: &str) -> Result<UserDevice> {
        let device = sqlx::query_as::<_, UserDevice>("SELECT * FROM user_devices WHERE device_id = ?")
            .bind(device_id)
            .fetch_one(&self.db.0)
            .await?;
        
        Ok(device)
    }
}
