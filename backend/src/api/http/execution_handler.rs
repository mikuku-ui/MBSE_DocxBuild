//! 执行 HTTP handlers。

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::{CreateExecution, ProjectScope, UpdateExecution, UpdateExecutionItem};
use crate::api::http::AppState;
use crate::application::execution::ExecutionService;
use crate::application::ServiceError;

fn svc(state: &AppState) -> Arc<ExecutionService> {
    state.executions.clone()
}

pub async fn list_instances(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
) -> Result<Json<Vec<crate::domain::execution::ExecutionInstance>>, ServiceError> {
    Ok(Json(svc(&state).list_instances(scope.project_id).await?))
}

pub async fn create_instance(
    State(state): State<AppState>,
    Json(body): Json<CreateExecution>,
) -> Result<Json<crate::domain::execution::ExecutionInstance>, ServiceError> {
    Ok(Json(
        svc(&state)
            .create_instance(body.project_id, body.template_id, body.stage)
            .await?,
    ))
}

pub async fn get_instance_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let (instance, items) = svc(&state).get_instance_detail(id).await?;
    Ok(Json(serde_json::json!({
        "instance": instance,
        "items": items,
    })))
}

pub async fn update_instance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateExecution>,
) -> Result<Json<crate::domain::execution::ExecutionInstance>, ServiceError> {
    Ok(Json(
        // stage: None = 不改；Some(s) = 设为 s（与 trace 的 REPLACE-ONLY 约定一致）
        svc(&state)
            .update_instance(id, body.status, body.stage.map(Some))
            .await?,
    ))
}

pub async fn list_items(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<crate::domain::execution::ExecutionItem>>, ServiceError> {
    Ok(Json(svc(&state).list_items(id).await?))
}

pub async fn update_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateExecutionItem>,
) -> Result<Json<crate::domain::execution::ExecutionItem>, ServiceError> {
    Ok(Json(svc(&state).update_item(id, body.status, body.content).await?))
}
