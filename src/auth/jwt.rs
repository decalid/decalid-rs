//! JWT authentication
//!
//! This module contains the JWT authentication logic.

use crate::{auth::Claims, config::AuthConfig};
use anyhow::{Context, Result};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use std::time::{SystemTime, UNIX_EPOCH};

/// Generate a JWT token
pub fn generate_token(config: &AuthConfig, device_id: &str) -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("Failed to get system time")?
        .as_secs();

    let claims = Claims {
        sub: device_id.to_string(),
        iat: now,
        exp: now + config.jwt_expiration_seconds,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .context("Failed to generate JWT token")?;

    Ok(token)
}

/// Validate a JWT token
pub fn validate_token(config: &AuthConfig, token: &str) -> Result<Claims> {
    let validation = Validation::default();
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )
    .context("Failed to validate JWT token")?;

    Ok(token_data.claims)
}

pub mod middleware {
    use std::sync::Arc;

    use axum::{
        extract::Request,
        http::StatusCode,
        response::{IntoResponse, Response},
    };
    use futures::future::BoxFuture;
    use futures::TryFutureExt;
    use tower::{Layer, Service};

    use crate::{caldav::server::DecalidHttpError, config::AuthConfig, db::Db};

    use super::validate_token;

    #[derive(Clone)]
    pub struct JWTMiddlewareLayer {
        pub config: AuthConfig,
        pub db: Arc<Db>,
    }

    impl JWTMiddlewareLayer {
        pub fn new(config: AuthConfig, db: Arc<Db>) -> Self {
            Self { config, db }
        }
    }

    impl<S> Layer<S> for JWTMiddlewareLayer {
        type Service = JWTMiddlewareService<S>;

        fn layer(&self, inner: S) -> Self::Service {
            JWTMiddlewareService {
                inner,
                config: self.config.clone(),
                db: self.db.clone(),
            }
        }
    }

    #[derive(Clone)]
    pub struct JWTMiddlewareService<S> {
        pub inner: S,
        pub(crate) config: AuthConfig,
        pub(crate) db: Arc<Db>,
    }

    #[derive(Debug)]
    pub enum JWTErrors<E> {
        NoToken,
        InvalidToken,
        InvalidTokenFormat,
        UnknownInternalError,
        InternalError(E),
    }
    impl<E> JWTErrors<E> {
        fn transmute<X>(&self) -> JWTErrors<X> {
            match self {
                JWTErrors::InternalError(_) => JWTErrors::UnknownInternalError,
                JWTErrors::UnknownInternalError => JWTErrors::UnknownInternalError,
                JWTErrors::NoToken => JWTErrors::NoToken,
                JWTErrors::InvalidToken => JWTErrors::InvalidToken,
                JWTErrors::InvalidTokenFormat => JWTErrors::InvalidTokenFormat,
            }
        }
    }

