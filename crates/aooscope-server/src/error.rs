use aooscope_config::ConfigError;
use aooscope_display::DisplayError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    Config(ConfigError),
    Display(DisplayError),
    BadRequest(&'static str),
}

impl From<ConfigError> for ApiError {
    fn from(value: ConfigError) -> Self {
        Self::Config(value)
    }
}

impl From<DisplayError> for ApiError {
    fn from(value: DisplayError) -> Self {
        Self::Display(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = if matches!(self, Self::BadRequest(_)) {
            StatusCode::BAD_REQUEST
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        let (error, message) = match self {
            Self::Config(_) => ("config", "configuration unavailable"),
            Self::Display(_) => ("display", "display unavailable"),
            Self::BadRequest(message) => ("invalid_request", message),
        };
        (
            status,
            Json(json!({"ok": false, "error": error, "message": message})),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
