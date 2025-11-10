use axum::{extract::FromRequestParts, http::request::Parts};
use reqwest::Url;
use serde::Serialize;

#[derive(Debug)]
pub struct Paginated<R> {
    pub limit: u32,
    pub offset: u32,
    total: u32,
    base_url: Option<Url>,
    data: Vec<R>,
}

impl<R: Serialize> Serialize for Paginated<R> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        PaginatedToRender::from(self).serialize(serializer)
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedToRender<'a, R> {
    limit: u32,
    offset: u32,
    total: u32,
    next: Option<String>,
    prev: Option<String>,
    data: &'a Vec<R>,
}

impl<'a, R: Serialize> From<&'a Paginated<R>> for PaginatedToRender<'a, R> {
    fn from(paginated: &'a Paginated<R>) -> Self {
        let (prev, next) = paginated.get_prev_next();
        Self {
            limit: paginated.limit,
            offset: paginated.offset,
            total: paginated.total,
            next,
            prev,
            data: &paginated.data,
        }
    }
}

impl<S: Send + Sync, R> FromRequestParts<S> for Paginated<R> {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Look for limit/offset in query parameters. Leave other params empty.
        println!("Parts: {parts:#?}");
        // parts.uri does not contain schema/authority, only path (because it does what hyper does, which is the "/" that the client puts on "GET / HTTP/1.1")
        let url: Result<Url, anyhow::Error> = (|| {
            let authority = parts
                .headers
                .get("host")
                .ok_or(anyhow::anyhow!("No host header"))?
                .to_str()?;
            let mut schema = "http";
            // if HTTPS is set in the headers, use it
            if parts.headers.get("https").is_some() {
                schema = "https";
            }

            Ok(Url::parse(&format!(
                "{}://{}{}",
                schema,
                authority,
                parts.uri.path()
            ))?)
        })();

        let query_parts = parts
            .uri
            .query()
            .unwrap_or("")
            .split('&')
            .collect::<Vec<&str>>();
        let mut limit = 10;
        let mut offset = 0;
        for param in query_parts {
            if let Some((key, value)) = param.split_once('=') {
                if key == "limit" {
                    limit = value.parse().unwrap_or(10);
                } else if key == "offset" {
                    offset = value.parse().unwrap_or(0);
                }
            }
        }
        let paginated = Paginated::new(limit, offset, 0);

        if let Ok(url) = url {
            Ok(paginated.with_base_url(&url))
        } else {
            Ok(paginated)
        }
    }
}

impl<R> Paginated<R> {
    pub fn new(limit: u32, offset: u32, total: u32) -> Self {
        Self {
            limit,
            offset,
            total,
            base_url: None,
            data: Vec::new(),
        }
    }

    /// Set the body and pagination metadata
    pub fn with_full_body(mut self, data: impl IntoIterator<Item = R> + ExactSizeIterator) -> Self {
        self.total = data.len() as u32;
        self.data = data
            .into_iter()
            .skip(self.offset as usize)
            .take(self.limit as usize)
            .collect();
        self
    }

    /// Only set the body, not the pagination metadata
    pub fn with_body(mut self, data: impl IntoIterator<Item = R>) -> Self {
        self.data = data.into_iter().collect();
        self
    }

    pub fn with_base_url(mut self, base_url: &Url) -> Self {
        self.base_url = Some(base_url.clone());
        self
    }

    fn get_prev_next(&self) -> (Option<String>, Option<String>) {
        if let Some(base_url) = &self.base_url {
            let prev = if self.offset > 0 {
                base_url
                    .join(&format!(
                        "?limit={}&offset={}",
                        self.limit,
                        self.offset - self.limit
                    ))
                    .ok()
                    .map(|u| u.to_string())
            } else {
                None
            };
            let next = if self.offset + self.limit < self.total {
                base_url
                    .join(&format!(
                        "?limit={}&offset={}",
                        self.limit,
                        self.offset + self.limit
                    ))
                    .ok()
                    .map(|u| u.to_string())
            } else {
                None
            };
            (prev, next)
        } else {
            (None, None)
        }
    }
}
