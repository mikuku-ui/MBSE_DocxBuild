//! HTTP 层：DTO 组装 + 错误映射。HTTP 不碰查询、不碰业务规则。

mod assembly_handler;
mod document_handler;
mod dto;
mod execution_handler;
mod project_handler;
mod projectdata_handler;
mod routes;
mod scheme_handler;
mod trace_handler;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::application::ServiceError;

impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            ServiceError::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            ServiceError::Validation(_) => (StatusCode::BAD_REQUEST, "validation_error"),
            ServiceError::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            ServiceError::Repository(_) => (StatusCode::INTERNAL_SERVER_ERROR, "repository_error"),
        };
        let body = Json(serde_json::json!({
            "error": code,
            "message": self.to_string(),
        }));
        (status, body).into_response()
    }
}

pub use routes::{router, AppState};
