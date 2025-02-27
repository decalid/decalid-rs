//! Telemetry module for metrics and logging
//! 
//! This module contains the telemetry logic for the application.

use log::{debug, error, info, trace, warn, LevelFilter};
use std::sync::Once;

mod metrics;
mod tracing;

static INIT: Once = Once::new();

/// Initialize telemetry
pub fn init(log_level: &str) {
    INIT.call_once(|| {
        // Set up logging
        let level = match log_level {
            "trace" => LevelFilter::Trace,
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => LevelFilter::Info,
        };
        
        env_logger::Builder::new()
            .filter_level(level)
            .init();
        
        info!("Telemetry initialized with log level: {}", log_level);
    });
}

/// Record a metric
pub fn record_metric(name: &str, value: f64) {
    // TODO: Implement metric recording
    debug!("Metric {}: {}", name, value);
}

/// Start a span
pub fn start_span(name: &str) -> SpanGuard {
    trace!("Starting span: {}", name);
    SpanGuard { name: name.to_string() }
}

/// A guard for a span
pub struct SpanGuard {
    name: String,
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        trace!("Ending span: {}", self.name);
    }
}

/// Record an error
pub fn record_error(error: &anyhow::Error) {
    error!("Error: {}", error);
    
    // TODO: Send error to Sentry
}
