//! PostgreSQL 执行仓储（实例 / 清单项）。

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::RepositoryError;
use crate::domain::execution::{
    ExecutionInstance, ExecutionItem, ExecutionRepository, InstanceStatus, ItemStatus,
};
use crate::infrastructure::database::postgres::db_err;

pub struct PostgresExecutionRepository {
    pool: PgPool,
}

impl PostgresExecutionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const INST_COLS: &'static str =
        "id, project_id, template_id, stage, status, created_at, updated_at";
    const ITEM_COLS: &'static str =
        "id, instance_id, template_node_id, title, status, content, sort_key, created_at, updated_at";

    fn row_to_instance(row: &PgRow) -> Result<ExecutionInstance, RepositoryError> {
        let status_raw: String = row.try_get("status").map_err(db_err)?;
        let status = InstanceStatus::parse(&status_raw)
            .ok_or_else(|| RepositoryError::Database(format!("unknown instance status: {status_raw}")))?;
        Ok(ExecutionInstance {
            id: row.try_get("id").map_err(db_err)?,
            project_id: row.try_get("project_id").map_err(db_err)?,
            template_id: row.try_get("template_id").map_err(db_err)?,
            stage: row.try_get("stage").map_err(db_err)?,
            status,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }

    fn row_to_item(row: &PgRow) -> Result<ExecutionItem, RepositoryError> {
        let status_raw: String = row.try_get("status").map_err(db_err)?;
        let status = ItemStatus::parse(&status_raw)
            .ok_or_else(|| RepositoryError::Database(format!("unknown item status: {status_raw}")))?;
        Ok(ExecutionItem {
            id: row.try_get("id").map_err(db_err)?,
            instance_id: row.try_get("instance_id").map_err(db_err)?,
            template_node_id: row.try_get("template_node_id").map_err(db_err)?,
            title: row.try_get("title").map_err(db_err)?,
            status,
            content: row.try_get("content").map_err(db_err)?,
            sort_key: row.try_get("sort_key").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }
}

#[async_trait]
impl ExecutionRepository for PostgresExecutionRepository {
    async fn find_instance_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<ExecutionInstance>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM execution_instances WHERE id = $1",
            Self::INST_COLS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_instance).transpose()
    }

    async fn list_instances(&self, project_id: Uuid) -> Result<Vec<ExecutionInstance>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM execution_instances WHERE project_id = $1 ORDER BY created_at DESC, id",
            Self::INST_COLS
        ))
        .bind(project_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(Self::row_to_instance).collect()
    }

    async fn insert_instance(&self, i: &ExecutionInstance) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO execution_instances (id, project_id, template_id, stage, status, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7)",
        )
        .bind(i.id)
        .bind(i.project_id)
        .bind(i.template_id)
        .bind(&i.stage)
        .bind(i.status.as_str())
        .bind(i.created_at)
        .bind(i.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn update_instance(&self, i: &ExecutionInstance) -> Result<(), RepositoryError> {
        let res = sqlx::query(
            "UPDATE execution_instances SET stage=$2, status=$3, updated_at=$4 WHERE id=$1",
        )
        .bind(i.id)
        .bind(&i.stage)
        .bind(i.status.as_str())
        .bind(i.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn find_item_by_id(&self, id: Uuid) -> Result<Option<ExecutionItem>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM execution_items WHERE id = $1",
            Self::ITEM_COLS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_item).transpose()
    }

    async fn list_items_by_instance(
        &self,
        instance_id: Uuid,
    ) -> Result<Vec<ExecutionItem>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM execution_items WHERE instance_id = $1 ORDER BY sort_key, created_at, id",
            Self::ITEM_COLS
        ))
        .bind(instance_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(Self::row_to_item).collect()
    }

    async fn insert_item(&self, item: &ExecutionItem) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO execution_items \
             (id, instance_id, template_node_id, title, status, content, sort_key, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(item.id)
        .bind(item.instance_id)
        .bind(item.template_node_id)
        .bind(&item.title)
        .bind(item.status.as_str())
        .bind(&item.content)
        .bind(item.sort_key)
        .bind(item.created_at)
        .bind(item.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(())
    }

    async fn update_item(&self, item: &ExecutionItem) -> Result<(), RepositoryError> {
        let res = sqlx::query(
            "UPDATE execution_items SET title=$2, status=$3, content=$4, updated_at=$5 WHERE id=$1",
        )
        .bind(item.id)
        .bind(&item.title)
        .bind(item.status.as_str())
        .bind(&item.content)
        .bind(item.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }
}
