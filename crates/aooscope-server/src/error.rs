use aooscope_config::ConfigError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub struct ApiError(pub ConfigError);

impl From<ConfigError> for ApiError {
    fn from(value: ConfigError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let _ = self.0;
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "error": "config", "message": "configuration unavailable"})),
        )
            .into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
