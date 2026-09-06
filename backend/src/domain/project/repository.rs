//! 项目仓储 port。实现放 infrastructure（sqlx/Postgres），应用层只认 trait。

use async_trait::async_trait;
use uuid::Uuid;

use super::entity::Project;
use crate::domain::RepositoryError;

#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Project>, RepositoryError>;

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Project>, RepositoryError>;

    /// 新建项目。项目名唯一（`name UNIQUE`），撞名返回 `RepositoryError::Conflict`。
    async fn insert(&self, project: &Project) -> Result<(), RepositoryError>;

    /// 重命名。不存在返回 `NotFound`；撞名返回 `Conflict`。
    async fn rename(&self, id: Uuid, name: &str) -> Result<(), RepositoryError>;

    /// 绑定 / 解绑体系（`scheme_id=None` = 解绑）。**切到不同体系时先清空该项目
    /// 已填的项目数据**（值引用旧体系字段/表 id，换体系后无意义）；绑定同一体系为
    /// 幂等空操作（不清数据）。不存在返回 `NotFound`。
    async fn set_scheme(&self, id: Uuid, scheme_id: Option<Uuid>) -> Result<(), RepositoryError>;

    /// 删除项目（其下追踪数据由 FK ON DELETE CASCADE 一并清空）。不存在返回 `NotFound`。
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
}
