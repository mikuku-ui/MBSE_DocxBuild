//! PostgreSQL 项目数据仓储：结构 × 值合并成视图（get_view）+ 整存替换（replace）。
//!
//! get_view 在单次调用内取齐：体系字段（左连项目值）→ 表 → 列 → 项目行，
//! 组装成自描述的 [`ProjectDataView`]。replace 单事务：先清空该项目全部值行，
//! 再按 write 重插（field 值、表行），行 project_id 一律以入参为准。

use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::RepositoryError;
use crate::domain::projectdata::{
    DataColumn, DataField, DataRow, DataTable, ProjectDataRepository, ProjectDataView,
    ProjectDataWrite,
};
use crate::infrastructure::database::postgres::db_err;

pub struct PostgresProjectDataRepository {
    pool: PgPool,
}

impl PostgresProjectDataRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn row_to_field(row: &PgRow) -> Result<DataField, RepositoryError> {
        let value: Option<String> = row.try_get("value").map_err(db_err)?;
        Ok(DataField {
            id: row.try_get("id").map_err(db_err)?,
            doc_kind: row.try_get("doc_kind").map_err(db_err)?,
            field_key: row.try_get("field_key").map_err(db_err)?,
            label: row.try_get("label").map_err(db_err)?,
            value: value.unwrap_or_default(),
        })
    }
}

#[async_trait]
impl ProjectDataRepository for PostgresProjectDataRepository {
    async fn get_view(
        &self,
        project_id: Uuid,
        scheme_id: Uuid,
    ) -> Result<ProjectDataView, RepositoryError> {
        let fields = sqlx::query(
            "SELECT sf.id, sf.doc_kind, sf.field_key, sf.label, pf.value \
             FROM scheme_fields sf \
             LEFT JOIN project_field_values pf \
               ON pf.field_id = sf.id AND pf.project_id = $1 \
             WHERE sf.scheme_id = $2 \
             ORDER BY sf.sort_key, sf.created_at, sf.id",
        )
        .bind(project_id)
        .bind(scheme_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(Self::row_to_field)
        .collect::<Result<_, _>>()?;

        let tables = sqlx::query(
            "SELECT id, scheme_id, table_key, label, sort_key \
             FROM scheme_tables WHERE scheme_id = $1 \
             ORDER BY sort_key, created_at, id",
        )
        .bind(scheme_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            Ok(DataTable {
                id: row.try_get("id").map_err(db_err)?,
                table_key: row.try_get("table_key").map_err(db_err)?,
                label: row.try_get("label").map_err(db_err)?,
                columns: Vec::new(),
                rows: Vec::new(),
            })
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;

        // 列：SELECT 需带出 table_id 用于归组，但 DataColumn 自身不含它 → 以 (table_id, column) 元组收集。
        let cols: Vec<(Uuid, DataColumn)> = sqlx::query(
            "SELECT c.id, c.table_id, c.column_key, c.label \
             FROM scheme_table_columns c \
             JOIN scheme_tables t ON t.id = c.table_id \
             WHERE t.scheme_id = $1 \
             ORDER BY t.sort_key, c.sort_key, c.created_at, c.id",
        )
        .bind(scheme_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(|row| {
            let table_id: Uuid = row.try_get("table_id").map_err(db_err)?;
            let col = DataColumn {
                id: row.try_get("id").map_err(db_err)?,
                column_key: row.try_get("column_key").map_err(db_err)?,
                label: row.try_get("label").map_err(db_err)?,
            };
            Ok((table_id, col))
        })
        .collect::<Result<Vec<_>, RepositoryError>>()?;

        let mut defs: Vec<DataTable> = tables;
        for (table_id, c) in cols {
            if let Some(t) = defs.iter_mut().find(|t| t.id == table_id) {
                t.columns.push(c);
            }
        }

        // 行：cells jsonb（以 column_key 为键）→ HashMap。
        let table_ids: Vec<Uuid> = defs.iter().map(|t| t.id).collect();
        if !table_ids.is_empty() {
            let rows = sqlx::query(
                "SELECT id, table_id, row_index, cells \
                 FROM project_table_rows WHERE project_id = $1 AND table_id = ANY($2::uuid[]) \
                 ORDER BY table_id, row_index",
            )
            .bind(project_id)
            .bind(&table_ids)
            .fetch_all(&self.pool)
            .await
            .map_err(db_err)?;

            for r in rows {
                let table_id: Uuid = r.try_get("table_id").map_err(db_err)?;
                let row_index: i32 = r.try_get("row_index").map_err(db_err)?;
                let cells: serde_json::Value = r.try_get("cells").map_err(db_err)?;
                let cells: HashMap<String, String> = serde_json::from_value(cells).map_err(|e| {
                    RepositoryError::Database(format!("bad cells jsonb: {e}"))
                })?;
                if let Some(t) = defs.iter_mut().find(|t| t.id == table_id) {
                    t.rows.push(DataRow { row_index, cells });
                }
            }
        }
        for t in defs.iter_mut() {
            t.rows.sort_by_key(|r| r.row_index);
        }

        Ok(ProjectDataView {
            project_id,
            scheme_id: Some(scheme_id),
            fields,
            tables: defs,
        })
    }

    async fn replace(
        &self,
        project_id: Uuid,
        write: &ProjectDataWrite,
    ) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        // 项目不存在由 FK 兜底翻译；先删后插保证整存语义。
        sqlx::query("DELETE FROM project_field_values WHERE project_id = $1")
            .bind(project_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        sqlx::query("DELETE FROM project_table_rows WHERE project_id = $1")
            .bind(project_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

        for f in &write.fields {
            sqlx::query(
                "INSERT INTO project_field_values (id, project_id, field_id, value, created_at, updated_at) \
                 VALUES ($1,$2,$3,$4,now(),now())",
            )
            .bind(Uuid::new_v4())
            .bind(project_id)
            .bind(f.field_id)
            .bind(&f.value)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
        }
        for t in &write.tables {
            for r in &t.rows {
                let cells = serde_json::to_value(&r.cells).map_err(|e| {
                    RepositoryError::Database(format!("cells not serializable: {e}"))
                })?;
                sqlx::query(
                    "INSERT INTO project_table_rows \
                     (id, project_id, table_id, row_index, cells, created_at, updated_at) \
                     VALUES ($1,$2,$3,$4,$5,now(),now())",
                )
                .bind(Uuid::new_v4())
                .bind(project_id)
                .bind(t.table_id)
                .bind(r.row_index)
                .bind(cells)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
        }

        tx.commit().await.map_err(db_err)?;
        Ok(())
    }
}
