use axum::{
    body::Body,
    extract::{rejection::PathRejection, Path, State},
    http::{HeaderMap, HeaderValue, Response},
    response::IntoResponse,
    routing::Router,
    RequestExt,
};
use extractors::XmlExtract;
use std::sync::Arc;

use crate::{db::Db, ics::convert_to_icalendar};

use super::propfind::{self, PropGetETag};
mod extractors;
mod preconditions;

pub type Result<T, E = DecalidHttpError> = core::result::Result<T, E>;

fn push_webdav_options_headers(headers: &mut axum::http::HeaderMap) {
    headers.insert(
        "DAV",
        HeaderValue::from_static("1, 2, access-control, calendar-access"),
    );
    headers.insert(
        "Allow",
        HeaderValue::from_static(
            "OPTIONS, GET, HEAD, POST, PUT, DELETE, PROPFIND, PROPPATCH, MKCOL, REPORT",
        ),
    );
}

fn webdav_response_builder() -> axum::http::response::Builder {
    let mut builder = Response::builder();
    push_webdav_options_headers(builder.headers_mut().unwrap());
    builder
}

async fn webdav_options(
    Path(share_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    preconditions::share_exists(&state.db, &share_id).await?;

    Ok(webdav_response_builder().body(Body::empty())?)
}

async fn webdav_propfind(
    Path(share_id): Path<String>,
    headers: HeaderMap,
    State(state): State<AppState>,
    XmlExtract(payload): XmlExtract<propfind::Propfind>,
) -> Result<Response<Body>> {
    preconditions::propfind_payload_is_valid(&payload)?;
    preconditions::share_exists(&state.db, &share_id).await?;
    let calendars = state.db.shares(&share_id).get_calendars().await?;

    // Implement logic to retrieve and return properties as XML
    let host = headers
        .get("Host")
        .map(|header| header.to_str().unwrap())
        .unwrap_or("localhost");
    let use_tls = ""; // TODO: implement this check
    let host = format!("http{}://{}", use_tls, host);

    let allowed_calendar_components = vec![
        propfind::PropSupportedCalendarComponent {
            name: "VEVENT".to_string(),
        },
        // propfind::PropSupportedCalendarComponent {
        //     name: "VTODO".to_string(),
        // },
        // propfind::PropSupportedCalendarComponent {
        //     name: "VJOURNAL".to_string(),
        // },
        // propfind::PropSupportedCalendarComponent {
        //     name: "VALARM".to_string(),
        // },
    ];

    let responses: Vec<propfind::Response> = calendars
        .into_iter()
        .map(|calendar| {
            if payload.allprop.is_some() {
                // We need to return all properties
                propfind::Response {
                    href: format!(
                        "{}/{}/{}",
                        host,
                        share_id,
                        calendar.share_id.expect(
                            "calendar should retrieve share_id from shares_db::get_calendars()"
                        )
                    ),
                    propstat: vec![propfind::Propstat {
                        prop: vec![propfind::Prop {
                            displayname: Some(propfind::PropDisplayName {
                                content: Some(calendar.name.into()),
                            }),
                            calendar_color: calendar.color.map(|color| {
                                propfind::PropCalendarColor {
                                    content: Some(color.into()),
                                }
                            }),
                            calendar_description: calendar.share_description.map(|content| {
                                propfind::PropCalendarDescription {
                                    content: Some(content.into()),
                                }
                            }),
                            resourcetype: Some(propfind::PropResourceType {
                                calendar: None,
                                collection: Some(propfind::Empty),
                            }),
                            getetag: calendar.etag.map(|etag| PropGetETag {
                                content: Some(etag.into()),
                            }),
                            supported_calendar_component_set: None,
                            calendar_timezone: "".to_string(),
                            supported_calendar_data: None,
                            calendar_data: None,
                        }],
                        status: "HTTP/1.1 200 OK".to_string(),
                    }],
                }
            } else if payload.propname.is_some() {
                propfind::Response {
                    href: format!(
                        "{}/{}/{}",
                        host,
                        share_id,
                        calendar.share_id.expect(
                            "calendar should retrieve share_id from shares_db::get_calendars()"
                        )
                    ),
                    propstat: vec![propfind::Propstat {
                        prop: vec![propfind::Prop::propnames()],
                        status: "HTTP/1.1 200 OK".to_string(),
                    }],
                }
            } else {
                fn when_in_payload<'a, P: 'a, T>(
                    payload: &'a propfind::Propfind,
                    get_payload_field: impl Fn(&'a propfind::Prop) -> &'a Option<P>,
                    calendar_getter: impl FnOnce() -> T,
                ) -> Option<T> {
                    payload
                        .prop
                        .iter()
                        .find(|prop| get_payload_field(prop).is_some())
                        .map(|_| calendar_getter())
                }
                // Depending on the requested properties, only return those
                propfind::Response {
                    href: format!(
                        "{}/{}/{}",
                        host,
                        share_id,
                        calendar.share_id.expect(
                            "calendar should retrieve share_id from shares_db::get_calendars()"
                        )
                    ),
                    propstat: vec![propfind::Propstat {
                        prop: vec![propfind::Prop {
                            displayname: when_in_payload(
                                &payload,
                                |prop| &prop.displayname,
                                || propfind::PropDisplayName {
                                    content: Some(calendar.name.into()),
                                },
                            ),
                            calendar_color: when_in_payload(
                                &payload,
                                |prop| &prop.calendar_color,
                                || propfind::PropCalendarColor {
                                    content: calendar.color.map(|color| color.into()),
                                },
                            ),
                            calendar_description: when_in_payload(
                                &payload,
                                |prop| &prop.calendar_description,
                                || propfind::PropCalendarDescription {
                                    content: calendar
                                        .share_description
                                        .map(|content| content.into()),
                                },
                            ),
                            resourcetype: when_in_payload(
                                &payload,
                                |prop| &prop.resourcetype,
                                || propfind::PropResourceType {
                                    calendar: None,
                                    collection: Some(propfind::Empty {}),
                                },
                            ),
                            getetag: when_in_payload(
                                &payload,
                                |prop| &prop.getetag,
                                || PropGetETag {
                                    content: calendar.etag.map(|etag| etag.into()),
                                },
                            ),
                            supported_calendar_component_set: when_in_payload(
                                &payload,
                                |prop| &prop.supported_calendar_component_set,
                                || allowed_calendar_components.clone().into(),
                            ),
                            calendar_timezone: if payload
                                .prop
                                .iter()
                                .any(|p| !p.calendar_timezone.is_empty())
                            {
                                calendar.timezone.clone()
                            } else {
                                "".to_string()
                            },
                            supported_calendar_data: when_in_payload(
                                &payload,
                                |prop| &prop.supported_calendar_data,
                                || propfind::PropSupportedCalendarData {
                                    calendar_data: Some(propfind::PropCalendarData::default()),
                                },
                            ),
                            calendar_data: when_in_payload(
                                &payload,
                                |prop| &prop.calendar_data,
                                || propfind::PropCalendarData::default(),
                            ),
                        }],
                        status: "HTTP/1.1 200 OK".to_string(),
                    }],
                }
            }
        })
        .collect();

    let propfind_response = propfind::Multistatus {
        sync_token: None,
        responses,
    };

    let mut xml_config = yaserde::ser::Config::default();
    xml_config.perform_indent = true;
    let properties_xml = yaserde::ser::to_string_with_config(&propfind_response, &xml_config)
        .expect("Failed to serialize to XML");
    Ok(webdav_response_builder()
        .status(207) // Multi-Status
        .header("Content-Type", "application/xml")
        .body(Body::from(properties_xml))?)
}

async fn webdav_report(
    Path(_share_id): Path<String>,
    State(_app_state): State<AppState>,
) -> Result<Response<Body>> {
    let report_response = propfind::Multistatus {
        sync_token: None,
        responses: Vec::new(),
    };
    let properties_xml =
        yaserde::ser::to_string(&report_response).expect("Failed to serialize to XML");
    Ok(webdav_response_builder()
        .status(207)
        .header("Content-Type", "application/xml")
        .body(Body::from(properties_xml))?)
}

async fn webdav_fallback(
    state: State<AppState>,
    mut request: axum::http::Request<Body>,
) -> Result<Response<Body>> {
    match request.method().as_str() {
        "PROPFIND" => {
            let (parts, headers, payload) = request
                .extract()
                .await
                .map_err(|e: Response<Body>| DecalidHttpError(e))?;
            webdav_propfind(parts, headers, state, payload).await
        }
        "REPORT" => {
            let parts = request
                .extract_parts()
                .await
                .map_err(|e: PathRejection| DecalidHttpError(e.into_response()))?;
            webdav_report(parts, state).await
        }
        _ => Ok(webdav_response_builder()
            .status(400)
            .header("Content-Type", "text/plain")
            .body(Body::from("Method not supported"))?),
    }
}

pub struct DecalidHttpError(pub Response<Body>);
impl DecalidHttpError {
    pub fn from_str(arg: &str) -> DecalidHttpError {
        DecalidHttpError(
            Response::builder()
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "text/plain")
                .body(Body::from(arg.to_string()))
                .unwrap(),
        )
    }
    pub fn with_status(self, status: axum::http::StatusCode) -> DecalidHttpError {
        DecalidHttpError(
            Response::builder()
                .status(status)
                .header("Content-Type", "text/plain")
                .body(self.0.into_body())
                .unwrap(),
        )
    }
}

impl<T: Into<anyhow::Error>> From<T> for DecalidHttpError {
    fn from(error: T) -> Self {
        let anyerror: anyhow::Error = error.into();
        let error_message = if log::max_level() >= log::Level::Debug {
            format!(
                "Internal Server Error: {} (at {})",
                anyerror,
                anyerror.backtrace()
            )
        } else {
            "Internal Server Error".to_string()
        };

        let response = Response::builder()
            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            .header("Content-Type", "text/plain")
            .body(Body::from(error_message))
            .unwrap();
        DecalidHttpError(response)
    }
}

impl IntoResponse for DecalidHttpError {
    fn into_response(self) -> Response<Body> {
        self.0
    }
}

// Now calendar-specific methods go here

async fn webdav_calendar_ics_get(
    Path((share_id, calendar_id)): Path<(String, String)>,
    State(app_state): State<AppState>,
) -> Result<Response<Body>> {
    preconditions::share_exists(&app_state.db, &share_id).await?;
    preconditions::share_calendar_exists(&app_state.db, &share_id, &calendar_id).await?;

    // Fetch the calendar data from the database
    let calendar_data = app_state
        .db
        .shares(&share_id)
        .get_calendar(&calendar_id)
        .await
        .expect("Failed to fetch calendar data");

    let calendar_events = app_state
        .db
        .shares(&share_id)
        .get_calendar_events(&calendar_id, None, None)
        .await
        .expect("Failed to fetch calendar events");

    let events_filters = app_state.db.get_filters(&share_id, &calendar_id).await?;

    // Filter events by any filter that we must use according to the share
    let mut filtered_calendar_events = calendar_events;
    for filter in events_filters {
        filtered_calendar_events = filter.apply(&filtered_calendar_events).await?;
    }

    // Convert the calendar data to an iCalendar format
    let icalendar_data =
        convert_to_icalendar(&app_state.db, &calendar_data, &filtered_calendar_events).await;

    // Return the iCalendar data as the response
    Ok(webdav_response_builder()
        .status(200)
        .header("Content-Type", "text/calendar; charset=utf-8")
        .body(Body::from(icalendar_data))?)
}

async fn webdav_calendar_put(
    Path((_share_id, _calendar_id)): Path<(String, String)>,
    State(_app_state): State<AppState>,
) -> impl IntoResponse {
    "todo!"
}
async fn webdav_calendar_delete(
    Path((_share_id, _calendar_id)): Path<(String, String)>,
    State(_app_state): State<AppState>,
) -> impl IntoResponse {
    "todo!"
}
async fn webdav_calendar_fallback(
    State(_app_state): State<AppState>,
    mut request: axum::http::Request<Body>,
) -> impl IntoResponse {
    match request.method().as_str() {
        "REPORT" => {
            let Path((_share_id, _calendar_id)): Path<(String, String)> =
                request.extract_parts().await.unwrap();
            webdav_response_builder()
                .status(400)
                .header("Content-Type", "text/plain")
                .body(Body::from("Method not supported yet"))
                .unwrap()
        }
        _ => webdav_response_builder()
            .status(400)
            .header("Content-Type", "text/plain")
            .body(Body::from("Method not supported"))
            .unwrap(),
    }
}

#[derive(Clone)]
struct AppState {
    db: Arc<Db>,
}

/// Register CalDAV routes on the provided router
pub fn register_routes(db: Arc<Db>) -> Router {
    Router::new()
        .route(
            "/share/{share_id}",
            axum::routing::options(webdav_options).fallback(webdav_fallback),
        )
        .route(
            "/share/{share_id}/{calendar_id}",
            axum::routing::get(webdav_calendar_ics_get)
                .put(webdav_calendar_put)
                .delete(webdav_calendar_delete)
                .fallback(webdav_calendar_fallback),
        )
        .with_state(AppState { db })
}
