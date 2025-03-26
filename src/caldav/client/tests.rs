use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::Response,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct MockServer {
    task: Option<JoinHandle<()>>,
    port: u16,
    db: Arc<Mutex<Option<crate::db::Db>>>,
}

impl MockServer {
    pub fn new() -> Self {
        Self {
            task: None,
            port: 8099,
            db: Arc::new(Mutex::new(None)),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://localhost:{}/{}", self.port(), path)
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run(&mut self, db: crate::db::Db) {
        let db = Arc::new(Mutex::new(Some(db)));
        self.db = db.clone();

        let app = Router::new()
            .route("/", get(handle_root))
            .route("/calendars/user/calendar1/", get(handle_calendar))
            .route(
                "/calendars/user/calendar1/",
                axum::routing::any(handle_calendar_any),
            )
            .route("/{calendar}", post(handle_calendar_post))
            .with_state(db);

        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], self.port()));
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind to port");

        self.task = Some(tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app.into_make_service()).await {
                eprintln!("Server error: {}", e);
            }
        }));
    }

    pub async fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

async fn handle_root() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .body(axum::body::Body::empty())
        .unwrap()
}

async fn handle_calendar() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .body(axum::body::Body::empty())
        .unwrap()
}

async fn handle_calendar_any(request: Request<Body>) -> Response {
    if request.method().as_str() == "REPORT" {
        return Response::builder()
            .status(StatusCode::OK)
            .body(axum::body::Body::from(
                r#"<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
        <D:response>
            <D:href>/calendars/user/calendar1/test.ics</D:href>
            <D:propstat>
                <D:prop>
                    <D:displayname>Test Event</D:displayname>
                    <C:calendar-data>BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Example Corp.//CalDAV Client//EN
BEGIN:VEVENT
UID:1234567890
DTSTAMP:20240101T120000Z
DTSTART:20240101T120000Z
DTEND:20240101T130000Z
SUMMARY:Test Event
END:VEVENT
END:VCALENDAR</C:calendar-data>
                </D:prop>
                <D:status>HTTP/1.1 200 OK</D:status>
            </D:propstat>
        </D:response></D:multistatus>"#,
            ))
            .unwrap()
    }

    return Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .body(axum::body::Body::empty())
        .unwrap();
}

