//! 文档模板 HTTP handlers。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::Json;
use uuid::Uuid;

use crate::api::http::dto::{
    CreateTemplate, CreateTemplateEdge, CreateTemplateNode, ReorderRequest, UpdateTemplate,
    UpdateTemplateNode,
};
use crate::api::http::AppState;
use crate::application::document::DocumentService;
use crate::application::ServiceError;
use crate::domain::document::TemplateNode;

fn svc(state: &AppState) -> Arc<DocumentService> {
    state.documents.clone()
}

pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::domain::document::DocumentTemplate>>, ServiceError> {
    Ok(Json(svc(&state).list_templates().await?))
}

pub async fn create_template(
    State(state): State<AppState>,
    Json(body): Json<CreateTemplate>,
) -> Result<Json<crate::domain::document::DocumentTemplate>, ServiceError> {
    Ok(Json(
        svc(&state)
            .create_template(body.name, body.kind, body.description)
            .await?,
    ))
}

pub async fn get_template_full(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    let (tpl, nodes, edges) = svc(&state).get_template_full(id).await?;
    Ok(Json(serde_json::json!({
        "template": tpl,
        "nodes": nodes,
        "edges": edges,
    })))
}

pub async fn update_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateTemplate>,
) -> Result<Json<crate::domain::document::DocumentTemplate>, ServiceError> {
    Ok(Json(
        svc(&state)
            .update_template(id, body.name, body.kind, body.description)
            .await?,
    ))
}

pub async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    svc(&state).delete_template(id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn list_nodes(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TemplateNode>>, ServiceError> {
    Ok(Json(svc(&state).list_nodes(id).await?))
}

pub async fn create_node(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateTemplateNode>,
) -> Result<Json<TemplateNode>, ServiceError> {
    Ok(Json(
        svc(&state)
            .create_node(id, body.node_type, body.title, body.content_spec, body.sort_key)
            .await?,
    ))
}

pub async fn update_node(
    State(state): State<AppState>,
    // 路由是 /templates/{id}/nodes/{nid}：模板 id 用不到，但路径上有两个参数，
    // 必须成对提取（单 Path<Uuid> 会报「Expected 1 but got 2」）。
    Path((_template_id, node_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateTemplateNode>,
) -> Result<Json<TemplateNode>, ServiceError> {
    Ok(Json(
        svc(&state)
            .update_node(
                node_id,
                body.node_type,
                body.title,
                body.content_spec,
                body.sort_key,
                body.tex_component,
            )
            .await?,
    ))
}

pub async fn delete_node(
    State(state): State<AppState>,
    Path((_template_id, node_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    svc(&state).delete_node(node_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

/// 批量重排（导航窗格拖拽 / 画布同组纵向拖拽共用）：逐条覆写 `(id, sort_key)`。
pub async fn reorder_nodes(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ReorderRequest>,
) -> Result<Json<Vec<TemplateNode>>, ServiceError> {
    let orders = body.orders.into_iter().map(|o| (o.id, o.sort_key)).collect();
    Ok(Json(svc(&state).reorder_nodes(id, orders).await?))
}

pub async fn list_edges(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<crate::domain::document::TemplateEdge>>, ServiceError> {
    let (_, _, edges) = svc(&state).get_template_full(id).await?;
    Ok(Json(edges))
}

pub async fn create_edge(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateTemplateEdge>,
) -> Result<Json<crate::domain::document::TemplateEdge>, ServiceError> {
    Ok(Json(
        svc(&state)
            .add_edge(
                id,
                body.source_node_id,
                body.target_node_id,
                body.edge_type.unwrap_or_else(|| "contains".into()),
            )
            .await?,
    ))
}

pub async fn delete_edge(
    State(state): State<AppState>,
    Path((_template_id, edge_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ServiceError> {
    svc(&state).remove_edge(edge_id).await?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}
