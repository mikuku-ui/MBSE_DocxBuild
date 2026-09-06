//! 项目数据服务：把「项目绑定体系的结构」×「本项目已填值」合并成
//! [`ProjectDataView`]，对外 GET/PUT `/api/projects/{id}/data`。
//!
//! - GET 语义：项目未绑体系 → 返回空视图（结构待绑定后才有）；绑了 → 按体系结构
//!   列出全部字段/表，值为本项目已填或空。
//! - PUT 语义（执行清单 / 项目页共用同一填数面）：**整存替换**——项目须已绑体系；
//!   body 引用的 field/table id 必须属于该项目当前体系（防跨体系/跨项目串写）；
//!   每表行按序重编 row_index，cells 只保留该表体系列内的键。

use std::collections::HashSet;
use std::sync::Arc;

use uuid::Uuid;

use crate::application::project::ProjectService;
use crate::application::scheme::SchemeService;
use crate::application::ServiceError;
use crate::domain::projectdata::{
    ProjectDataRepository, ProjectDataView, ProjectDataWrite, TableRowsWrite,
};

pub struct ProjectDataService {
    repo: Arc<dyn ProjectDataRepository>,
    projects: Arc<ProjectService>,
    schemes: Arc<SchemeService>,
}

impl ProjectDataService {
    pub fn new(
        repo: Arc<dyn ProjectDataRepository>,
        projects: Arc<ProjectService>,
        schemes: Arc<SchemeService>,
    ) -> Self {
        Self {
            repo,
            projects,
            schemes,
        }
    }

    /// 项目数据视图（结构 + 值）。项目不存在 → NotFound；未绑体系 → 空视图。
    pub async fn view(&self, project_id: Uuid) -> Result<ProjectDataView, ServiceError> {
        let project = self.projects.get(project_id).await?;
        let Some(scheme_id) = project.scheme_id else {
            return Ok(ProjectDataView {
                project_id,
                scheme_id: None,
                fields: Vec::new(),
                tables: Vec::new(),
            });
        };
        self.repo.get_view(project_id, scheme_id).await.map_err(Into::into)
    }

    /// 整存替换项目数据，返回存后的视图。
    pub async fn replace(
        &self,
        project_id: Uuid,
        write: ProjectDataWrite,
    ) -> Result<ProjectDataView, ServiceError> {
        let project = self.projects.get(project_id).await?;
        let scheme_id = project.scheme_id.ok_or_else(|| {
            ServiceError::Validation("project has no scheme bound; pick a scheme first".into())
        })?;
        let def = self.schemes.get(scheme_id).await?;

        let allowed_fields: HashSet<Uuid> = def.fields.iter().map(|f| f.id).collect();
        let allowed_tables: HashSet<Uuid> = def.tables.iter().map(|t| t.table.id).collect();

        // 表内可写列键（cells 只保留这些，防体系换列后旧键残留）。
        let table_cols: std::collections::HashMap<Uuid, HashSet<String>> = def
            .tables
            .iter()
            .map(|t| {
                (
                    t.table.id,
                    t.columns.iter().map(|c| c.column_key.clone()).collect(),
                )
            })
            .collect();

        let mut fields = Vec::with_capacity(write.fields.len());
        for f in write.fields {
            if !allowed_fields.contains(&f.field_id) {
                return Err(ServiceError::Validation(format!(
                    "field {} does not belong to the project scheme",
                    f.field_id
                )));
            }
            fields.push(f);
        }

        let mut tables = Vec::with_capacity(write.tables.len());
        for mut t in write.tables {
            if !allowed_tables.contains(&t.table_id) {
                return Err(ServiceError::Validation(format!(
                    "table {} does not belong to the project scheme",
                    t.table_id
                )));
            }
            let cols = table_cols.get(&t.table_id).cloned().unwrap_or_default();
            // 行序重编：按当前 row_index 稳定排序后从 0 顺排（防 gap/重复冲突）。
            t.rows.sort_by_key(|r| r.row_index);
            let mut rows = Vec::with_capacity(t.rows.len());
            for (i, mut r) in t.rows.into_iter().enumerate() {
                r.cells.retain(|k, _| cols.contains(k));
                r.cells = r
                    .cells
                    .into_iter()
                    .map(|(k, v)| (k, v.trim().to_string()))
                    .filter(|(_, v)| !v.is_empty())
                    .collect();
                r.row_index = i as i32;
                rows.push(r);
            }
            t.rows = rows;
            tables.push(TableRowsWrite {
                table_id: t.table_id,
                rows: t.rows,
            });
        }
        let write = ProjectDataWrite { fields, tables };

        self.repo.replace(project_id, &write).await?;
        self.view(project_id).await
    }
}