    impl<E: std::error::Error + Send + Sync + 'static> IntoResponse for JWTErrors<E> {
        fn into_response(self) -> Response {
            match self {
                JWTErrors::NoToken => DecalidHttpError::from_str("No token provided")
                    .with_status(StatusCode::UNAUTHORIZED),
                JWTErrors::InvalidToken => DecalidHttpError::from_str("Invalid token provided")
                    .with_status(StatusCode::FORBIDDEN),
                JWTErrors::InvalidTokenFormat => DecalidHttpError::from_str("Invalid token format")
                    .with_status(StatusCode::BAD_REQUEST),
                JWTErrors::InternalError(e) => DecalidHttpError::from(e),
                JWTErrors::UnknownInternalError => {
                    DecalidHttpError::from_str("Unknown internal error")
                }
            }
            .into_response()
        }
    }

    impl<E: Sync + Send + std::fmt::Display> std::fmt::Display for JWTErrors<E> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                JWTErrors::NoToken => write!(f, "No token provided"),
                JWTErrors::InvalidToken => write!(f, "Invalid token"),
                JWTErrors::InvalidTokenFormat => write!(f, "Invalid token format"),
                JWTErrors::InternalError(e) => write!(f, "Internal error: {e}"),
                JWTErrors::UnknownInternalError => write!(f, "Unknown internal error"),
            }
        }
    }

    impl std::error::Error for JWTErrors<anyhow::Error> {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                JWTErrors::InternalError(e) => e.source(),
                _ => None,
            }
        }
    }

    #[derive(Debug)]
    pub struct JWTMiddlewareResponse<R>(Result<R, JWTErrors<()>>);

    impl<R: IntoResponse> IntoResponse for JWTMiddlewareResponse<R> {
        fn into_response(self) -> Response {
            match self.0 {
                Ok(r) => r.into_response(),
                Err(e) => e.transmute::<hyper::Error>().into_response(),
            }
        }
    }

    impl<S> Service<Request> for JWTMiddlewareService<S>
    where
        S: Service<Request, Response = Response> + Clone + Send + 'static,
        S::Future: Send + 'static,
    {
        type Response = JWTMiddlewareResponse<S::Response>;

        type Error = JWTErrors<S::Error>;

        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

        fn poll_ready(
            &mut self,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            match self.inner.poll_ready(cx) {
                std::task::Poll::Ready(Ok(())) => std::task::Poll::Ready(Ok(())),
                std::task::Poll::Ready(Err(e)) => {
                    std::task::Poll::Ready(Err(JWTErrors::InternalError(e)))
                }
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        }

        fn call(&mut self, mut req: Request) -> Self::Future {
            let config = self.config.clone();
            let token = req
                .headers()
                .get("Authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .map(str::to_owned);

            let Some(token) = token else {
                println!("No token provided");
                return Box::pin(async move { Ok(JWTMiddlewareResponse(Err(JWTErrors::NoToken))) });
            };

            // Now we validate the token as a JWT
            if let Ok(claims) = validate_token(&config, &token) {
                println!("Token is valid");
                let device_id = claims.sub;
                let db = self.db.clone();
                let mut inner: S = self.inner.clone();

                Box::pin(async move {
                    let users_db = db.users();
                    let device = users_db
                        .get_user_device(&device_id)
                        .map_err(|_e| JWTErrors::InternalError(()))
                        .await;

                    if let Ok(device) = device {
                        req.extensions_mut().insert(device);
                    } else {
                        // Device no longer exists!
                        return Ok(JWTMiddlewareResponse(Err(JWTErrors::InvalidToken)));
                    }

                    let future = inner.call(req);

                    match future.await {
                        Ok(result) => Ok(JWTMiddlewareResponse(Ok(result))),
                        Err(e) => Err(JWTErrors::InternalError(e)),
                    }
                })
            } else {
                Box::pin(async move { Ok(JWTMiddlewareResponse(Err(JWTErrors::InvalidToken))) })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{auth::jwt::middleware::JWTMiddlewareService, db::Db};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        response::{IntoResponse, Response},
    };
    use std::sync::Arc;
    use tower::{Service, ServiceBuilder, ServiceExt};

    #[derive(Clone)]
    struct MockService;

    impl Service<Request<Body>> for MockService {
        type Response = Response<Body>;
        type Error = anyhow::Error;
        type Future = std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
        >;

        fn poll_ready(
            &mut self,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn call(&mut self, _: Request<Body>) -> Self::Future {
            Box::pin(async {
                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap())
            })
        }
    }

    #[tokio::test]
    async fn test_jwt_token_flow() {
        let secret = "test_secret";
        let user_id = "test_user";
        let expiration = 3600; // 1 hour

        let config = AuthConfig {
            jwt_secret: secret.to_string(),
            jwt_expiration_seconds: expiration,
            ..Default::default()
        };

        // Generate token
        let token = generate_token(&config, user_id).unwrap();
        assert!(!token.is_empty());

        // Validate token
        let claims = validate_token(&config, &token).unwrap();
        assert_eq!(claims.sub, user_id);

        // Verify expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(claims.exp > now);
        assert!(claims.exp <= now + expiration);
    }

    #[tokio::test]
    async fn test_jwtmiddleware_valid_token() -> Result<(), anyhow::Error> {
        let config = AuthConfig {
            jwt_secret: "test_secret".to_string(),
            jwt_expiration_seconds: 3600,
            ..Default::default()
        };

        // Create a mock database that will return success for our test device
        let db = Arc::new(Db::new_in_memory().await);
        sqlx::migrate!().run(&db.0).await?;
        let users_db = db.users();
        let user = db.admin().create_user("user-testuser").await?;
        let device = users_db.create_user_device(user.id, "Test Device").await?;
        let token = generate_token(&config, &device.device_id)?;

        // Create the service stack with our middleware
        let service = ServiceBuilder::new()
            .layer_fn(|inner| JWTMiddlewareService {
                inner,
                config: config.clone(),
                db: db.clone(),
            })
            .service(MockService);

        // Test valid token
        let request = Request::builder()
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;

        let response = service
            .clone()
            .oneshot(request)
            .await
            .expect("Service should succeed with valid token")
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // Test invalid token
        let request = Request::builder()
            .header("Authorization", "Bearer invalid_token")
            .body(Body::empty())
            .unwrap();

        let result = service.clone().oneshot(request).await;
        assert!(
            result.is_ok_and(|r| r.into_response().status().is_client_error()),
            "Result should return client error with invalid token"
        );

        // Test missing token
        let request = Request::builder().body(Body::empty()).unwrap();

        let result = service.oneshot(request).await;
        assert!(
            result.is_ok_and(|r| r.into_response().status().is_client_error()),
            "Result should return client error with missing token"
        );

        Ok(())
    }
}
