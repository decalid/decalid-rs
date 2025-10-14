use anyhow::Result;
use chrono::{DateTime, Utc};
use http::StatusCode as HttpStatusCode;
use log::{debug, info, warn};
use reqwest::{Client, RequestBuilder, StatusCode};
use std::time::Duration;
use yaserde::{de::from_str, ser::to_string};

use crate::caldav::propfind::{FieldWithContent, Multistatus, OkProps, WithStatus};
use crate::db::models::SyncInfo;
use crate::db::Db;
use crate::ics;

use super::propfind::Propfind;

#[cfg(test)]
mod tests;

/// Configuration for a CalDAV client connection
#[derive(Debug, Clone)]
pub struct CalDavConfig {
    /// The URL of the CalDAV server
    pub url: String,
    /// Authentication credentials
    pub auth: CalDavAuth,
    /// Optional timeout in seconds (defaults to 30)
    pub timeout_secs: Option<u64>,
}

/// Authentication methods for CalDAV
#[derive(Debug, Clone)]
pub enum CalDavAuth {
    /// Basic authentication with username and password
    Basic { username: String, password: String },
    /// Token-based authentication
    Bearer { token: String },
    /// No authentication
    None,
}


#[derive(Debug)]
pub struct ClientIcsData {
    pub url: String,
    pub ics: String,
}

/// CalDAV client for interacting with external CalDAV servers
pub struct CalDavClient {
    client: Client,
    config: CalDavConfig,
}

