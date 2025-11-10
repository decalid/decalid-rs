use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::any,
    Router,
};

async fn wellknown_redirect_to_shareroot(request: Request<Body>) -> impl IntoResponse {
    if request.method().as_str().eq_ignore_ascii_case("PROPFIND") {
        Response::builder()
            .header("Location", "/shares/")
            .status(StatusCode::FOUND)
            .body(Body::empty())
            .unwrap()
    } else {
        Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Body::empty())
            .unwrap()
    }
}

pub(super) fn init_wellknown_router() -> Router {
    Router::new().route("/caldav", any(wellknown_redirect_to_shareroot))
}
