//! Input limits and the JSON extractor that reports violations as `{error}` with a clear message.

use super::auth::ApiError;
use crate::model::{Image, NewReply, NewThread, RegisterArtifact, Submit};
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, Request};
use axum::http::StatusCode;
use serde::de::DeserializeOwned;

/// Whole request body. Generous for text; images (planned) will get their own route.
pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
/// One comment or reply. Agent replies are markdown and can run long.
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;
/// One anchor, serialized. Quotes plus context and a selector fit easily.
pub const MAX_ANCHOR_BYTES: usize = 16 * 1024;
/// New threads plus replies in one submit.
pub const MAX_SUBMIT_ITEMS: usize = 100;
/// One region screenshot, decoded. Overlay crops are well under this.
pub const MAX_IMAGE_BYTES: usize = 512 * 1024;
/// Image types we store and serve back.
pub const IMAGE_TYPES: &[&str] = &["image/png", "image/jpeg", "image/webp"];
/// A progress note on a status change.
pub const MAX_NOTE_BYTES: usize = 1024;
/// Artifact title, uri, kind, id, session.
pub const MAX_FIELD_BYTES: usize = 2048;

/// Request-side `axum::Json`, but a rejection (bad JSON, wrong shape, body over [`MAX_REQUEST_BYTES`])
/// becomes our `{error}` body instead of axum's plain text.
pub struct JsonIn<T>(pub T);

impl<T: DeserializeOwned, S: Send + Sync> FromRequest<S> for JsonIn<T> {
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, ApiError> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(v)) => Ok(JsonIn(v)),
            Err(e) => Err(rejection(&e)),
        }
    }
}

fn rejection(e: &JsonRejection) -> ApiError {
    let status = e.status();
    let msg = if status == StatusCode::PAYLOAD_TOO_LARGE {
        format!("request body over {} KiB", MAX_REQUEST_BYTES / 1024)
    } else {
        e.body_text()
    };
    ApiError::new(status, msg)
}

fn too_big(what: &str, len: usize, max: usize) -> ApiError {
    ApiError::new(StatusCode::PAYLOAD_TOO_LARGE, format!("{what} is {len} bytes; the limit is {max}"))
}

/// A comment or reply body: non-empty, at most [`MAX_MESSAGE_BYTES`].
pub fn message(body: &str) -> Result<(), ApiError> {
    if body.trim().is_empty() {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "empty message".into()));
    }
    if body.len() > MAX_MESSAGE_BYTES {
        return Err(too_big("message", body.len(), MAX_MESSAGE_BYTES));
    }
    Ok(())
}

pub fn note(note: Option<&str>) -> Result<(), ApiError> {
    match note {
        Some(n) if n.len() > MAX_NOTE_BYTES => Err(too_big("note", n.len(), MAX_NOTE_BYTES)),
        _ => Ok(()),
    }
}

fn image(img: &Image) -> Result<(), ApiError> {
    if !IMAGE_TYPES.contains(&img.mime.as_str()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            format!("image type {:?}; expected one of {IMAGE_TYPES:?}", img.mime),
        ));
    }
    // Decoded size of base64 without decoding it twice (the store decodes).
    let n = img.data.len() / 4 * 3;
    if n > MAX_IMAGE_BYTES { Err(too_big("image", n, MAX_IMAGE_BYTES)) } else { Ok(()) }
}

fn thread(t: &NewThread) -> Result<(), ApiError> {
    message(&t.body)?;
    if let Some(img) = &t.image {
        image(img)?;
    }
    let n = t.anchor.to_string().len();
    if n > MAX_ANCHOR_BYTES { Err(too_big("anchor", n, MAX_ANCHOR_BYTES)) } else { Ok(()) }
}

fn reply(r: &NewReply) -> Result<(), ApiError> {
    message(&r.body)
}

pub fn submit(s: &Submit) -> Result<(), ApiError> {
    let n = s.threads.len() + s.replies.len();
    if n == 0 {
        return Err(ApiError::new(StatusCode::BAD_REQUEST, "nothing to submit".into()));
    }
    if n > MAX_SUBMIT_ITEMS {
        return Err(too_big("submit item count", n, MAX_SUBMIT_ITEMS).with_status(StatusCode::BAD_REQUEST));
    }
    s.threads.iter().try_for_each(thread)?;
    s.replies.iter().try_for_each(reply)
}

pub fn register(r: &RegisterArtifact) -> Result<(), ApiError> {
    let fields = [Some(&r.title), r.uri.as_ref(), r.kind.as_ref(), r.id.as_ref(), r.session.as_ref()];
    match fields.into_iter().flatten().find(|f| f.len() > MAX_FIELD_BYTES) {
        Some(f) => Err(too_big("artifact field", f.len(), MAX_FIELD_BYTES)),
        None => Ok(()),
    }
}
