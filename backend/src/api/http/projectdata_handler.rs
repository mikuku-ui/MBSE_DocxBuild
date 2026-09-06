//! 项目数据 HTTP handlers：GET/PUT /api/projects/{id}/data（项目级一份的值）。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::api::http::AppState;
use crate::application::projectdata::ProjectDataService;
use crate::application::ServiceError;
use crate::domain::projectdata::{ProjectDataView, ProjectDataWrite};

fn svc(state: &AppState) -> Arc<ProjectDataService> {
    state.project_data.clone()
}

/// GET /api/projects/{id}/data → 数据视图（项目绑定体系的结构 × 已填值）。
/// 项目未绑体系 → scheme_id=null + 空结构。
pub async fn get_project_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectDataView>, ServiceError> {
    Ok(Json(svc(&state).view(id).await?))
}

/// PUT /api/projects/{id}/data → 整存替换（执行清单 / 项目页共用的填数面），
/// 返回存后的数据视图。
pub async fn replace_project_data(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ProjectDataWrite>,
) -> Result<Json<ProjectDataView>, ServiceError> {
    Ok(Json(svc(&state).replace(id, body).await?))
}
