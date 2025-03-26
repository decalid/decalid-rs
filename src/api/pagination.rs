use reqwest::Url;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Paginated<R> {
    limit: u32,
    offset: u32,
    total: u32,
    next: Option<String>,
    prev: Option<String>,
    data: Vec<R>,
}

impl<R> Paginated<R> {
    pub fn new(limit: u32, offset: u32, total: u32) -> Self {
        Self {
            limit,
            offset,
            total,
            next: None,
            prev: None,
            data: Vec::new(),
        }
    }

    pub fn with_body(mut self, data: Vec<R>) -> Self {
        self.data = data;
        self
    }

    pub fn with_base_url(mut self, base_url: Url) -> Self {
        self.next = base_url
            .join(&format!("?limit={}&offset={}", self.limit, self.offset))
            .ok()
            .map(|u| u.to_string());
        self.prev = base_url
            .join(&format!(
                "?limit={}&offset={}",
                self.limit,
                self.offset - self.limit
            ))
            .ok()
            .map(|u| u.to_string());
        self
    }
}
