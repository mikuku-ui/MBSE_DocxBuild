//! 体系（台账编排）HTTP handlers。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::api::http::AppState;
use crate::application::scheme::SchemeService;
use crate::application::ServiceError;
use crate::domain::scheme::{Scheme, SchemeFull, SchemeSpec};

fn svc(state: &AppState) -> Arc<SchemeService> {
    state.schemes.clone()
}

/// GET /api/schemes → 体系清单（元数据，不含结构）。
pub async fn list_schemes(
    State(state): State<AppState>,
) -> Result<Json<Vec<Scheme>>, ServiceError> {
    Ok(Json(svc(&state).list().await?))
}

/// POST /api/schemes → 新建一套体系，返回完整定义。
pub async fn create_scheme(
    State(state): State<AppState>,
    Json(body): Json<SchemeSpec>,
) -> Result<Json<SchemeFull>, ServiceError> {
    Ok(Json(svc(&state).create(body).await?))
}

/// GET /api/schemes/{id} → 完整定义（含 id）。
pub async fn get_scheme(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SchemeFull>, ServiceError> {
    Ok(Json(svc(&state).get(id).await?))
}

/// PUT /api/schemes/{id} → 按自然键整存替换（保 id，项目值不丢），返回新定义。
pub async fn replace_scheme(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SchemeSpec>,
) -> Result<Json<SchemeFull>, ServiceError> {
    Ok(Json(svc(&state).replace(id, body).await?))
}

/// DELETE /api/schemes/{id} → 删除体系（被项目绑定 → 409）。
pub async fn delete_scheme(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    svc(&state).delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
