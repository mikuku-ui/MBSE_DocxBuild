//! 文档装配 HTTP handlers：GET assemble（按项目现算，文档正文投影）。

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::api::http::AppState;
use crate::application::assembly::service::ResolvedDocument;
use crate::application::assembly::AssemblyService;
use crate::application::ServiceError;

fn svc(state: &AppState) -> Arc<AssemblyService> {
    state.assemblies.clone()
}

/// assemble 的查询参数：装配必须的项目 + 可选人工槽执行实例。
#[derive(Debug, Deserialize)]
pub struct AssembleParams {
    pub project_id: Uuid,
    #[serde(default)]
    pub instance_id: Option<Uuid>,
}

/// GET /api/documents/templates/{id}/assemble?project_id=…&instance_id=…
/// → ResolvedDocument（模板元信息 + 文档序正文块序列）。
pub async fn assemble(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<AssembleParams>,
) -> Result<Json<ResolvedDocument>, ServiceError> {
    Ok(Json(
        svc(&state)
            .assemble(id, params.project_id, params.instance_id)
            .await?,
    ))
}
