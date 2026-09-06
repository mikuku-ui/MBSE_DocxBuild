//! PostgreSQL 体系仓储：清单 / 完整定义 / 新建 / **按自然键 upsert 保 id 的整存
//! 替换** / 删除。整存替换在单事务内：
//!   - scheme 行先改名/描述（不存在 → NotFound）；
//!   - 字段/表/列按自然键 upsert（DO UPDATE 只改 label，**不动 id** → 已填项目值
//!     引用不失效）；只删「新定义里已不存在的自然键」；
//!   - sort_key 一律以定义数组的顺序重编（0..n-1）——可视化列表顺序即真源。
//! 删除被项目绑定的体系 → Conflict（409）。

use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::domain::RepositoryError;
use crate::domain::scheme::{
    Scheme, SchemeField, SchemeFull, SchemeRepository, SchemeSpec, SchemeTable, SchemeTableColumn,
    SchemeTableDef, SchemeTableSpec,
};
use crate::infrastructure::database::postgres::db_err;

pub struct PostgresSchemeRepository {
    pool: PgPool,
}

impl PostgresSchemeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn map_err(e: sqlx::Error) -> RepositoryError {
        if let Some(db) = e.as_database_error() {
            if db.code().as_deref() == Some("23505") {
                return RepositoryError::Conflict;
            }
        }
        db_err(e)
    }

    fn row_to_scheme(row: &PgRow) -> Result<Scheme, RepositoryError> {
        Ok(Scheme {
            id: row.try_get("id").map_err(db_err)?,
            name: row.try_get("name").map_err(db_err)?,
            description: row.try_get("description").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }

    fn row_to_field(row: &PgRow) -> Result<SchemeField, RepositoryError> {
        Ok(SchemeField {
            id: row.try_get("id").map_err(db_err)?,
            scheme_id: row.try_get("scheme_id").map_err(db_err)?,
            doc_kind: row.try_get("doc_kind").map_err(db_err)?,
            field_key: row.try_get("field_key").map_err(db_err)?,
            label: row.try_get("label").map_err(db_err)?,
            sort_key: row.try_get("sort_key").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }

    fn row_to_table(row: &PgRow) -> Result<SchemeTable, RepositoryError> {
        Ok(SchemeTable {
            id: row.try_get("id").map_err(db_err)?,
            scheme_id: row.try_get("scheme_id").map_err(db_err)?,
            table_key: row.try_get("table_key").map_err(db_err)?,
            label: row.try_get("label").map_err(db_err)?,
            sort_key: row.try_get("sort_key").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }

    fn row_to_column(row: &PgRow) -> Result<SchemeTableColumn, RepositoryError> {
        Ok(SchemeTableColumn {
            id: row.try_get("id").map_err(db_err)?,
            table_id: row.try_get("table_id").map_err(db_err)?,
            column_key: row.try_get("column_key").map_err(db_err)?,
            label: row.try_get("label").map_err(db_err)?,
            sort_key: row.try_get("sort_key").map_err(db_err)?,
            created_at: row.try_get("created_at").map_err(db_err)?,
            updated_at: row.try_get("updated_at").map_err(db_err)?,
        })
    }

    async fn fetch_full(&self, id: Uuid) -> Result<Option<SchemeFull>, RepositoryError> {
        let row =
            sqlx::query("SELECT id, name, description, created_at, updated_at FROM schemes WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;
        let Some(row) = row else { return Ok(None) };
        let scheme = Self::row_to_scheme(&row)?;

        let fields: Vec<SchemeField> = sqlx::query(
            "SELECT id, scheme_id, doc_kind, field_key, label, sort_key, created_at, updated_at \
             FROM scheme_fields WHERE scheme_id = $1 ORDER BY sort_key, created_at, id",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(Self::row_to_field)
        .collect::<Result<_, _>>()?;

        let tables: Vec<SchemeTable> = sqlx::query(
            "SELECT id, scheme_id, table_key, label, sort_key, created_at, updated_at \
             FROM scheme_tables WHERE scheme_id = $1 ORDER BY sort_key, created_at, id",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(Self::row_to_table)
        .collect::<Result<_, _>>()?;

        let cols: Vec<SchemeTableColumn> = sqlx::query(
            "SELECT c.id, c.table_id, c.column_key, c.label, c.sort_key, c.created_at, c.updated_at \
             FROM scheme_table_columns c JOIN scheme_tables t ON t.id = c.table_id \
             WHERE t.scheme_id = $1 ORDER BY t.sort_key, c.sort_key, c.created_at, c.id",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?
        .iter()
        .map(Self::row_to_column)
        .collect::<Result<_, _>>()?;

        let mut defs: Vec<SchemeTableDef> = tables
            .into_iter()
            .map(|table| SchemeTableDef {
                table,
                columns: Vec::new(),
            })
            .collect();
        for c in cols {
            if let Some(d) = defs.iter_mut().find(|d| d.table.id == c.table_id) {
                d.columns.push(c);
            }
        }

        Ok(Some(SchemeFull {
            scheme,
            fields,
            tables: defs,
        }))
    }
}

#[async_trait]
impl SchemeRepository for PostgresSchemeRepository {
    async fn list(&self) -> Result<Vec<Scheme>, RepositoryError> {
        let rows =
            sqlx::query("SELECT id, name, description, created_at, updated_at FROM schemes ORDER BY created_at, id")
                .fetch_all(&self.pool)
                .await
                .map_err(db_err)?;
        rows.iter().map(Self::row_to_scheme).collect()
    }

    async fn get_full(&self, id: Uuid) -> Result<Option<SchemeFull>, RepositoryError> {
        self.fetch_full(id).await
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Scheme>, RepositoryError> {
        let row =
            sqlx::query("SELECT id, name, description, created_at, updated_at FROM schemes WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;
        row.as_ref().map(Self::row_to_scheme).transpose()
    }

    async fn create(&self, spec: &SchemeSpec) -> Result<SchemeFull, RepositoryError> {
        let id = Uuid::new_v4();
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        sqlx::query(
            "INSERT INTO schemes (id, name, description, created_at, updated_at) \
             VALUES ($1,$2,$3,now(),now())",
        )
        .bind(id)
        .bind(&spec.name)
        .bind(&spec.description)
        .execute(&mut *tx)
        .await
        .map_err(Self::map_err)?;

        write_fields(&mut tx, id, &spec.fields).await?;
        write_tables(&mut tx, id, &spec.tables).await?;

        tx.commit().await.map_err(db_err)?;
        let full = self
            .fetch_full(id)
            .await?
            .ok_or_else(|| RepositoryError::Database("scheme vanished after create".into()))?;
        Ok(full)
    }

    async fn replace(&self, id: Uuid, spec: &SchemeSpec) -> Result<SchemeFull, RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db_err)?;

        let res = sqlx::query(
            "UPDATE schemes SET name = $2, description = $3, updated_at = now() WHERE id = $1",
        )
        .bind(id)
        .bind(&spec.name)
        .bind(&spec.description)
        .execute(&mut *tx)
        .await
        .map_err(Self::map_err)?;
        if res.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        write_fields(&mut tx, id, &spec.fields).await?;
        write_tables(&mut tx, id, &spec.tables).await?;

        tx.commit().await.map_err(db_err)?;
        let full = self
            .fetch_full(id)
            .await?
            .ok_or_else(|| RepositoryError::Database("scheme vanished after replace".into()))?;
        Ok(full)
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError> {
        let referenced: Option<Uuid> =
            sqlx::query_scalar("SELECT id FROM projects WHERE scheme_id = $1 LIMIT 1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;
        if referenced.is_some() {
            return Err(RepositoryError::Conflict);
        }
        let res = sqlx::query("DELETE FROM schemes WHERE id = $1")
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
// 整存字段 / 表（含列）：create 与 replace 共享的事务内写路径。
// sort_key 按数组序重编（0..n-1）。错误统一经 PostgresSchemeRepository::map_err。
// ---------------------------------------------------------------------------

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

async fn write_fields(
    tx: &mut Tx<'_>,
    scheme_id: Uuid,
    fields: &[crate::domain::scheme::SchemeFieldSpec],
) -> Result<(), RepositoryError> {
    delete_removed_fields(tx, scheme_id, fields).await?;
    for (i, f) in fields.iter().enumerate() {
        sqlx::query(
            "INSERT INTO scheme_fields \
             (id, scheme_id, doc_kind, field_key, label, sort_key, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5,$6,now(),now()) \
             ON CONFLICT (scheme_id, doc_kind, field_key) DO UPDATE \
             SET label = EXCLUDED.label, sort_key = EXCLUDED.sort_key, updated_at = now()",
        )
        .bind(Uuid::new_v4())
        .bind(scheme_id)
        .bind(&f.doc_kind)
        .bind(&f.field_key)
        .bind(&f.label)
        .bind(i as i32)
        .execute(&mut **tx)
        .await
        .map_err(PostgresSchemeRepository::map_err)?;
    }
    Ok(())
}

async fn write_tables(
    tx: &mut Tx<'_>,
    scheme_id: Uuid,
    tables: &[SchemeTableSpec],
) -> Result<(), RepositoryError> {
    delete_removed_tables(tx, scheme_id, tables).await?;
    for (i, t) in tables.iter().enumerate() {
        let row = sqlx::query(
            "INSERT INTO scheme_tables \
             (id, scheme_id, table_key, label, sort_key, created_at, updated_at) \
             VALUES ($1,$2,$3,$4,$5,now(),now()) \
             ON CONFLICT (scheme_id, table_key) DO UPDATE \
             SET label = EXCLUDED.label, sort_key = EXCLUDED.sort_key, updated_at = now() \
             RETURNING id",
        )
        .bind(Uuid::new_v4())
        .bind(scheme_id)
        .bind(&t.table_key)
        .bind(&t.label)
        .bind(i as i32)
        .fetch_one(&mut **tx)
        .await
        .map_err(PostgresSchemeRepository::map_err)?;
        let table_id: Uuid = row.get("id");

        delete_removed_columns(tx, table_id, t).await?;
        for (j, c) in t.columns.iter().enumerate() {
            sqlx::query(
                "INSERT INTO scheme_table_columns \
                 (id, table_id, column_key, label, sort_key, created_at, updated_at) \
                 VALUES ($1,$2,$3,$4,$5,now(),now()) \
                 ON CONFLICT (table_id, column_key) DO UPDATE \
                 SET label = EXCLUDED.label, sort_key = EXCLUDED.sort_key, updated_at = now()",
            )
            .bind(Uuid::new_v4())
            .bind(table_id)
            .bind(&c.column_key)
            .bind(&c.label)
            .bind(j as i32)
            .execute(&mut **tx)
            .await
            .map_err(PostgresSchemeRepository::map_err)?;
        }
    }
    Ok(())
}

async fn delete_removed_fields(
    tx: &mut Tx<'_>,
    scheme_id: Uuid,
    fields: &[crate::domain::scheme::SchemeFieldSpec],
) -> Result<(), RepositoryError> {
    if fields.is_empty() {
        sqlx::query("DELETE FROM scheme_fields WHERE scheme_id = $1")
            .bind(scheme_id)
            .execute(&mut **tx)
            .await
            .map_err(PostgresSchemeRepository::map_err)?;
        return Ok(());
    }
    let mut sql =
        "DELETE FROM scheme_fields WHERE scheme_id = $1 AND (doc_kind, field_key) NOT IN (VALUES ".to_string();
    for i in 0..fields.len() {
        if i > 0 {
            sql.push(',');
        }
        let p = 2 + 2 * i;
        sql.push_str(&format!("(${p},${})", p + 1));
    }
    sql.push(')');
    let mut q = sqlx::query(&sql).bind(scheme_id);
    for f in fields {
        q = q.bind(&f.doc_kind).bind(&f.field_key);
    }
    q.execute(&mut **tx)
        .await
        .map_err(PostgresSchemeRepository::map_err)?;
    Ok(())
}

async fn delete_removed_tables(
    tx: &mut Tx<'_>,
    scheme_id: Uuid,
    tables: &[SchemeTableSpec],
) -> Result<(), RepositoryError> {
    if tables.is_empty() {
        sqlx::query("DELETE FROM scheme_tables WHERE scheme_id = $1")
            .bind(scheme_id)
            .execute(&mut **tx)
            .await
            .map_err(PostgresSchemeRepository::map_err)?;
        return Ok(());
    }
    let mut sql = "DELETE FROM scheme_tables WHERE scheme_id = $1 AND table_key NOT IN (".to_string();
    for i in 0..tables.len() {
        if i > 0 {
            sql.push(',');
        }
        sql.push_str(&format!("${}", i + 2));
    }
    sql.push(')');
    let mut q = sqlx::query(&sql).bind(scheme_id);
    for t in tables {
        q = q.bind(&t.table_key);
    }
    q.execute(&mut **tx)
        .await
        .map_err(PostgresSchemeRepository::map_err)?;
    Ok(())
}

async fn delete_removed_columns(
    tx: &mut Tx<'_>,
    table_id: Uuid,
    table: &SchemeTableSpec,
) -> Result<(), RepositoryError> {
    if table.columns.is_empty() {
        sqlx::query("DELETE FROM scheme_table_columns WHERE table_id = $1")
            .bind(table_id)
            .execute(&mut **tx)
            .await
            .map_err(PostgresSchemeRepository::map_err)?;
        return Ok(());
    }
    let mut sql = "DELETE FROM scheme_table_columns WHERE table_id = $1 AND column_key NOT IN (".to_string();
    for i in 0..table.columns.len() {
        if i > 0 {
            sql.push(',');
        }
        sql.push_str(&format!("${}", i + 2));
    }
    sql.push(')');
    let mut q = sqlx::query(&sql).bind(table_id);
    for c in &table.columns {
        q = q.bind(&c.column_key);
    }
    q.execute(&mut **tx)
        .await
        .map_err(PostgresSchemeRepository::map_err)?;
    Ok(())
}
