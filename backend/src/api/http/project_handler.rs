//! 项目 HTTP handlers：CRUD + 绑定/解绑体系。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::{BindScheme, CreateProject, RenameProject};
use crate::api::http::AppState;
use crate::application::project::ProjectService;
use crate::application::ServiceError;
use crate::domain::project::Project;

fn svc(state: &AppState) -> Arc<ProjectService> {
    state.projects.clone()
}

pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<Project>>, ServiceError> {
    Ok(Json(svc(&state).list().await?))
}

pub async fn create_project(
    State(state): State<AppState>,
    Json(body): Json<CreateProject>,
) -> Result<Json<Project>, ServiceError> {
    Ok(Json(svc(&state).create(body.name, body.scheme_id).await?))
}

pub async fn rename_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<RenameProject>,
) -> Result<Json<Project>, ServiceError> {
    Ok(Json(svc(&state).rename(id, body.name).await?))
}

/// PUT /api/projects/{id}/scheme → 绑定/解绑体系（换体系会清空该项目已填数据）。
pub async fn bind_scheme(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<BindScheme>,
) -> Result<Json<Project>, ServiceError> {
    Ok(Json(svc(&state).bind_scheme(id, body.scheme_id).await?))
}

pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    svc(&state).delete(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
