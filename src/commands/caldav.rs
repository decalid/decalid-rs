use anyhow::Result;
use log::{debug, info};

use decalid::{
    caldav::client::{CalDavAuth, CalDavClient, CalDavConfig, ClientIcsData},
    db::{models::SyncInfo, Db},
    events::model::ParsedEvent,
    timezone::{parse_chrono_tz, resolve_timezone_reference},
};

/// Add a CalDAV source to a calendar
pub async fn add_caldav_source(
    db: &mut Db,
    calendar_id: i64,
    url: &str,
    username: Option<&str>,
    password: Option<&str>,
    token: Option<&str>,
) -> Result<()> {
    // Validate the calendar exists
    let calendar = db.admin().get_calendar_by_id(calendar_id).await?;
    
    // Create authentication based on provided credentials
    let auth = if let Some(token) = token {
        CalDavAuth::Bearer {
            token: token.to_string(),
        }
    } else if let (Some(username), Some(password)) = (username, password) {
        CalDavAuth::Basic {
            username: username.to_string(),
            password: password.to_string(),
        }
    } else {
        CalDavAuth::None
    };
    
    // Create the CalDAV client configuration
    let config = CalDavConfig {
        url: url.to_string(),
        auth,
        timeout_secs: Some(30),
    };
    
    // Create the CalDAV client
    let client = CalDavClient::new(config)?;
    
    // Test the connection
    info!("Testing connection to CalDAV server at {}", url);
    if !client.test_connection().await? {
        debug!("Failed to connect to CalDAV server");
        return Err(anyhow::anyhow!("Failed to connect to CalDAV server"));
    }
    
    // Discover calendars
    info!("Discovering calendars on CalDAV server");
    let calendars = client.discover_calendars().await?;
    
    if calendars.is_empty() {
        debug!("No calendars found on CalDAV server");
        return Err(anyhow::anyhow!("No calendars found on CalDAV server"));
    }
    
    // For each calendar, fetch events and save them
    for cal_info in calendars {
        info!(
            "Found calendar: {} ({})",
            cal_info.display_name,
            cal_info.url
        );
        
        // Fetch events for this calendar
        info!("Fetching events from {}", cal_info.url);
        let (events, sync_info) = client.fetch_calendar_events_with_sync(&cal_info.url, SyncInfo::None).await?;

        // Save the events to the database
        info!("Saving {} events to calendar {}", events.len(), calendar.id);
        client
            .save_to_database(db, calendar.id, &cal_info.url, &sync_info, events)
            .await?;
    }
    
    info!("Successfully added CalDAV source to calendar {}", calendar_id);
    Ok(())
}

/// Sync a calendar with its CalDAV source
pub async fn sync_caldav_calendar(db: &mut Db, calendar_id: i64) -> Result<()> {
    // Get the CalDAV source for this calendar
    let source = db.admin().get_calendar_source_by_calendar_id(calendar_id).await?;
    
    let Some(source) = source else {
        debug!("No CalDAV source found for calendar {}", calendar_id);
        return Err(anyhow::anyhow!("No CalDAV source found for this calendar"));
    };
    let source = source.parsed()?;
    
    let Some(url) = &source.caldav_url else {
        debug!("CalDAV source has no URL for calendar {}", calendar_id);
        return Err(anyhow::anyhow!("CalDAV source has no URL"));
    };
    
    // For now, we'll just use unauthenticated access
    // In a real implementation, we would store and retrieve credentials securely
    let config = CalDavConfig {
        url: url.to_string(),
        auth: CalDavAuth::None,
        timeout_secs: Some(30),
    };
    
    // Create the CalDAV client
    let client = CalDavClient::new(config)?;
    
    // Test the connection
    info!("Testing connection to CalDAV server at {}", url);
    if !client.test_connection().await? {
        debug!("Failed to connect to CalDAV server");
        return Err(anyhow::anyhow!("Failed to connect to CalDAV server"));
    }
    
    // Use the sync token if available for efficient syncing
    info!("Fetching events from {} using sync token: {:?}", url, source.sync_info);
    let (events, new_sync_info) = client
        .fetch_calendar_events_with_sync(url, source.sync_info)
        .await?;

    // Process each event
    info!("Processing {} events for calendar {}", events.len(), calendar_id);
    let events_db = db.events(calendar_id);
    let calendar = db.admin().get_calendar_by_id(calendar_id).await?;
    let default_timezone = if let Some(timezone_id) = calendar.timezone_id {
        db.timezones()
            .get_by_id(timezone_id)
            .await?
            .map(|tz| resolve_timezone_reference(&tz.tzid))
    } else {
        None
    }
    .or_else(|| {
        let tz = calendar.timezone.trim();
        if tz.is_empty() {
            None
        } else {
            Some(resolve_timezone_reference(tz))
        }
    })
    .map(|tzid| parse_chrono_tz(&tzid))
    .transpose()?;

    for ClientIcsData { ics, url } in events {
        // Parse the ICS data to extract events
        let reader = ical::IcalParser::new(ics.as_bytes());

        for cal_result in reader {
            let cal = cal_result?;
            for event in cal.events {
                // Create a new event
                let parsed_event =
                    ParsedEvent::new_with_default_timezone(event, default_timezone)?;
                let parsed_event = ParsedEvent {
                    url: Some(url.clone()),
                    ..parsed_event
                };
                
                // Check if this event already exists by UID
                if let Some(uid) = &parsed_event.uid {
                    match events_db.find_by_uid(uid).await? {
                        Some(existing_version) => {
                            // Event exists, update it
                            info!("Updating existing event with UID: {}", uid);
                            events_db.update(existing_version.event_id, existing_version.version, parsed_event).await?;
                        }
                        None => {
                            // Event doesn't exist, create it
                            info!("Creating new event with UID: {}", uid);
                            events_db.create(parsed_event).await?;
                        }
                    }
                } else {
                    // No UID, always create as new
                    info!("Creating new event without UID");
                    events_db.create(parsed_event).await?;
                }
            }
        }
    }
    
    // Update the sync token in the database
    if new_sync_info != SyncInfo::None {
        info!("Updating sync token to: {:?}", new_sync_info);
        db.create_or_update_calendar_source(calendar_id, url, &new_sync_info).await?;
    }
    
    info!("Successfully synced calendar {} with CalDAV source", calendar_id);
    Ok(())
}
