//! 追踪图 HTTP handlers。除追溯（reachable）外，每个接口都带项目作用域
//! `?project_id=<uuid>`（`ProjectScope`）；reachable 复用携带 project_id 的 `ReachParams`。

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::{
    CreateBaseline, CreateTestItem, CreateTraceLink, CreateTraceNode, IngestRequest, ProjectScope,
    ReachParams, ReorderRequest, UpdateTraceNode, UploadRequirementsRequest,
};
use crate::api::http::AppState;
use crate::application::trace::graph::Direction;
use crate::application::trace::service::TraceService;
use crate::application::ServiceError;
use crate::domain::trace::TraceNode;

fn svc(state: &AppState) -> Arc<TraceService> {
    state.trace.clone()
}

pub async fn list_nodes(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
) -> Result<Json<Vec<TraceNode>>, ServiceError> {
    Ok(Json(svc(&state).list_nodes(scope.project_id).await?))
}

pub async fn get_node(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Path(id): Path<Uuid>,
) -> Result<Json<TraceNode>, ServiceError> {
    Ok(Json(svc(&state).get_node(scope.project_id, id).await?))
}

pub async fn create_node(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Json(body): Json<CreateTraceNode>,
) -> Result<Json<TraceNode>, ServiceError> {
    Ok(Json(
        svc(&state)
            .create_node(
                scope.project_id,
                body.kind,
                body.external_source,
                body.external_ref,
                body.module,
                body.title,
                body.description,
            )
            .await?,
    ))
}

pub async fn update_node(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTraceNode>,
) -> Result<Json<TraceNode>, ServiceError> {
    Ok(Json(
        svc(&state)
            .update_node(
                scope.project_id,
                id,
                body.kind,
                body.title,
                body.description,
                body.module,
                body.attributes,
            )
            .await?,
    ))
}

pub async fn delete_node(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    svc(&state).delete_node(scope.project_id, id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn list_links(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
) -> Result<Json<Vec<crate::domain::trace::TraceLink>>, ServiceError> {
    Ok(Json(svc(&state).list_links(scope.project_id).await?))
}

pub async fn create_link(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Json(body): Json<CreateTraceLink>,
) -> Result<Json<crate::domain::trace::TraceLink>, ServiceError> {
    Ok(Json(
        svc(&state)
            .create_link(scope.project_id, body.source_node_id, body.target_node_id, body.link_type)
            .await?,
    ))
}

pub async fn delete_link(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    svc(&state).delete_link(scope.project_id, id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// 测试项编写：创建一个测试项并 derives_from 到 ≥1 个需求（body 带 requirement_ids）。
pub async fn create_test_item(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Json(body): Json<CreateTestItem>,
) -> Result<Json<TraceNode>, ServiceError> {
    Ok(Json(
        svc(&state)
            .create_test_item(
                scope.project_id,
                body.title,
                body.module,
                body.description,
                body.requirement_ids,
            )
            .await?,
    ))
}

/// 「整理」按钮：按需求锚定规则重排某项目全图并落库。
pub async fn arrange(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
) -> Result<Json<Vec<crate::domain::trace::TraceNode>>, ServiceError> {
    Ok(Json(svc(&state).arrange(scope.project_id).await?))
}

/// 用户手动排序：逐条覆写 `(id, sort_key)`。
pub async fn apply_order(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Json(body): Json<ReorderRequest>,
) -> Result<Json<Vec<crate::domain::trace::TraceNode>>, ServiceError> {
    let orders = body.orders.into_iter().map(|o| (o.id, o.sort_key)).collect();
    Ok(Json(svc(&state).apply_order(scope.project_id, orders).await?))
}

pub async fn ingest(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Json(body): Json<IngestRequest>,
) -> Result<Json<crate::application::trace::service::IngestReport>, ServiceError> {
    Ok(Json(svc(&state).ingest(scope.project_id, body.nodes, body.links).await?))
}

/// 结构化需求 JSON 上传（研发侧导出 → 需求节点）。`source` 在最外层声明一次，
/// 条目不重复带 external_source。
pub async fn upload_requirements(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Json(body): Json<UploadRequirementsRequest>,
) -> Result<Json<crate::application::trace::service::UploadReport>, ServiceError> {
    Ok(Json(
        svc(&state)
            .upload_requirements(scope.project_id, body.source, body.requirements)
            .await?,
    ))
}

fn parse_link_types(raw: Option<String>) -> Result<Option<Vec<crate::domain::trace::LinkType>>, ServiceError> {
    let Some(s) = raw else { return Ok(None); };
    let mut out = Vec::new();
    for part in s.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match crate::domain::trace::LinkType::parse(part) {
            Some(lt) => out.push(lt),
            None => {
                return Err(ServiceError::Validation(format!("unknown link_type: {part}")));
            }
        }
    }
    Ok(Some(out))
}

pub async fn reachable(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<ReachParams>,
) -> Result<Json<Vec<TraceNode>>, ServiceError> {
    let direction = match params.direction.as_deref() {
        Some("up") => Direction::Up,
        _ => Direction::Down,
    };
    let link_types = parse_link_types(params.link_types)?;
    Ok(Json(
        svc(&state)
            .reachable(params.project_id, vec![id], direction, link_types)
            .await?,
    ))
}

pub async fn coverage(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
) -> Result<Json<Vec<TraceNode>>, ServiceError> {
    Ok(Json(svc(&state).coverage(scope.project_id).await?))
}

pub async fn matrix(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
) -> Result<Json<crate::application::trace::service::TraceMatrix>, ServiceError> {
    Ok(Json(svc(&state).matrix(scope.project_id).await?))
}

pub async fn list_baselines(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
) -> Result<Json<Vec<crate::domain::trace::TraceBaseline>>, ServiceError> {
    Ok(Json(svc(&state).list_baselines(scope.project_id).await?))
}

pub async fn create_baseline(
    State(state): State<AppState>,
    Query(scope): Query<ProjectScope>,
    Json(body): Json<CreateBaseline>,
) -> Result<Json<crate::domain::trace::TraceBaseline>, ServiceError> {
    Ok(Json(
        svc(&state)
            .create_baseline(scope.project_id, body.name, body.reason)
            .await?,
    ))
}
