//! REST API endpoints
//!
//! This module contains the REST API endpoints for the application.

use std::sync::Arc;

use axum::{
    error_handling::HandleErrorLayer,
    extract::{Path, State},
    response::IntoResponse,
    routing, Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;

use crate::{
    auth::jwt::middleware::{self, JWTErrors},
    caldav::server::DecalidHttpError,
    config::{AuthConfig, Config},
    db::{
        models::{Calendar, CalendarSource, UserDevice},
        Db,
    },
};

use super::pagination::Paginated;

/// API state containing the database connection
#[derive(Clone)]
pub(crate) struct AppStateImpl {
    pub(super) db: Arc<Db>,
    pub(super) config: AuthConfig,
}

pub(crate) type AppState = axum::extract::State<AppStateImpl>;

// Placeholder types for API responses
#[derive(Debug, Serialize)]
pub struct ShareResponse {
    id: String,
    title: String,
    // Other fields will be added as needed
}

#[derive(Debug, Serialize)]
pub struct CalendarResponse {
    id: i64,
    name: String,
    // Other fields will be added as needed
}

#[derive(Debug, Deserialize)]
pub struct CreateShareRequest {
    title: String,
    // Other fields will be added as needed
}

#[derive(Debug, Deserialize)]
pub struct CreateCalendarRequest {
    name: String,
    color: Option<String>,
    // Other fields will be added as needed
}

#[derive(Debug, Deserialize)]
pub struct PreviewFilterRequest {
    filter: String,
    sample_events: Vec<String>,
    // Other fields will be added as needed
}

#[derive(Debug, Serialize)]
pub struct PreviewFilterResponse {
    original_events: Vec<String>,
    transformed_events: Vec<String>,
    processing_times: Vec<u64>,
    errors: Vec<String>,
    // Other fields will be added as needed
}

// Shares endpoints
pub async fn list_shares(State(_db): AppState) -> Json<Vec<ShareResponse>> {
    // TODO: Implement list shares
    Json(vec![])
}

pub async fn create_share(
    State(_db): AppState,
    Json(_request): Json<CreateShareRequest>,
) -> Json<ShareResponse> {
    // TODO: Implement create share
    Json(ShareResponse {
        id: "placeholder".to_string(),
        title: "Placeholder Share".to_string(),
    })
}

pub async fn get_share(State(_db): AppState, Path(_id): Path<String>) -> Json<ShareResponse> {
    // TODO: Implement get share
    Json(ShareResponse {
        id: "placeholder".to_string(),
        title: "Placeholder Share".to_string(),
    })
}

pub async fn update_share(
    State(_db): AppState,
    Path(_id): Path<String>,
    Json(_request): Json<CreateShareRequest>,
) -> Json<ShareResponse> {
    // TODO: Implement update share
    Json(ShareResponse {
        id: "placeholder".to_string(),
        title: "Placeholder Share".to_string(),
    })
}

pub async fn delete_share(State(_db): AppState, Path(_id): Path<String>) -> Json<()> {
    // TODO: Implement delete share
    Json(())
}

// Calendars endpoints
pub async fn list_calendars(
    State(app): AppState,
    Extension(user_device): Extension<UserDevice>,
) -> Result<Json<Paginated<Calendar>>, DecalidHttpError> {
    let calendars = app.db.list_calendars(user_device.user_id).await?;
    Ok(Json(Paginated::new(10, 0, calendars.len() as u32).with_body(calendars)))
}

pub async fn create_calendar(
    State(app): AppState,
    Extension(user_device): Extension<UserDevice>,
    Json(request): Json<CreateCalendarRequest>,
) -> Result<Json<CalendarResponse>, DecalidHttpError> {
    let calendar = app
        .db
        .create_calendar(user_device.user_id, &request.name, request.color.as_deref())
        .await?;
    println!("Got user device logged in: {}", user_device.device_id);
    println!("Created calendar: {}", calendar.id);

    Ok(Json(CalendarResponse {
        id: calendar.id,
        name: calendar.name,
    }))
}

pub async fn get_calendar(State(_db): AppState, Path(_id): Path<i64>) -> Json<CalendarResponse> {
    // TODO: Implement get calendar
    Json(CalendarResponse {
        id: 1,
        name: "Placeholder Calendar".to_string(),
    })
}

pub async fn update_calendar(
    State(_db): AppState,
    Path(_id): Path<i64>,
    Json(_request): Json<CreateCalendarRequest>,
) -> Json<CalendarResponse> {
    // TODO: Implement update calendar
    Json(CalendarResponse {
        id: 1,
        name: "Placeholder Calendar".to_string(),
    })
}

pub async fn delete_calendar(State(_db): AppState, Path(_id): Path<i64>) -> Json<()> {
    // TODO: Implement delete calendar
    Json(())
}

// Filter preview endpoint
pub async fn preview_filter(
    State(_db): AppState,
    Json(_request): Json<PreviewFilterRequest>,
) -> Json<PreviewFilterResponse> {
    // TODO: Implement preview filter
    Json(PreviewFilterResponse {
        original_events: vec![],
        transformed_events: vec![],
        processing_times: vec![],
        errors: vec![],
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateCalendarSourceRequest {
    calendar_id: i64,
    caldav_url: String,
}

pub async fn get_calendar_sources(
    State(app): AppState,
    Path(calendar_id): Path<i64>,
) -> Result<Json<Vec<CalendarSource>>, DecalidHttpError> {
    Ok(Json(app.db.list_calendar_sources(calendar_id).await?))
}

pub async fn create_calendar_source(
    State(app): AppState,
    Path(calendar_id): Path<i64>,
    Json(request): Json<CreateCalendarSourceRequest>,
) -> Result<Json<CalendarSource>, DecalidHttpError> {
    Ok(Json(
        app.db
            .create_calendar_source(calendar_id, &request.caldav_url)
            .await?,
    ))
}

pub async fn delete_calendar_source(
    State(app): AppState,
    Path(calendar_id): Path<i64>,
    Path(source_id): Path<i64>,
) -> Result<Json<()>, DecalidHttpError> {
    app.db
        .delete_calendar_source(calendar_id, source_id)
        .await?;
    Ok(Json(()))
}

mod login {
    use crate::caldav::server::DecalidHttpError;

    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub(super) struct LoginRequest {
        username: String,
        password: String,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub(super) struct LoginResponse {
        token: String,
    }

    pub(super) async fn login(
        State(_app): AppState,
        Json(_request): Json<LoginRequest>,
    ) -> Result<Json<LoginResponse>, DecalidHttpError> {
        // TODO: Implement login
        Ok(Json(LoginResponse {
            token: "placeholder".to_string(),
        }))
    }
}
pub(super) fn init_rest_router(config: &Config, db: Arc<Db>) -> Router {
    let layer = ServiceBuilder::new()
        .layer(HandleErrorLayer::new(|error: JWTErrors<_>| async move {
            error.into_response()
        }))
        .layer(middleware::JWTMiddlewareLayer::new(
            config.auth.clone(),
            db.clone(),
        ));
    Router::new()
        // REST endpoints
        .route("/shares/", routing::get(list_shares))
        .route("/shares/", routing::post(create_share))
        .route("/shares/{id}", routing::get(get_share))
        .route("/shares/{id}", routing::put(update_share))
        .route("/shares/{id}", routing::delete(delete_share))
        .route("/calendars/", routing::get(list_calendars))
        .route("/calendars/", routing::post(create_calendar))
        .route("/calendars/{id}", routing::get(get_calendar))
        .route("/calendars/{id}", routing::put(update_calendar))
        .route("/calendars/{id}", routing::delete(delete_calendar))
        .route(
            "/calendars/{id}/sources",
            routing::get(get_calendar_sources),
        )
        .route(
            "/calendars/{id}/sources",
            routing::post(create_calendar_source),
        )
        .route(
            "/calendars/{id}/sources/{source_id}",
            routing::delete(delete_calendar_source),
        )
        .route("/preview", routing::post(preview_filter))
        .layer(layer)
        .route("/login", routing::post(login::login))
        .with_state(AppStateImpl {
            db,
            config: config.auth.clone(),
        })
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use crate::auth;

    use super::*;

    #[tokio::test]
    async fn test_list_calendars_works_with_correct_authentication() -> Result<()> {
        let config = Config::test_config();
        let db = Arc::new(Db::new_in_memory().await);
        sqlx::migrate!().run(&db.0).await?;
        let router = init_rest_router(&config, db.clone());

        // Create a valid user & device
        let user = db.admin().create_user("test-user").await?;
        let device = db
            .users()
            .create_user_device(user.id, "test_device")
            .await?;

        // Create JWT
        let token = auth::jwt::generate_token(&config.auth, &device.device_id)?;

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/calendars/")
                    .header("Authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        assert!(response.status().is_success());
        Ok(())
    }

    #[tokio::test]
    async fn test_list_calendars_works_with_invalid_authentication() -> Result<()> {
        let config = Config::test_config();
        let db = Arc::new(Db::new_in_memory().await);
        sqlx::migrate!().run(&db.0).await?;
        let router = init_rest_router(&config, db.clone());

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/calendars/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await?;

        assert!(response.status().is_client_error());
        Ok(())
    }
}
