//! 路由 + 应用状态。状态只持服务（Arc），不持连接池——HTTP 层不碰数据访问。

use std::sync::Arc;

use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use tower_http::cors::CorsLayer;

use crate::application::assembly::AssemblyService;
use crate::application::document::DocumentService;
use crate::application::execution::ExecutionService;
use crate::application::project::ProjectService;
use crate::application::projectdata::ProjectDataService;
use crate::application::scheme::SchemeService;
use crate::application::trace::TraceService;

use super::assembly_handler as assemble;
use super::document_handler as doc;
use super::execution_handler as exec;
use super::project_handler as proj;
use super::projectdata_handler as data;
use super::scheme_handler as scheme;
use super::trace_handler as trace;

/// 应用状态：服务的组合根（composition root 注入）。
#[derive(Clone)]
pub struct AppState {
    pub trace: Arc<TraceService>,
    pub documents: Arc<DocumentService>,
    pub executions: Arc<ExecutionService>,
    pub projects: Arc<ProjectService>,
    pub schemes: Arc<SchemeService>,
    pub project_data: Arc<ProjectDataService>,
    pub assemblies: Arc<AssemblyService>,
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "mbse-testlib-backend" }))
}

/// 组装路由：全部 /api 前缀。CORS 全放开（本地开发，前端 Vite 代理同源走 /api）。
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        // ---- projects ----
        .route(
            "/api/projects",
            get(proj::list_projects).post(proj::create_project),
        )
        .route(
            "/api/projects/{id}",
            put(proj::rename_project).delete(proj::delete_project),
        )
        .route("/api/projects/{id}/scheme", put(proj::bind_scheme))
        // ---- schemes（台账编排 = 体系设计）----
        .route(
            "/api/schemes",
            get(scheme::list_schemes).post(scheme::create_scheme),
        )
        .route(
            "/api/schemes/{id}",
            get(scheme::get_scheme)
                .put(scheme::replace_scheme)
                .delete(scheme::delete_scheme),
        )
        // ---- project data（项目级一份的值；执行清单/项目页共用填数面）----
        .route(
            "/api/projects/{id}/data",
            get(data::get_project_data).put(data::replace_project_data),
        )
        // ---- trace ----
        .route("/api/trace/nodes", get(trace::list_nodes).post(trace::create_node))
        .route(
            "/api/trace/nodes/{id}",
            get(trace::get_node)
                .put(trace::update_node)
                .delete(trace::delete_node),
        )
        .route(
            "/api/trace/nodes/{id}/reachable",
            get(trace::reachable),
        )
        .route(
            "/api/trace/links",
            get(trace::list_links).post(trace::create_link),
        )
        .route("/api/trace/links/{id}", delete(trace::delete_link))
        .route("/api/trace/test-items", post(trace::create_test_item))
        .route("/api/trace/ingest", post(trace::ingest))
        .route(
            "/api/trace/requirements/upload",
            post(trace::upload_requirements),
        )
        .route("/api/trace/nodes/arrange", post(trace::arrange))
        .route("/api/trace/nodes/order", put(trace::apply_order))
        .route("/api/trace/coverage", get(trace::coverage))
        .route("/api/trace/matrix", get(trace::matrix))
        .route("/api/trace/baselines", get(trace::list_baselines).post(trace::create_baseline))
        // ---- documents ----
        .route(
            "/api/documents/templates",
            get(doc::list_templates).post(doc::create_template),
        )
        .route(
            "/api/documents/templates/{id}",
            get(doc::get_template_full)
                .put(doc::update_template)
                .delete(doc::delete_template),
        )
        .route(
            "/api/documents/templates/{id}/nodes",
            get(doc::list_nodes).post(doc::create_node),
        )
        .route(
            "/api/documents/templates/{id}/nodes/{nid}",
            put(doc::update_node).delete(doc::delete_node),
        )
        .route(
            "/api/documents/templates/{id}/nodes/order",
            put(doc::reorder_nodes),
        )
        .route(
            "/api/documents/templates/{id}/edges",
            get(doc::list_edges).post(doc::create_edge),
        )
        .route(
            "/api/documents/templates/{id}/edges/{eid}",
            delete(doc::delete_edge),
        )
        // ---- executions ----
        .route(
            "/api/executions/instances",
            get(exec::list_instances).post(exec::create_instance),
        )
        .route(
            "/api/executions/instances/{id}",
            get(exec::get_instance_detail).put(exec::update_instance),
        )
        .route(
            "/api/executions/instances/{id}/items",
            get(exec::list_items),
        )
        .route(
            "/api/executions/items/{id}",
            put(exec::update_item),
        )
        // ---- assembly（文档装配预览：模板 × 项目数据 → 正文）----
        .route(
            "/api/documents/templates/{id}/assemble",
            get(assemble::assemble),
        )
        .layer(CorsLayer::permissive())
        .with_state(state)
}
