//! PostgreSQL 追踪图仓储（节点 / 链接 / 基线）。

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::RepositoryError;
use crate::domain::trace::{
    BaselineRepository, LinkType, TraceBaseline, TraceLink, TraceNode, TraceNodeKind,
    TraceLinkRepository, TraceNodeRepository,
};
use crate::infrastructure::database::postgres::db_err;

// ---------------------------------------------------------------------------
// TraceNodeRepository
// ---------------------------------------------------------------------------

pub struct PostgresTraceNodeRepository {
    pool: PgPool,
}

impl PostgresTraceNodeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const COLS: &'static str =
        "id, project_id, kind, external_source, external_ref, module, title, description, \
         attributes, sort_key, created_at, updated_at";

    fn row_to_node(row: &PgRow) -> Result<TraceNode, RepositoryError> {
        let kind_raw: String = row.try_get("kind").map_err(db_err)?;
        let kind = TraceNodeKind::parse(&kind_raw)
            .ok_or_else(|| RepositoryError::Database(format!("unknown trace node kind: {kind_raw}")))?;
        Ok(TraceNode {
            id: row.try_get("id").map_err(db_err)?,
            project_id: row.try_get("project_id").map_err(db_err)?,
            kind,
            external_source: row.try_get("external_source").map_err(db_err)?,
            external_ref: row.try_get("external_ref").map_err(db_err)?,
            module: row.try_get("module").map_err(db_err)?,
            title: row.try_get("title").map_err(db_err)?,
            description: row.try_get("description").map_err(db_err)?,
            attributes: row.try_get("attributes").map_err(db_err)?,
            sort_key: row.try_get("sort_key").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }
}

#[async_trait]
impl TraceNodeRepository for PostgresTraceNodeRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TraceNode>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM trace_nodes WHERE id = $1",
            Self::COLS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_node).transpose()
    }

    async fn find_by_external(
        &self,
        project_id: Uuid,
        external_source: &str,
        external_ref: &str,
    ) -> Result<Option<TraceNode>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM trace_nodes \
             WHERE project_id = $1 AND external_source = $2 AND external_ref = $3",
            Self::COLS
        ))
        .bind(project_id)
        .bind(external_source)
        .bind(external_ref)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_node).transpose()
    }

    async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceNode>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM trace_nodes WHERE project_id = $1 \
             ORDER BY created_at, sort_key, id",
            Self::COLS
        ))
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(Self::row_to_node).collect()
    }

    async fn upsert(&self, node: &TraceNode) -> Result<Uuid, RepositoryError> {
        // 注意：ON CONFLICT DO UPDATE **不**更新 sort_key —— 重摄取/幂等更新时保留
        // 用户已经排好的手动顺序；只有全新插入的行才会带上调用方给它的 sort_key。
        // 唯一键为「项目内 external 键」：跨项目可出现相同 external_source+external_ref。
        let row = sqlx::query(
            "INSERT INTO trace_nodes \
             (id, project_id, kind, external_source, external_ref, module, title, description, \
              attributes, sort_key, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) \
             ON CONFLICT (project_id, external_source, external_ref) DO UPDATE SET \
               kind = EXCLUDED.kind, module = EXCLUDED.module, title = EXCLUDED.title, \
               description = EXCLUDED.description, attributes = EXCLUDED.attributes, \
               updated_at = EXCLUDED.updated_at \
             RETURNING id",
        )
        .bind(node.id)
        .bind(node.project_id)
        .bind(node.kind.as_str())
        .bind(&node.external_source)
        .bind(&node.external_ref)
        .bind(&node.module)
        .bind(&node.title)
        .bind(&node.description)
        .bind(&node.attributes)
        .bind(node.sort_key)
        .bind(node.created_at)
        .bind(node.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        row.try_get("id").map_err(db_err)
    }

    async fn set_sort_keys(&self, orders: &[(Uuid, i32)]) -> Result<(), RepositoryError> {
        for (id, key) in orders {
            sqlx::query("UPDATE trace_nodes SET sort_key = $1 WHERE id = $2")
                .bind(key)
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(db_err)?;
        }
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        let res = sqlx::query("DELETE FROM trace_nodes WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// TraceLinkRepository
// ---------------------------------------------------------------------------

pub struct PostgresTraceLinkRepository {
    pool: PgPool,
}

impl PostgresTraceLinkRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const COLS: &'static str =
        "id, project_id, source_node_id, target_node_id, link_type, attributes, created_at, updated_at";

    fn row_to_link(row: &PgRow) -> Result<TraceLink, RepositoryError> {
        let lt_raw: String = row.try_get("link_type").map_err(db_err)?;
        let link_type = LinkType::parse(&lt_raw)
            .ok_or_else(|| RepositoryError::Database(format!("unknown link_type: {lt_raw}")))?;
        Ok(TraceLink {
            id: row.try_get("id").map_err(db_err)?,
            project_id: row.try_get("project_id").map_err(db_err)?,
            source_node_id: row.try_get("source_node_id").map_err(db_err)?,
            target_node_id: row.try_get("target_node_id").map_err(db_err)?,
            link_type,
            attributes: row.try_get("attributes").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }
}

#[async_trait]
impl TraceLinkRepository for PostgresTraceLinkRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TraceLink>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM trace_links WHERE id = $1",
            Self::COLS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_link).transpose()
    }

    async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceLink>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM trace_links WHERE project_id = $1 ORDER BY created_at, id",
            Self::COLS
        ))
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(Self::row_to_link).collect()
    }

    async fn upsert(&self, link: &TraceLink) -> Result<Uuid, RepositoryError> {
        let row = sqlx::query(
            "INSERT INTO trace_links \
             (id, project_id, source_node_id, target_node_id, link_type, attributes, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8) \
             ON CONFLICT (source_node_id, target_node_id, link_type) DO UPDATE SET \
               attributes = EXCLUDED.attributes, updated_at = EXCLUDED.updated_at \
             RETURNING id",
        )
        .bind(link.id)
        .bind(link.project_id)
        .bind(link.source_node_id)
        .bind(link.target_node_id)
        .bind(link.link_type.as_str())
        .bind(&link.attributes)
        .bind(link.created_at)
        .bind(link.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        row.try_get("id").map_err(db_err)
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        let res = sqlx::query("DELETE FROM trace_links WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BaselineRepository
// ---------------------------------------------------------------------------

pub struct PostgresBaselineRepository {
    pool: PgPool,
}

impl PostgresBaselineRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BaselineRepository for PostgresBaselineRepository {
    async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceBaseline>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, project_id, name, reason, snapshot_json, created_at \
             FROM trace_baselines WHERE project_id = $1 ORDER BY created_at DESC, id",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter()
            .map(|row| {
                Ok(TraceBaseline {
                    id: row.try_get("id").map_err(db_err)?,
                    project_id: row.try_get("project_id").map_err(db_err)?,
                    name: row.try_get("name").map_err(db_err)?,
                    reason: row.try_get("reason").map_err(db_err)?,
                    snapshot_json: row.try_get("snapshot_json").map_err(db_err)?,
                    created_at: row.try_get("created_at").map_err(db_err)?,
                })
            })
            .collect()
    }

    async fn insert(&self, baseline: &TraceBaseline) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO trace_baselines (id, project_id, name, reason, snapshot_json, created_at) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(baseline.id)
        .bind(baseline.project_id)
        .bind(&baseline.name)
        .bind(&baseline.reason)
        .bind(&baseline.snapshot_json)
        .bind(baseline.created_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }
}
