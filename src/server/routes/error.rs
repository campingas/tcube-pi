use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub(crate) const SESSION_COOKIE_NAME: &str = "tcube_session";

#[derive(Debug)]
pub(crate) struct ApiError {
    status: StatusCode,
    detail: String,
}

impl ApiError {
    pub(crate) fn bad_request(error: impl ToString) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error)
    }

    pub(crate) fn unauthorized(error: impl ToString) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, error)
    }

    pub(crate) fn server(error: impl ToString) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error)
    }

    pub(crate) fn conflict(error: impl ToString) -> Self {
        Self::new(StatusCode::CONFLICT, error)
    }

    fn new(status: StatusCode, error: impl ToString) -> Self {
        Self {
            status,
            detail: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, axum::Json(json!({ "detail": self.detail }))).into_response()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SessionCookie(pub(crate) Option<String>);

impl<S> FromRequestParts<S> for SessionCookie
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .and_then(|header| cookie_value(header, SESSION_COOKIE_NAME))
            .map(str::to_string);
        Ok(Self(token))
    }
}

fn cookie_value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|part| {
        let (key, value) = part.trim().split_once('=')?;
        if key == name {
            Some(value)
        } else {
            None
        }
    })
}
