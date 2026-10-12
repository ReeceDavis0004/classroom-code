use axum::{
    extract::{
        FromRequest,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    Unauthenticated,
    Forbidden,
    NotFound,
    PayloadTooLarge,
    Validation(Vec<FieldError>),
    Internal(String),
}

#[derive(Debug, Serialize)]
pub struct FieldError {
    pub field: &'static str,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn field(field: &'static str, code: &'static str, message: impl Into<String>) -> Self {
        Self::Validation(vec![FieldError {
            field,
            code,
            message: message.into(),
        }])
    }
}

#[derive(Serialize)]
struct Problem<'a> {
    #[serde(rename = "type")]
    kind: String,
    title: &'a str,
    status: u16,
    code: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    errors: Option<&'a [FieldError]>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, title) = match &self {
            Self::Unauthenticated => (StatusCode::UNAUTHORIZED, "unauthenticated", "Not signed in"),
            Self::Forbidden => (StatusCode::FORBIDDEN, "forbidden", "Forbidden"),
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found", "Not found"),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "Payload too large",
            ),
            Self::Validation(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "validation_failed",
                "Validation failed",
            ),
            Self::Internal(msg) => {
                eprintln!("internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal",
                    "Internal error",
                )
            }
        };
        let errors = match &self {
            Self::Validation(errors) => Some(errors.as_slice()),
            _ => None,
        };
        let body = serde_json::to_vec(&Problem {
            kind: format!("/problems/{code}"),
            title,
            status: status.as_u16(),
            code,
            errors,
        })
        .unwrap_or_default();
        (
            status,
            [(header::CONTENT_TYPE, "application/problem+json")],
            body,
        )
            .into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(err.to_string())
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            return Self::PayloadTooLarge;
        }
        Self::field("body", "invalid", rejection.body_text())
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        Self::field("query", "invalid", rejection.body_text())
    }
}

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct Json<T>(pub T);
