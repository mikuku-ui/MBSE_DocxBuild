//! 体系服务：体系清单 / 完整定义 / 新建 / 整存替换（保 id）/ 删除。
//!
//! 写入形态是 [`SchemeSpec`]（纯业务列，无 id）；仓储按自然键 upsert 保已有 id，
//! 所以**编辑一套体系不会丢已填的项目值**——只有真正被删掉的字段/表才级联清值。
//! 删除被项目绑定的体系 → 409。

use std::collections::HashSet;
use std::sync::Arc;

use uuid::Uuid;

use crate::application::ServiceError;
use crate::domain::scheme::{Scheme, SchemeFull, SchemeRepository, SchemeSpec};
use crate::domain::RepositoryError;

pub struct SchemeService {
    repo: Arc<dyn SchemeRepository>,
}

impl SchemeService {
    pub fn new(repo: Arc<dyn SchemeRepository>) -> Self {
        Self { repo }
    }

    pub async fn list(&self) -> Result<Vec<Scheme>, ServiceError> {
        Ok(self.repo.list().await?)
    }

    /// 完整定义（含 id）。用于体系设计页编辑与装配取结构。
    pub async fn get(&self, id: Uuid) -> Result<SchemeFull, ServiceError> {
        self.repo
            .get_full(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("scheme_not_found".into()))
    }

    pub async fn create(&self, spec: SchemeSpec) -> Result<SchemeFull, ServiceError> {
        let spec = normalize(spec)?;
        match self.repo.create(&spec).await {
            Ok(full) => Ok(full),
            Err(RepositoryError::Conflict) => Err(ServiceError::Conflict(
                format!("scheme name already exists: {}", spec.name),
            )),
            Err(e) => Err(e.into()),
        }
    }

    /// 整存替换：先确认存在（NotFound），再做 upsert。撞名 → Conflict。
    pub async fn replace(&self, id: Uuid, spec: SchemeSpec) -> Result<SchemeFull, ServiceError> {
        let spec = normalize(spec)?;
        match self.repo.replace(id, &spec).await {
            Ok(full) => Ok(full),
            Err(RepositoryError::NotFound) => {
                Err(ServiceError::NotFound("scheme_not_found".into()))
            }
            Err(RepositoryError::Conflict) => Err(ServiceError::Conflict(format!(
                "scheme name already exists: {}",
                spec.name
            ))),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), ServiceError> {
        match self.repo.delete(id).await {
            Ok(()) => Ok(()),
            Err(RepositoryError::NotFound) => {
                Err(ServiceError::NotFound("scheme_not_found".into()))
            }
            Err(RepositoryError::Conflict) => Err(ServiceError::Conflict(
                "scheme is referenced by projects; unbind first".into(),
            )),
            Err(e) => Err(e.into()),
        }
    }
}

/// 归一化写入定义：trim + 默认值（label/column label 缺省 = key；doc_kind 缺省
/// 'project'）+ 结构完整性校验（key 非空、无重复自然键、列归属表且不重名）。
fn normalize(mut spec: SchemeSpec) -> Result<SchemeSpec, ServiceError> {
    spec.name = spec.name.trim().to_string();
    if spec.name.is_empty() {
        return Err(ServiceError::Validation("scheme name must not be empty".into()));
    }
    spec.description = spec.description.trim().to_string();

    let mut seen_fields: HashSet<(String, String)> = HashSet::new();
    let mut fields = Vec::with_capacity(spec.fields.len());
    for mut f in spec.fields {
        f.field_key = f.field_key.trim().to_string();
        if f.field_key.is_empty() {
            return Err(ServiceError::Validation(
                "scheme field_key must not be empty".into(),
            ));
        }
        let mut dk = f.doc_kind.trim().to_string();
        if dk.is_empty() {
            dk = "project".into();
        }
        f.doc_kind = dk;
        if f.label.trim().is_empty() {
            f.label = f.field_key.clone();
        } else {
            f.label = f.label.trim().to_string();
        }
        if !seen_fields.insert((f.doc_kind.clone(), f.field_key.clone())) {
            return Err(ServiceError::Validation(format!(
                "duplicate scheme field ({}, {})",
                f.doc_kind, f.field_key
            )));
        }
        fields.push(f);
    }
    spec.fields = fields;

    let mut seen_tables: HashSet<String> = HashSet::new();
    let mut tables = Vec::with_capacity(spec.tables.len());
    for mut t in spec.tables {
        t.table_key = t.table_key.trim().to_string();
        if t.table_key.is_empty() {
            return Err(ServiceError::Validation(
                "scheme table_key must not be empty".into(),
            ));
        }
        if t.label.trim().is_empty() {
            t.label = t.table_key.clone();
        } else {
            t.label = t.label.trim().to_string();
        }
        if !seen_tables.insert(t.table_key.clone()) {
            return Err(ServiceError::Validation(format!(
                "duplicate scheme table: {}",
                t.table_key
            )));
        }
        let mut seen_cols: HashSet<String> = HashSet::new();
        let mut cols = Vec::with_capacity(t.columns.len());
        for mut c in t.columns {
            c.column_key = c.column_key.trim().to_string();
            if c.column_key.is_empty() {
                return Err(ServiceError::Validation(format!(
                    "column key in table {} must not be empty",
                    t.table_key
                )));
            }
            if c.label.trim().is_empty() {
                c.label = c.column_key.clone();
            } else {
                c.label = c.label.trim().to_string();
            }
            if !seen_cols.insert(c.column_key.clone()) {
                return Err(ServiceError::Validation(format!(
                    "duplicate column {} in table {}",
                    c.column_key, t.table_key
                )));
            }
            cols.push(c);
        }
        t.columns = cols;
        tables.push(t);
    }
    spec.tables = tables;
    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::scheme::entity::SchemeColumnSpec;
    use crate::domain::scheme::{SchemeFieldSpec, SchemeTableSpec};

    fn spec(name: &str) -> SchemeSpec {
        SchemeSpec {
            name: name.into(),
            description: String::new(),
            fields: vec![SchemeFieldSpec {
                doc_kind: String::new(), // 空 → 归一为 'project'
                field_key: "项目代号".into(),
                label: String::new(), // 空 → = field_key
                sort_key: 0,
            }],
            tables: vec![SchemeTableSpec {
                table_key: "parties".into(),
                label: String::new(),
                sort_key: 0,
                columns: vec![SchemeColumnSpec {
                    column_key: "name".into(),
                    label: String::new(),
                    sort_key: 0,
                }],
            }],
        }
    }

    #[test]
    fn normalize_fills_defaults() {
        let s = normalize(spec("一套")).unwrap();
        assert_eq!(s.name, "一套");
        assert_eq!(s.fields[0].doc_kind, "project");
        assert_eq!(s.fields[0].label, "项目代号");
        assert_eq!(s.tables[0].label, "parties");
        assert_eq!(s.tables[0].columns[0].label, "name");
    }

    #[test]
    fn normalize_rejects_empty_name_key_and_duplicates() {
        assert!(normalize(spec("  ")).is_err()); // 空名
        let mut dup = spec("一套");
        dup.fields.push(SchemeFieldSpec {
            doc_kind: "project".into(),
            field_key: "项目代号".into(),
            label: String::new(),
            sort_key: 1,
        });
        assert!(normalize(dup).is_err()); // 重复字段
        let mut empty_key = spec("一套");
        empty_key.tables[0].columns[0].column_key = "  ".into();
        assert!(normalize(empty_key).is_err()); // 空列键
    }
}
