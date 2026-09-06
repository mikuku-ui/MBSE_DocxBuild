//! 追踪图仓储 port。实现放 infrastructure（sqlx/Postgres），应用层只认 trait。

use async_trait::async_trait;
use uuid::Uuid;

use super::entity::{TraceBaseline, TraceLink, TraceNode};
use crate::domain::RepositoryError;

#[async_trait]
pub trait TraceNodeRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TraceNode>, RepositoryError>;

    /// 按项目 + 大系统幂等键查找（ingest / 幂等冲突检测用）。
    async fn find_by_external(
        &self,
        project_id: Uuid,
        external_source: &str,
        external_ref: &str,
    ) -> Result<Option<TraceNode>, RepositoryError>;

    /// 列某项目的全部节点（追踪数据按项目隔离）。
    async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceNode>, RepositoryError>;

    /// 幂等写入（external 键冲突则更新），返回最终落库的本地 id。
    async fn upsert(&self, node: &TraceNode) -> Result<Uuid, RepositoryError>;

    /// 批量覆写节点排序序号 `(id, sort_key)`；幂等，缺失/未知 id 静默忽略。
    async fn set_sort_keys(&self, orders: &[(Uuid, i32)]) -> Result<(), RepositoryError>;

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait TraceLinkRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<TraceLink>, RepositoryError>;

    /// 列某项目的全部链接（追踪数据按项目隔离）。
    async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceLink>, RepositoryError>;

    /// 幂等写入（source+target+link_type 冲突则更新），返回本地 id。
    async fn upsert(&self, link: &TraceLink) -> Result<Uuid, RepositoryError>;

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;
}

#[async_trait]
pub trait BaselineRepository: Send + Sync {
    /// 列某项目的基线（快照按项目隔离）。
    async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceBaseline>, RepositoryError>;

    async fn insert(&self, baseline: &TraceBaseline) -> Result<(), RepositoryError>;
}