impl CalDavClient {
    /// Create a new CalDAV client with the given configuration
    pub fn new(config: CalDavConfig) -> Result<Self> {
        let timeout = Duration::from_secs(config.timeout_secs.unwrap_or(30));

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self { client, config })
    }

    /// Add authentication to a request
    fn authenticate(&self, request: RequestBuilder) -> RequestBuilder {
        match &self.config.auth {
            CalDavAuth::Basic { username, password } => {
                request.basic_auth(username, Some(password))
            }
            CalDavAuth::Bearer { token } => request.bearer_auth(token),
            CalDavAuth::None => request,
        }
    }

    /// Test the connection to the CalDAV server
    pub async fn test_connection(&self) -> Result<bool> {
        let response = self
            .authenticate(self.client.get(&self.config.url))
            .header("Content-Type", "application/xml")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send request to CalDAV server: {}", e))?;

        let status = response.status();
        debug!("Connection test status: {}", status);

        Ok(status.is_success()
            || status == StatusCode::UNAUTHORIZED
            || status == StatusCode::FORBIDDEN)
    }

    /// Discover calendars on the CalDAV server
    pub async fn discover_calendars(&self) -> Result<Vec<CalendarInfo>> {
        // First, try to find the principal URL
        let principal_url = self.discover_principal().await?;

        // Then, find the calendar home set
        let calendar_home = self.discover_calendar_home(&principal_url).await?;

        // Finally, get the list of calendars
        self.list_calendars(&calendar_home).await
    }

    /// Discover the principal URL
    async fn discover_principal(&self) -> Result<String> {
        // Implement PROPFIND to find the current-user-principal
        // For simplicity, we'll just return the base URL for now
        Ok(self.config.url.clone())
    }

    /// Discover the calendar home set
    async fn discover_calendar_home(&self, principal_url: &str) -> Result<String> {
        // Implement PROPFIND to find the calendar-home-set
        // For simplicity, we'll just return the principal URL for now
        Ok(principal_url.to_string())
    }

    /// List calendars in the calendar home set
    async fn list_calendars(&self, calendar_home: &str) -> Result<Vec<CalendarInfo>> {
        // Create a PROPFIND request to get calendar information
        let propfind = Propfind {
            prop: vec![super::propfind::Prop {
                resourcetype: Some(Default::default()),
                displayname: Some(Default::default()),
                calendar_color: Some(Default::default()),
                calendar_description: Some(Default::default()),
                getetag: Some(Default::default()),
                ..Default::default()
            }],
            allprop: None,
            propname: None,
        };

        let propfind_xml = to_string(&propfind)
            .map_err(|e| anyhow::anyhow!("Failed to serialize PROPFIND request: {}", e))?;

        let response = self
            .authenticate(self.client.request(
                reqwest::Method::from_bytes(b"PROPFIND").unwrap(),
                &format!("{}", calendar_home),
            ))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(propfind_xml)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send PROPFIND request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to list calendars: HTTP {}",
                response.status()
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;
        debug!("Calendar list response: {}", body);

        // Parse the XML response to extract calendar information
        let mut calendars = Vec::new();

        match from_str::<crate::caldav::propfind::Multistatus>(&body) {
            Ok(multistatus) => {
                for response in &multistatus.responses {
                    // Check if this is a calendar resource
                    // Find if in any of the propstats we have a prop for resourcetype calendar
                    let is_calendar = response
                        .ok_props_iter()
                        .filter(|&WithStatus { prop, .. }| {
                            prop.resourcetype
                                .as_ref()
                                .map(|rt| rt.calendar.is_some())
                                .unwrap_or(false)
                        })
                        .drop_status()
                        .flat_map(|prop| prop.resourcetype.as_ref())
                        .flat_map(|rt| rt.calendar.as_ref())
                        .next()
                        .is_some();

                    if is_calendar {
                        // Extract calendar information
                        let url = response.href.clone();

                        // Get display name
                        let display_name = response
                            ._get_prop_string(|prop| prop.displayname.as_ref())
                            .unwrap_or_else(|| "Unnamed Calendar".to_string());

                        // Get color
                        let color = response._get_prop_string(|prop| prop.calendar_color.as_ref());

                        // Get description
                        let description =
                            response._get_prop_string(|prop| prop.calendar_description.as_ref());

                        // Get ctag for change tracking
                        let ctag = response._get_prop_string(|prop| prop.getetag.as_ref());

                        calendars.push(CalendarInfo {
                            url,
                            display_name,
                            color,
                            description,
                            ctag,
                        });
                    }
                }
            }
            Err(e) => {
                warn!("Failed to parse calendar list response: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to parse calendar list response: {}",
                    e
                ));
            }
        }

        Ok(calendars)
    }

    /// Fetch calendar events from a specific calendar URL
    pub async fn fetch_calendar_events(&self, calendar_url: &str) -> Result<Vec<String>> {
        // Create a REPORT request to get calendar data
        let report_body = r#"<?xml version="1.0" encoding="utf-8" ?>
            <C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
                <D:prop>
                    <D:getetag/>
                    <C:calendar-data/>
                </D:prop>
                <C:filter>
                    <C:comp-filter name="VCALENDAR"/>
                </C:filter>
            </C:calendar-query>"#;

        let response = self
            .authenticate(self.client.request(
                reqwest::Method::from_bytes(b"REPORT").unwrap(),
                calendar_url,
            ))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(report_body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send REPORT request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch calendar events: HTTP {}",
                response.status()
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;
        debug!("Calendar events response: {}", body);

        // In a real implementation, we would parse the XML response to extract iCalendar data
        // For simplicity, we'll just return a dummy iCalendar string
        let ics_data = vec![r#"BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Example Corp.//CalDAV Client//EN
BEGIN:VEVENT
UID:1234567890
DTSTAMP:20250227T013409Z
DTSTART:20250301T090000Z
DTEND:20250301T100000Z
SUMMARY:Example Event
DESCRIPTION:This is an example event from CalDAV
END:VEVENT
END:VCALENDAR"#
            .to_string()];

        Ok(ics_data)
    }

    pub async fn discover_calendar_syncinfo(&self, calendar_url: &str) -> Result<SyncInfo> {
        // MAke an OPTIONS request to check if this is a DAV share or not. If it's DAV, then we want to issue a REPORT.
        // If it's not DAV, then we want to issue a GET

        let response = reqwest::Client::new()
            .request(reqwest::Method::OPTIONS, calendar_url)
            .header("Depth", "1")
            .header("Content-Type", "application/xml")
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send OPTIONS request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch calendar events: HTTP {}",
                response.status()
            ));
        }

        // Check if it's a DAV share
        let dav = response
            .headers()
            .get("DAV")
            .map(|dav| dav.to_str())
            .transpose()?;
        let allow = response
            .headers()
            .get("Allow")
            .map(|allow| allow.to_str())
            .transpose()?;

        if let (Some(dav), Some(allow)) = (dav, allow) {
            if dav.contains("calendar-access") && allow.contains("REPORT") {
                return Ok(SyncInfo::CalDavSyncInfo {
                    last_successful_sync: None,
                    sync_token: None,
                });
            }
        }
        Ok(SyncInfo::ICalSyncInfo {
            last_successful_sync: None,
            last_etag: None,
            last_modfified: None,
        })
    }

    pub async fn fetch_calendar_events_caldav(
        &self,
        calendar_url: &str,
        sync_info: SyncInfo,
    ) -> Result<(Vec<ClientIcsData>, SyncInfo)> {
        let SyncInfo::CalDavSyncInfo { sync_token, .. } = sync_info else {
            return Err(anyhow::anyhow!("Invalid sync info"));
        };

        if let Some(token) = sync_token {
            log::debug!(
                "Fetching calendar events from {} with sync token: {}",
                calendar_url,
                token
            );
            self.fetch_calendar_events_caldav_sync_collection(calendar_url, &token)
                .await
        } else {
            log::debug!(
                "Fetching calendar events from {} (first sync)",
                calendar_url
            );
            self.fetch_calendar_events_caldav_calendar_query(calendar_url)
                .await
        }
    }

    async fn fetch_calendar_events_caldav_sync_collection(
        &self,
        calendar_url: &str,
        sync_token: &str,
    ) -> Result<(Vec<ClientIcsData>, SyncInfo)> {
        let report_body = format!(
            r#"<?xml version="1.0" encoding="utf-8" ?>
            <D:sync-collection xmlns:D="DAV:">
                <D:sync-token>{}</D:sync-token>
                <D:sync-level>1</D:sync-level>
                <D:prop>
                    <D:getetag/>
                    <C:calendar-data xmlns:C="urn:ietf:params:xml:ns:caldav"/>
                    <D:sync-token/>
                </D:prop>
            </D:sync-collection>"#,
            sync_token
        );

        let response = self
            .authenticate(self.client.request(
                reqwest::Method::from_bytes(b"REPORT").unwrap(),
                calendar_url,
            ))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(report_body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send REPORT request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch calendar events: HTTP {}",
                response.status()
            ));
        }

        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;
        debug!("Calendar events response: {}", body);

        // Parse the XML response based on whether we're using sync-collection or calendar-query
        let mut calendar_data = Vec::new();
        let mut new_sync_token = None;

        // Parse the sync-collection response
        match from_str::<Multistatus>(&body) {
            Ok(sync_response) => {
                // Extract the new sync token
                if let Some(token_content) = &sync_response.sync_token {
                    if let Some(token) = &token_content.text {
                        new_sync_token = Some(token.clone());
                    }
                }

                // Extract calendar data from each response
                for response in sync_response.responses {
                    let url = response.href.clone();
                    for prop in response
                        .into_all_props()
                        .filter_by_status(HttpStatusCode::is_success)
                        .drop_status()
                    {
                        if let Some(cal_data) = prop.calendar_data {
                            if let Some(ics) = cal_data.text {
                                calendar_data.push(ClientIcsData {
                                    url: url.clone(),
                                    ics,
                                });
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Failed to parse sync-collection response: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to parse sync-collection response: {}",
                    e
                ));
            }
        }

        Ok((
            calendar_data,
            SyncInfo::CalDavSyncInfo {
                last_successful_sync: Some(Utc::now()),
                sync_token: new_sync_token,
            },
        ))
    }

    async fn fetch_calendar_events_caldav_calendar_query(
        &self,
        calendar_url: &str,
    ) -> Result<(Vec<ClientIcsData>, SyncInfo)> {
        let report_body = r#"<?xml version="1.0" encoding="utf-8" ?>
            <C:calendar-query xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
                <D:prop>
                <D:getetag/>
                <C:calendar-data/>
                <D:sync-token/>
                </D:prop>
                <C:filter>
                    <C:comp-filter name="VCALENDAR"/>
                </C:filter>
            </C:calendar-query>"#
            .to_string();

        let response = self
            .authenticate(self.client.request(
                reqwest::Method::from_bytes(b"REPORT").unwrap(),
                calendar_url,
            ))
            .header("Content-Type", "application/xml")
            .header("Depth", "1")
            .body(report_body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send REPORT request: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch calendar events: HTTP {}",
                response.status()
            ));
        }

        let headers = response.headers().clone();
        let body = response
            .text()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;
        debug!("Calendar events response: {}", body);

        // Parse the XML response based on whether we're using sync-collection or calendar-query
        let mut calendar_data = Vec::new();
        let mut new_sync_token = None;

        // Parse the calendar-query response
        match from_str::<Multistatus>(&body) {
            Ok(query_response) => {
                log::debug!("[OK] Calendar events response: {:#?}", query_response);
                // Extract calendar data from each response
                for response in query_response.responses {
                    let url = response.href.clone();
                    for prop in response
                        .into_all_props()
                        .filter_by_status(HttpStatusCode::is_success)
                        .drop_status()
                    {
                        if let Some(cal_data) = prop.calendar_data {
                            if let Some(ics) = cal_data.text {
                                calendar_data.push(ClientIcsData {
                                    url: url.clone(),
                                    ics,
                                });
                            }
                        }
                    }
                }

                log::debug!("Headers: {:#?}", headers);
                log::debug!("Calendar data: {:#?}", calendar_data);

                // For initial queries, we might get a sync token in a header or property
                // This is server-dependent, so we'll check common locations
                if let Some(sync_token_header) = headers.get("Sync-Token") {
                    if let Ok(token) = sync_token_header.to_str() {
                        new_sync_token = Some(token.to_string());
                    }
                }
                log::debug!("New sync token: {:#?}", new_sync_token);
            }
            Err(e) => {
                warn!("Failed to parse calendar-query response: {}", e);
                return Err(anyhow::anyhow!(
                    "Failed to parse calendar-query response: {}",
                    e
                ));
            }
        }

        // If we didn't get any calendar data but the request was successful,
        // it might mean there were no changes
        if calendar_data.is_empty() {
            info!("No calendar data returned, possibly no changes since last sync");
        }

        Ok((
            calendar_data,
            SyncInfo::CalDavSyncInfo {
                last_successful_sync: Some(Utc::now()),
                sync_token: new_sync_token,
            },
        ))
    }

    pub async fn fetch_calendar_events_ical(
        &self,
        calendar_url: &str,
        sync_info: SyncInfo,
    ) -> Result<(Vec<ClientIcsData>, SyncInfo)> {
        // Create a GET request to get calendar data
        // If we have a sync_token, we can use it to only fetch changes
        match sync_info {
            SyncInfo::ICalSyncInfo {
                last_successful_sync: _,
                last_etag,
                last_modfified,
            } => {
                let response = self
                    .authenticate(self.client.get(calendar_url))
                    .header("If-None-Match", last_etag.unwrap_or_default())
                    .header(
                        "If-Modified-Since",
                        last_modfified.unwrap_or_default().to_rfc3339(),
                    )
                    .send()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to send GET request: {}", e))?;

                if !response.status().is_success() {
                    return Err(anyhow::anyhow!(
                        "Failed to fetch calendar events: HTTP {}",
                        response.status()
                    ));
                }

                let headers = response.headers().clone();
                let ics = response
                    .text()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;
                debug!("Calendar events response: {}", ics);

                let last_modified_header = headers
                    .get("Last-Modified")
                    .map(|h| h.to_str())
                    .transpose()?
                    .map(|s| DateTime::<chrono::FixedOffset>::parse_from_rfc2822(s))
                    .transpose()?
                    .map(|dt| dt.to_utc());
                let etag = headers
                    .get("Etag")
                    .map(|h| h.to_str())
                    .transpose()?
                    .map(|s| s.to_string());

                let content_type = headers
                    .get("Content-Type")
                    .map(|h| h.to_str())
                    .transpose()?
                    .map(|s| s.to_string())
                    .map(|s| s.contains("text/calendar"))
                    .unwrap_or(false);

                if !content_type {
                    return Err(anyhow::anyhow!("Invalid calendar data"));
                }

                Ok((
                    vec![ClientIcsData {
                        url: calendar_url.to_string(),
                        ics,
                    }],
                    SyncInfo::ICalSyncInfo {
                        last_successful_sync: Some(Utc::now()),
                        last_etag: etag,
                        last_modfified: last_modified_header,
                    },
                ))
            }
            _ => Err(anyhow::anyhow!("Invalid sync info")),
        }
    }

    /// Fetch calendar events from a specific calendar URL using etag/sync-token for efficient syncing
    pub async fn fetch_calendar_events_with_sync(
        &self,
        calendar_url: &str,
        sync_info: SyncInfo,
    ) -> Result<(Vec<ClientIcsData>, SyncInfo)> {
        // Create a REPORT request to get calendar data
        // If we have a sync_token, we can use it to only fetch changes
        log::debug!(
            "Fetching calendar events from {} with sync: {:#?}",
            calendar_url,
            sync_info
        );
        match sync_info {
            SyncInfo::None => {
                let new_sync_info = self.discover_calendar_syncinfo(calendar_url).await?;
                Box::pin(self.fetch_calendar_events_with_sync(calendar_url, new_sync_info)).await
            }
            SyncInfo::ICalSyncInfo { .. } => {
                self.fetch_calendar_events_ical(calendar_url, sync_info)
                    .await
            }
            SyncInfo::CalDavSyncInfo { .. } => {
                self.fetch_calendar_events_caldav(calendar_url, sync_info)
                    .await
            }
        }
    }

    /// Save calendar data to the database
    pub async fn save_to_database(
        &self,
        db: &Db,
        calendar_id: i64,
        url: &str,
        sync_info: &SyncInfo,
        ics_data: Vec<ClientIcsData>,
    ) -> Result<i64> {
        // Create or update the calendar source
        db.create_or_update_calendar_source(calendar_id, &url, sync_info)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create/update calendar source: {}", e))?;

        // Import each ICS file
        for ClientIcsData { ics, url } in ics_data {
            info!("Importing ICS data for calendar {}", calendar_id);
            ics::import_ics_data(db, calendar_id, &url, &ics)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to import ICS data: {}", e))?;
        }

        Ok(calendar_id)
    }

    pub async fn fetch_pull_save(&self, db: &Db, user_id: i64, calendar_id: i64) -> Result<()> {
        // This check ensures the user/calendar pair is correct, before proceeding
        db.get_calendar(user_id, calendar_id).await?;
        let source = db.get_calendar_source(calendar_id).await?;
        let url = source
            .caldav_url
            .ok_or(anyhow::anyhow!("No Calendar source URL provided"))?;
        let sync_info = source
            .sync_info
            .map(|s| serde_json::from_str(&s).ok())
            .flatten()
            .unwrap_or_default();
        let (events, sync_info) = self
            .fetch_calendar_events_with_sync(&url, sync_info)
            .await?;

        self.save_to_database(db, calendar_id, &url, &sync_info, events)
            .await?;

        Ok(())
    }
}

/// Information about a calendar from a CalDAV server
#[derive(Debug, Clone)]
pub struct CalendarInfo {
    /// The URL of the calendar
    pub url: String,
    /// The display name of the calendar
    pub display_name: String,
    /// The color of the calendar (optional)
    pub color: Option<String>,
    /// The description of the calendar (optional)
    pub description: Option<String>,
    /// The CTag of the calendar for change tracking (optional)
    pub ctag: Option<String>,
}
