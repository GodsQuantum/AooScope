#![forbid(unsafe_code)]

pub mod app;
pub mod assets;
pub mod dto;
pub mod error;
pub mod metrics;
pub mod openapi;
pub mod routes;
pub mod state;

pub use app::app;
pub use dto::StatusDto;
pub use openapi::ApiDoc;
pub use state::AppState;

pub const APP_VERSION: &str = "0.3.0-dev";
