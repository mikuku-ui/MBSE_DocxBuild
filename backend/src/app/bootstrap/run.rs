//! Composition root：加载配置 → 连库（跑迁移）→ 组装 repo/service → 起 HTTP。

use std::sync::Arc;

use anyhow::Context;
use axum::Router;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tracing::info;

use crate::api::http::{router, AppState};
use crate::application::assembly::AssemblyService;
use crate::application::document::DocumentService;
use crate::application::execution::ExecutionService;
use crate::application::project::ProjectService;
use crate::application::projectdata::ProjectDataService;
use crate::application::scheme::SchemeService;
use crate::application::trace::TraceService;
use crate::app::config::Settings;
use crate::domain::document::DocumentRepository;
use crate::domain::execution::ExecutionRepository;
use crate::domain::project::ProjectRepository;
use crate::domain::projectdata::ProjectDataRepository;
use crate::domain::scheme::SchemeRepository;
use crate::domain::trace::{BaselineRepository, TraceLinkRepository, TraceNodeRepository};
use crate::infrastructure::database::{
    connect, PostgresBaselineRepository, PostgresDocumentRepository, PostgresExecutionRepository,
    PostgresProjectDataRepository, PostgresProjectRepository, PostgresSchemeRepository,
    PostgresTraceLinkRepository, PostgresTraceNodeRepository,
};

/// 装配一组后端服务所需的 Postgres 仓储（同一连接池）。
///
/// 注意：document 表域被文档服务与执行服务**共享**（执行清单由模板派生），
/// 必须传入同一个 `Arc<dyn DocumentRepository>`——克隆 Arc 指针即可。
/// 装配服务进一步把文档 + 追踪 + 执行 + 体系 + 项目数据串起来（各自拿到 Arc 指针）。
fn build_repos(pool: PgPool) -> AppState {
    let trace_nodes: Arc<dyn TraceNodeRepository> =
        Arc::new(PostgresTraceNodeRepository::new(pool.clone()));
    let trace_links: Arc<dyn TraceLinkRepository> =
        Arc::new(PostgresTraceLinkRepository::new(pool.clone()));
    let baselines: Arc<dyn BaselineRepository> =
        Arc::new(PostgresBaselineRepository::new(pool.clone()));
    let docs: Arc<dyn DocumentRepository> =
        Arc::new(PostgresDocumentRepository::new(pool.clone()));
    let execution: Arc<dyn ExecutionRepository> =
        Arc::new(PostgresExecutionRepository::new(pool.clone()));
    let projects_repo: Arc<dyn ProjectRepository> =
        Arc::new(PostgresProjectRepository::new(pool.clone()));
    let schemes_repo: Arc<dyn SchemeRepository> =
        Arc::new(PostgresSchemeRepository::new(pool.clone()));
    let project_data_repo: Arc<dyn ProjectDataRepository> =
        Arc::new(PostgresProjectDataRepository::new(pool));

    let documents = Arc::new(DocumentService::new(docs.clone()));
    let trace = Arc::new(TraceService::new(trace_nodes, trace_links, baselines));
    let executions = Arc::new(ExecutionService::new(docs, execution));
    let projects = Arc::new(ProjectService::new(projects_repo, schemes_repo.clone()));
    let schemes = Arc::new(SchemeService::new(schemes_repo));
    let project_data = Arc::new(ProjectDataService::new(
        project_data_repo,
        projects.clone(),
        schemes.clone(),
    ));
    let assemblies = Arc::new(AssemblyService::new(
        documents.clone(),
        trace.clone(),
        executions.clone(),
        project_data.clone(),
    ));

    AppState {
        trace,
        documents,
        executions,
        projects,
        schemes,
        project_data,
        assemblies,
    }
}

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let settings = Settings::load()?;
    let database_url = settings.database_url()?;

    let pool = connect(&database_url)
        .await
        .with_context(|| format!("Failed to connect/migrate database {database_url}"))?;
    info!("database ready (migrations applied)");

    let state = build_repos(pool);
    let app: Router = router(state);

    let addr = format!("{}:{}", settings.host, settings.port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("Failed to bind {addr}"))?;
    info!("listening on http://{addr}");

    axum::serve(listener, app).await.context("server error")
}
