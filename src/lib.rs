//! Decalid - A CalDAV server implementation in Rust
//!
//! This library provides functionality for implementing a CalDAV server
//! that can aggregate multiple calendars and apply transformations.

pub mod api;
pub mod auth;
pub mod cache;
pub mod caldav;
pub mod chrono_utils;
pub mod config;
pub mod db;
pub mod events;
pub mod ics;
pub mod telemetry;
pub mod timezone;
pub mod transformation;

// Re-export important types
pub use caldav::client::CalDavClient;
pub use db::models::Timezone as ParsedTimezone;
pub use db::Db;

// Re-export important functions
pub use auth::jwt::generate_token;
