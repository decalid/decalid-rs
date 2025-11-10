#![allow(clippy::result_large_err)]

use axum::body::Body;

use crate::{caldav::propfind, db::Db};

use super::{webdav_response_builder, DecalidHttpError};

fn generic_error500(e: anyhow::Error) -> DecalidHttpError {
    log::warn!("Got an error 500 due to: {e:?}");
    DecalidHttpError(
        webdav_response_builder()
            .status(500)
            .body(Body::from("Internal Server Error"))
            .unwrap(),
    )
}

pub(super) async fn share_exists(db: &Db, share_id: &str) -> Result<(), DecalidHttpError> {
    let shares_db = db.shares(share_id);

    match shares_db.check_exists().await {
        Ok(true) => Ok(()),
        Ok(false) => Err(DecalidHttpError(
            webdav_response_builder()
                .status(404)
                .body(Body::from("Share not found"))?,
        )),
        Err(e) => Err(generic_error500(e.into())),
    }
}

pub(crate) async fn share_calendar_exists(
    db: &Db,
    share_id: &str,
    calendar_id: &str,
) -> Result<(), DecalidHttpError> {
    match db.shares(share_id).check_exists_calendar(calendar_id).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(DecalidHttpError(
            webdav_response_builder()
                .status(404)
                .body(Body::from("Share not found"))?,
        )),
        Err(e) => Err(generic_error500(e.into())),
    }
}

pub(crate) fn propfind_payload_is_valid(
    payload: &propfind::Propfind,
) -> Result<(), DecalidHttpError> {
    // Only one of the following should be present:
    // - allprop
    // - propname
    // - prop
    // If more than one is present, return a 400 error
    // If none are present, return a 400 error
    let mut found = 0;
    if payload.allprop.is_some() {
        found += 1;
    }
    if payload.propname.is_some() {
        found += 1;
    }
    if !payload.prop.is_empty() {
        found += 1;
    }
    if found > 1 {
        return Err(DecalidHttpError(
            webdav_response_builder()
                .status(400)
                .body(Body::from("Multiple properties are not allowed"))?,
        ));
    }
    if found == 0 {
        return Err(DecalidHttpError(
            webdav_response_builder()
                .status(400)
                .body(Body::from("Empty propfind body"))?,
        ));
    }
    Ok(())
}