async fn handle_calendar_post() -> Response {
    Response::builder()
        .status(StatusCode::CREATED)
        .body(axum::body::Body::empty())
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caldav::{
        client::{CalDavAuth, CalDavClient, CalDavConfig, CalendarInfo},
        propfind::Multistatus,
    };
    use yaserde::de::from_str;

    #[test]
    fn test_parse_calendar_list_response() {
        let xml = r#"<?xml version="1.0" encoding="utf-8" ?>
        <d:multistatus xmlns:d="DAV:" xmlns:cal="urn:ietf:params:xml:ns:caldav" xmlns:ext="https://decalid.com/ns/calendar-0">
            <d:response>
                <d:href>/calendars/user/calendar1/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:resourcetype>
                            <d:collection/>
                            <cal:calendar/>
                        </d:resourcetype>
                        <d:displayname>
                            <d:content>Work Calendar</d:content>
                        </d:displayname>
                        <ext:calendar-color>
                            <d:content>#FF0000</d:content>
                        </ext:calendar-color>
                        <cal:calendar-description>
                            <d:content>Calendar for work events</d:content>
                        </cal:calendar-description>
                        <d:getetag>
                            <d:content>"12345"</d:content>
                        </d:getetag>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
            <d:response>
                <d:href>/calendars/user/calendar2/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:resourcetype>
                            <d:collection/>
                            <cal:calendar/>
                        </d:resourcetype>
                        <d:displayname>
                            <d:content>Personal Calendar</d:content>
                        </d:displayname>
                        <ext:calendar-color>
                            <d:content>#0000FF</d:content>
                        </ext:calendar-color>
                        <d:getetag>
                            <d:content>"67890"</d:content>
                        </d:getetag>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
            <d:response>
                <d:href>/calendars/user/not-a-calendar/</d:href>
                <d:propstat>
                    <d:prop>
                        <d:resourcetype>
                            <d:collection/>
                        </d:resourcetype>
                        <d:displayname>
                            <d:content>Not a Calendar</d:content>
                        </d:displayname>
                    </d:prop>
                    <d:status>HTTP/1.1 200 OK</d:status>
                </d:propstat>
            </d:response>
        </d:multistatus>"#;

        let multistatus: Multistatus = from_str(xml).unwrap();

        // There should be 3 responses in total
        assert_eq!(multistatus.responses.len(), 3);

        // Extract calendar information
        let mut calendars = Vec::new();

        for response in &multistatus.responses {
            // Check if this is a calendar resource
            let is_calendar = if let Some(resource_type) = &response.propstat.prop.resourcetype {
                resource_type.calendar.is_some()
            } else {
                false
            };

            if is_calendar {
                // Extract calendar information
                let url = response.href.clone();

                // Get display name
                let display_name = response
                    .propstat
                    .prop
                    .displayname
                    .as_ref()
                    .and_then(|prop| prop.content.as_ref())
                    .and_then(|content| content.text.as_ref())
                    .map(|text| text.clone())
                    .unwrap_or_else(|| "Unnamed Calendar".to_string());

                // Get color
                let color = response
                    .propstat
                    .prop
                    .calendar_color
                    .as_ref()
                    .and_then(|prop| prop.content.as_ref())
                    .and_then(|content| content.text.as_ref())
                    .map(|text| text.clone());

                // Get description
                let description = response
                    .propstat
                    .prop
                    .calendar_description
                    .as_ref()
                    .and_then(|prop| prop.content.as_ref())
                    .and_then(|content| content.text.as_ref())
                    .map(|text| text.clone());

                // Get ctag for change tracking
                let ctag = response
                    .propstat
                    .prop
                    .getetag
                    .as_ref()
                    .and_then(|prop| prop.content.as_ref())
                    .and_then(|content| content.text.as_ref())
                    .map(|text| text.clone());

                calendars.push(CalendarInfo {
                    url,
                    display_name,
                    color,
                    description,
                    ctag,
                });
            }
        }

        // There should be 2 calendars
        assert_eq!(calendars.len(), 2);

        // Check the first calendar
        assert_eq!(calendars[0].url, "/calendars/user/calendar1/");
        assert_eq!(calendars[0].display_name, "Work Calendar");
        assert_eq!(calendars[0].color, Some("#FF0000".to_string()));
        assert_eq!(
            calendars[0].description,
            Some("Calendar for work events".to_string())
        );
        assert_eq!(calendars[0].ctag, Some("\"12345\"".to_string()));

        // Check the second calendar
        assert_eq!(calendars[1].url, "/calendars/user/calendar2/");
        assert_eq!(calendars[1].display_name, "Personal Calendar");
        assert_eq!(calendars[1].color, Some("#0000FF".to_string()));
        assert_eq!(calendars[1].description, None);
        assert_eq!(calendars[1].ctag, Some("\"67890\"".to_string()));
    }

    #[tokio::test]
    async fn test_fetch_mock_calendar_events() {
        // Create mock http server that can answer certain CalDAV requests
        let db = crate::db::Db::new_in_memory().await;
        let mut server = tests::MockServer::new();
        server.run(db).await;

        // Create CalDAV client
        let config = CalDavConfig {
            url: server.url("/"),
            auth: CalDavAuth::None,
            timeout_secs: Some(5),
        };

        let client = CalDavClient::new(config).unwrap();

        // Fetch calendar events
        let calendar_url = server.url("calendars/user/calendar1/");
        println!("Fetching calendar events from {}", calendar_url);
        let (ics_data, _) = client
            .fetch_calendar_events_with_sync(&calendar_url, crate::db::models::SyncInfo::CalDavSyncInfo { last_successful_sync: None, sync_token: None })
            .await
            .unwrap();
        assert!(!ics_data.is_empty());

        server.stop().await;
    }
}
