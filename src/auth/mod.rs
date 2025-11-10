//! Authentication module for Passkeys, pairing, and JWT
//!
//! This module contains the authentication logic for the application.

use serde::{Deserialize, Serialize};

pub mod jwt;
mod pairing;

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,

    /// Issued at
    pub iat: u64,

    /// Expiration
    pub exp: u64,
}
