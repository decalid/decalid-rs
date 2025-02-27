//! REST API endpoints
//!
//! This module contains the REST API endpoints for the application.

use std::sync::Arc;

use axum::{
    error_handling::HandleErrorLayer,
    extract::{Path, State},
    response::IntoResponse,
    routing, Json, Router,
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;

use crate::{auth::jwt::middleware, caldav::server::DecalidHttpError, config::Config, db::Db};

/// API state containing the database connection
#[derive(Clone)]
pub(crate) struct AppStateImpl {
    db: Arc<Db>,
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
    user_id: i64,
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
pub async fn list_calendars(State(_db): AppState) -> Json<Vec<CalendarResponse>> {
    // TODO: Implement list calendars
    Json(vec![])
}

pub async fn create_calendar(
    State(_db): AppState,
    Json(_request): Json<CreateCalendarRequest>,
) -> Json<CalendarResponse> {
    // TODO: Implement create calendar
    Json(CalendarResponse {
        id: 1,
        name: "Placeholder Calendar".to_string(),
    })
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

mod login {
    use crate::caldav::server::DecalidHttpError;
    use axum::response::IntoResponse;

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

    pub(super) async fn signup(
        State(_app): AppState,
        Json(_request): Json<LoginRequest>,
    ) -> Result<Json<LoginResponse>, DecalidHttpError> {
        // TODO: Implement signup
        Ok(Json(LoginResponse {
            token: "placeholder".to_string(),
        }))
    }
}
pub(super) fn init_rest_router(config: &Config, db: Arc<Db>) -> Router {
    Router::new()
        // REST endpoints
        .route("/api/shares", routing::get(list_shares))
        .route("/api/shares", routing::post(create_share))
        .route("/api/shares/:id", routing::get(get_share))
        .route("/api/shares/:id", routing::put(update_share))
        .route("/api/shares/:id", routing::delete(delete_share))
        .route("/api/calendars", routing::get(list_calendars))
        .route("/api/calendars", routing::post(create_calendar))
        .route("/api/calendars/:id", routing::get(get_calendar))
        .route("/api/calendars/:id", routing::put(update_calendar))
        .route("/api/calendars/:id", routing::delete(delete_calendar))
        .route("/api/preview", routing::post(preview_filter))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::JWTMiddlewareLayer::new(config.auth.clone(), db.clone())),
        )
        .route("/api/signup", routing::post(login::signup))
        .route("/api/login", routing::post(login::login))
        .with_state(AppStateImpl { db })
}
