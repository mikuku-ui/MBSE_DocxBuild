//! PostgreSQL 文档模板仓储（模板 / 节点 / 边）。

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::RepositoryError;
use crate::domain::document::{
    DocumentRepository, DocumentTemplate, TemplateEdge, TemplateNode, TemplateNodeType,
};
use crate::infrastructure::database::postgres::db_err;

pub struct PostgresDocumentRepository {
    pool: PgPool,
}

impl PostgresDocumentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const TPL_COLS: &'static str =
        "id, name, kind, description, created_at, updated_at";
    const NODE_COLS: &'static str =
        "id, template_id, node_type, title, content_spec, tex_component, sort_key, created_at, updated_at";

    fn row_to_template(row: &PgRow) -> Result<DocumentTemplate, RepositoryError> {
        Ok(DocumentTemplate {
            id: row.try_get("id").map_err(db_err)?,
            name: row.try_get("name").map_err(db_err)?,
            kind: row.try_get("kind").map_err(db_err)?,
            description: row.try_get("description").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }

    fn row_to_node(row: &PgRow) -> Result<TemplateNode, RepositoryError> {
        let nt_raw: String = row.try_get("node_type").map_err(db_err)?;
        let node_type = TemplateNodeType::parse(&nt_raw).ok_or_else(|| {
            RepositoryError::Database(format!("unknown template node_type: {nt_raw}"))
        })?;
        Ok(TemplateNode {
            id: row.try_get("id").map_err(db_err)?,
            template_id: row.try_get("template_id").map_err(db_err)?,
            node_type,
            title: row.try_get("title").map_err(db_err)?,
            content_spec: row.try_get("content_spec").map_err(db_err)?,
            tex_component: row.try_get("tex_component").map_err(db_err)?,
            sort_key: row.try_get("sort_key").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }

    fn row_to_edge(row: &PgRow) -> Result<TemplateEdge, RepositoryError> {
        Ok(TemplateEdge {
            id: row.try_get("id").map_err(db_err)?,
            template_id: row.try_get("template_id").map_err(db_err)?,
            source_node_id: row.try_get("source_node_id").map_err(db_err)?,
            target_node_id: row.try_get("target_node_id").map_err(db_err)?,
            edge_type: row.try_get("edge_type").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
        })
    }
}

#[async_trait]
impl DocumentRepository for PostgresDocumentRepository {
    // —— 模板 ——
    async fn find_template_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<DocumentTemplate>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM document_templates WHERE id = $1",
            Self::TPL_COLS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_template).transpose()
    }

    async fn list_templates(&self) -> Result<Vec<DocumentTemplate>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM document_templates ORDER BY created_at, id",
            Self::TPL_COLS
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(Self::row_to_template).collect()
    }

    async fn insert_template(&self, tpl: &DocumentTemplate) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO document_templates (id, name, kind, description, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6)",
        )
        .bind(tpl.id)
        .bind(&tpl.name)
        .bind(&tpl.kind)
        .bind(&tpl.description)
        .bind(tpl.created_at)
        .bind(tpl.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn update_template(&self, tpl: &DocumentTemplate) -> Result<(), RepositoryError> {
        let res = sqlx::query(
            "UPDATE document_templates SET name=$2, kind=$3, description=$4, updated_at=$5 \
             WHERE id=$1",
        )
        .bind(tpl.id)
        .bind(&tpl.name)
        .bind(&tpl.kind)
        .bind(&tpl.description)
        .bind(tpl.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn delete_template(&self, id: Uuid) -> Result<(), RepositoryError> {
        let res = sqlx::query("DELETE FROM document_templates WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    // —— 节点 ——
    async fn find_node_by_id(&self, id: Uuid) -> Result<Option<TemplateNode>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM document_template_nodes WHERE id = $1",
            Self::NODE_COLS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_node).transpose()
    }

    async fn list_nodes_by_template(
        &self,
        template_id: Uuid,
    ) -> Result<Vec<TemplateNode>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM document_template_nodes WHERE template_id = $1 \
             ORDER BY sort_key, created_at, id",
            Self::NODE_COLS
        ))
        .bind(template_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(Self::row_to_node).collect()
    }

    async fn insert_node(&self, node: &TemplateNode) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO document_template_nodes \
             (id, template_id, node_type, title, content_spec, tex_component, sort_key, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(node.id)
        .bind(node.template_id)
        .bind(node.node_type.as_str())
        .bind(&node.title)
        .bind(&node.content_spec)
        .bind(&node.tex_component)
        .bind(node.sort_key)
        .bind(node.created_at)
        .bind(node.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn update_node(&self, node: &TemplateNode) -> Result<(), RepositoryError> {
        let res = sqlx::query(
            "UPDATE document_template_nodes \
             SET node_type=$2, title=$3, content_spec=$4, tex_component=$5, sort_key=$6, updated_at=$7 \
             WHERE id=$1",
        )
        .bind(node.id)
        .bind(node.node_type.as_str())
        .bind(&node.title)
        .bind(&node.content_spec)
        .bind(&node.tex_component)
        .bind(node.sort_key)
        .bind(node.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn delete_node(&self, id: Uuid) -> Result<(), RepositoryError> {
        let res = sqlx::query("DELETE FROM document_template_nodes WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn set_sort_keys(&self, orders: &[(Uuid, i32)]) -> Result<(), RepositoryError> {
        if orders.is_empty() {
            return Ok(());
        }
        let ids: Vec<Uuid> = orders.iter().map(|(id, _)| *id).collect();
        let keys: Vec<i32> = orders.iter().map(|(_, key)| *key).collect();
        sqlx::query(
            "UPDATE document_template_nodes AS n \
             SET sort_key = v.key, updated_at = now() \
             FROM unnest($1::uuid[], $2::int[]) AS v(id, key) \
             WHERE n.id = v.id",
        )
        .bind(ids)
        .bind(keys)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    // —— 边 ——
    async fn find_edge_by_id(&self, id: Uuid) -> Result<Option<TemplateEdge>, RepositoryError> {
        let row = sqlx::query(
            "SELECT id, template_id, source_node_id, target_node_id, edge_type, created_at \
             FROM document_template_edges WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_edge).transpose()
    }

    async fn list_edges_by_template(
        &self,
        template_id: Uuid,
    ) -> Result<Vec<TemplateEdge>, RepositoryError> {
        let rows = sqlx::query(
            "SELECT id, template_id, source_node_id, target_node_id, edge_type, created_at \
             FROM document_template_edges WHERE template_id = $1 ORDER BY created_at, id",
        )
        .bind(template_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(Self::row_to_edge).collect()
    }

    async fn insert_edge(&self, edge: &TemplateEdge) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO document_template_edges \
             (id, template_id, source_node_id, target_node_id, edge_type, created_at) \
             VALUES ($1,$2,$3,$4,$5,$6) \
             ON CONFLICT (template_id, source_node_id, target_node_id, edge_type) DO NOTHING",
        )
        .bind(edge.id)
        .bind(edge.template_id)
        .bind(edge.source_node_id)
        .bind(edge.target_node_id)
        .bind(&edge.edge_type)
        .bind(edge.created_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn delete_edge(&self, id: Uuid) -> Result<(), RepositoryError> {
        let res = sqlx::query("DELETE FROM document_template_edges WHERE id = $1")
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
