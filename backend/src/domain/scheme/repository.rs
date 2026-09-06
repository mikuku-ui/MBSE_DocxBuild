//! 体系仓储 port。实现放 infrastructure（sqlx/Postgres），应用层只认 trait。

use async_trait::async_trait;
use uuid::Uuid;

use super::entity::{Scheme, SchemeFull, SchemeSpec};
use crate::domain::RepositoryError;

#[async_trait]
pub trait SchemeRepository: Send + Sync {
    /// 体系清单（元数据行，不含结构）。
    async fn list(&self) -> Result<Vec<Scheme>, RepositoryError>;

    /// 完整定义（含 id）。不存在 → None。
    async fn get_full(&self, id: Uuid) -> Result<Option<SchemeFull>, RepositoryError>;

    /// 按 id 找元数据（绑定校验用）。不存在 → None。
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Scheme>, RepositoryError>;

    /// 新建：单事务写 scheme + fields + tables + columns，返回带 id 的完整定义。
    /// 撞名（name UNIQUE）→ `Conflict`。
    async fn create(&self, spec: &SchemeSpec) -> Result<SchemeFull, RepositoryError>;

    /// **按自然键 upsert 保 id 的整存替换**：改字段/表/列定义不换 id → 项目值不丢；
    /// 只删除真正被移除的自然键。返回替换后的完整定义。不存在 → `NotFound`；
    /// 撞名 → `Conflict`。
    async fn replace(&self, id: Uuid, spec: &SchemeSpec) -> Result<SchemeFull, RepositoryError>;

    /// 删除体系。仍有项目绑定（projects.scheme_id 引用）→ `Conflict`。
    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
}
