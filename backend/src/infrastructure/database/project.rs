//! PostgreSQL 项目仓储。

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::project::{Project, ProjectRepository};
use crate::domain::RepositoryError;
use crate::infrastructure::database::postgres::db_err;

pub struct PostgresProjectRepository {
    pool: PgPool,
}

impl PostgresProjectRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const COLS: &'static str = "id, name, scheme_id, created_at, updated_at";

    fn row_to_project(row: &PgRow) -> Result<Project, RepositoryError> {
        Ok(Project {
            id: row.try_get("id").map_err(db_err)?,
            name: row.try_get("name").map_err(db_err)?,
            scheme_id: row.try_get("scheme_id").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }

    /// 把唯一约束冲突（Postgres SQLSTATE 23505）翻译成 `RepositoryError::Conflict`。
    fn map_err(e: sqlx::Error) -> RepositoryError {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                return RepositoryError::Conflict;
            }
        }
        db_err(e)
    }
}

#[async_trait]
impl ProjectRepository for PostgresProjectRepository {
    async fn list_all(&self) -> Result<Vec<Project>, RepositoryError> {
        let rows = sqlx::query(&format!(
            "SELECT {} FROM projects ORDER BY created_at, id",
            Self::COLS
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;
        rows.iter().map(Self::row_to_project).collect()
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Project>, RepositoryError> {
        let row = sqlx::query(&format!(
            "SELECT {} FROM projects WHERE id = $1",
            Self::COLS
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;
        row.as_ref().map(Self::row_to_project).transpose()
    }

    async fn insert(&self, project: &Project) -> Result<(), RepositoryError> {
        // 项目名唯一由 `name UNIQUE` 保证；撞名走 map_err → Conflict。
        sqlx::query(
            "INSERT INTO projects (id, name, scheme_id, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(project.id)
        .bind(&project.name)
        .bind(project.scheme_id)
        .bind(project.created_at)
        .bind(project.updated_at)
        .execute(&self.pool)
        .await
        .map_err(Self::map_err)?;
        Ok(())
    }

    async fn rename(&self, id: Uuid, name: &str) -> Result<(), RepositoryError> {
        let res = sqlx::query("UPDATE projects SET name = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(Self::map_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }

    async fn set_scheme(&self, id: Uuid, scheme_id: Option<Uuid>) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        let old: Option<Option<Uuid>> =
            sqlx::query_scalar("SELECT scheme_id FROM projects WHERE id = $1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
        let Some(old_scheme) = old else {
            return Err(RepositoryError::NotFound);
        };
        if old_scheme == scheme_id {
            // 绑同一体系 = 幂等空操作：不清项目数据。
            tx.commit().await.map_err(db_err)?;
            return Ok(());
        }
        // 换体系：旧体系字段/表 id 下的已填值对新体系无意义 → 先清空。
        sqlx::query("DELETE FROM project_field_values WHERE project_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        sqlx::query("DELETE FROM project_table_rows WHERE project_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        sqlx::query("UPDATE projects SET scheme_id = $2, updated_at = now() WHERE id = $1")
            .bind(id)
            .bind(scheme_id)
            .execute(&mut *tx)
            .await
            .map_err(Self::map_err)?;
        tx.commit().await.map_err(db_err)?;
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        let res = sqlx::query("DELETE FROM projects WHERE id = $1")
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
