//! 项目服务：项目 CRUD + 绑定/解绑「体系（执行方式）」。
//!
//! 删除项目即连同其下全部追踪数据一起清空——由 FK `ON DELETE CASCADE` 完成，
//! 这里只做存在性校验与撞名冲突翻译。绑定体系时校验体系存在（应用层给友好错误）；
//! 换体系清空已填项目数据的动作落在仓储层（`set_scheme`，单事务）。

use std::sync::Arc;

use uuid::Uuid;

use crate::application::ServiceError;
use crate::domain::project::{Project, ProjectRepository};
use crate::domain::scheme::SchemeRepository;
use crate::domain::RepositoryError;

pub struct ProjectService {
    repos: Arc<dyn ProjectRepository>,
    schemes: Arc<dyn SchemeRepository>,
}

impl ProjectService {
    pub fn new(repos: Arc<dyn ProjectRepository>, schemes: Arc<dyn SchemeRepository>) -> Self {
        Self { repos, schemes }
    }

    pub async fn list(&self) -> Result<Vec<Project>, ServiceError> {
        Ok(self.repos.list_all().await?)
    }

    pub async fn get(&self, id: Uuid) -> Result<Project, ServiceError> {
        self.repos
            .find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("project_not_found".into()))
    }

    pub async fn create(
        &self,
        name: String,
        scheme_id: Option<Uuid>,
    ) -> Result<Project, ServiceError> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(ServiceError::Validation(
                "project name must not be empty".into(),
            ));
        }
        if let Some(sid) = scheme_id {
            self.ensure_scheme_exists(sid).await?;
        }
        let project = Project::with_scheme(name.clone(), scheme_id);
        match self.repos.insert(&project).await {
            Ok(()) => Ok(project),
            Err(RepositoryError::Conflict) => Err(ServiceError::Conflict(format!(
                "project name already exists: {name}"
            ))),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn rename(&self, id: Uuid, name: String) -> Result<Project, ServiceError> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(ServiceError::Validation(
                "project name must not be empty".into(),
            ));
        }
        match self.repos.rename(id, &name).await {
            Ok(()) => self.get(id).await, // 重取新行（name 与 updated_at 已更新）
            Err(RepositoryError::NotFound) => {
                Err(ServiceError::NotFound("project_not_found".into()))
            }
            Err(RepositoryError::Conflict) => Err(ServiceError::Conflict(format!(
                "project name already exists: {name}"
            ))),
            Err(e) => Err(e.into()),
        }
    }

    /// 绑定 / 解绑体系。换体系会在仓储层清空该项目已填数据（幂等语义见仓库注释）。
    pub async fn bind_scheme(
        &self,
        id: Uuid,
        scheme_id: Option<Uuid>,
    ) -> Result<Project, ServiceError> {
        if let Some(sid) = scheme_id {
            self.ensure_scheme_exists(sid).await?;
        }
        match self.repos.set_scheme(id, scheme_id).await {
            Ok(()) => self.get(id).await,
            Err(RepositoryError::NotFound) => {
                Err(ServiceError::NotFound("project_not_found".into()))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), ServiceError> {
        match self.repos.delete(id).await {
            Ok(()) => Ok(()),
            Err(RepositoryError::NotFound) => {
                Err(ServiceError::NotFound("project_not_found".into()))
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn ensure_scheme_exists(&self, scheme_id: Uuid) -> Result<(), ServiceError> {
        match self.schemes.find_by_id(scheme_id).await {
            Ok(Some(_)) => Ok(()),
            Ok(None) => Err(ServiceError::NotFound("scheme_not_found".into())),
            Err(e) => Err(e.into()),
        }
    }
}
