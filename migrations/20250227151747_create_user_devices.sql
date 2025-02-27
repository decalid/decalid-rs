-- Add migration script here
CREATE TABLE user_devices (
    device_id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    device_description TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Add an index on user_id for faster lookups
CREATE INDEX idx_user_devices_user_id ON user_devices(user_id);

-- Add triggers to automatically update the updated_at timestamp
CREATE TRIGGER update_user_devices_updated_at 
    AFTER UPDATE ON user_devices
    BEGIN
        UPDATE user_devices SET updated_at = CURRENT_TIMESTAMP
        WHERE device_id = NEW.device_id;
    END;
