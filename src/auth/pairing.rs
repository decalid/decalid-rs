//! Device pairing
//!
//! This module contains the device pairing logic.

#![allow(dead_code)]

use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A pairing code entry
struct PairingCodeEntry {
    /// The user ID associated with the pairing code
    user_id: String,

    /// When the pairing code was created
    created_at: Instant,

    /// Whether the pairing code has been used
    used: bool,
}

/// A simple in-memory pairing code store
pub struct PairingCodeStore {
    /// The pairing codes
    codes: Arc<Mutex<HashMap<String, PairingCodeEntry>>>,

    /// The expiration time for pairing codes
    expiration: Duration,
}

impl PairingCodeStore {
    /// Create a new pairing code store
    pub fn new(expiration_seconds: u64) -> Self {
        Self {
            codes: Arc::new(Mutex::new(HashMap::new())),
            expiration: Duration::from_secs(expiration_seconds),
        }
    }

    /// Generate a new pairing code
    pub fn generate_code(&self, user_id: &str) -> Result<String> {
        // Generate a 6-digit code
        let code = format!("{:06}", rand::random::<u32>() % 1_000_000);

        let mut codes = self.codes.lock().unwrap();
        codes.insert(
            code.clone(),
            PairingCodeEntry {
                user_id: user_id.to_string(),
                created_at: Instant::now(),
                used: false,
            },
        );

        Ok(code)
    }

    /// Validate a pairing code
    pub fn validate_code(&self, code: &str) -> Result<Option<String>> {
        let mut codes = self.codes.lock().unwrap();

        if let Some(entry) = codes.get_mut(code) {
            // Check if the code has expired
            if entry.created_at.elapsed() > self.expiration {
                codes.remove(code);
                return Ok(None);
            }

            // Check if the code has been used
            if entry.used {
                return Ok(None);
            }

            // Mark the code as used
            entry.used = true;

            return Ok(Some(entry.user_id.clone()));
        }

        Ok(None)
    }

    /// Clean up expired pairing codes
    pub fn cleanup(&self) {
        let mut codes = self.codes.lock().unwrap();
        codes.retain(|_, entry| entry.created_at.elapsed() <= self.expiration);
    }
}
