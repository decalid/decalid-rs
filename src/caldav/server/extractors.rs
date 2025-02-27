use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::HeaderMap,
};
use yaserde::YaDeserialize;

use super::DecalidHttpError;

pub(super) struct XmlExtract<T>(pub T);

enum XmlExtractErrors {
    MissingXmlContentType,
    NotValidUtf8,
    CannotParse(String),
}

impl From<XmlExtractErrors> for DecalidHttpError {
    fn from(value: XmlExtractErrors) -> Self {
        match value {
            XmlExtractErrors::MissingXmlContentType => {
                DecalidHttpError::from_str("Missing content-type or not application/xml")
            }
            XmlExtractErrors::NotValidUtf8 => DecalidHttpError::from_str("Not a valid utf-8 body"),
            XmlExtractErrors::CannotParse(reason) => {
                DecalidHttpError::from_str(&format!("Cannot parse into the expected body: {}", reason))
            }
        }
    }
}

impl<T, S> FromRequest<S> for XmlExtract<T>
where
    T: YaDeserialize,
    S: Send + Sync,
{
    type Rejection = DecalidHttpError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        use XmlExtractErrors::*;

        if xml_content_type(req.headers()) {
            let bytes = Bytes::from_request(req, state).await?;
            let str = String::from_utf8(bytes.to_vec()).map_err(|_| NotValidUtf8)?;
            println!("Valid Thing!");
            Ok(Self::from_str(&str)?)
        } else {
            println!("INVALID CONTENT TYPE");
            Err(MissingXmlContentType.into())
        }
    }
}

fn xml_content_type(headers: &HeaderMap) -> bool {
    if let Some(content_type) = headers.get(axum::http::header::CONTENT_TYPE) {
        if let Ok(content_type_str) = content_type.to_str() {
            return content_type_str
                .trim()
                .to_lowercase()
                .starts_with("application/xml");
        }
    }
    false
}

impl<T> XmlExtract<T>
where
    T: YaDeserialize,
{
    /// Construct a `Json<T>` from a byte slice. Most users should prefer to use the `FromRequest` impl
    /// but special cases may require first extracting a `Request` into `Bytes` then optionally
    /// constructing a `Json<T>`.
    pub fn from_str(str: &str) -> Result<Self, DecalidHttpError> {
        let thing = XmlExtract(yaserde::de::from_str(str).map_err(|e| {
            log::warn!("Could not correctly parse the body: {:?}", e);
            XmlExtractErrors::CannotParse(e)
        })?);
        println!("Correctly parsed");
        Ok(thing)
    }
}
