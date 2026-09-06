//! 项目数据仓储 port。实现放 infrastructure（sqlx/Postgres），应用层只认 trait。

use async_trait::async_trait;
use uuid::Uuid;

use super::entity::ProjectDataWrite;
use crate::domain::RepositoryError;

#[async_trait]
pub trait ProjectDataRepository: Send + Sync {
    /// 读回某项目在其绑定体系下的数据视图（结构 + 值合并，仓储内一次取齐）。
    /// 入参 `scheme_id` 来自调用方已取到的项目绑定；项目不存在 → NotFound。
    async fn get_view(
        &self,
        project_id: Uuid,
        scheme_id: Uuid,
    ) -> Result<super::entity::ProjectDataView, RepositoryError>;

    /// **整存替换**：单事务内先删该项目全部值行，再按 write 重插。
    /// `project_id` 一律以入参为准（防跨项目串写）；无行 = 清空该项目数据。
    async fn replace(
        &self,
        project_id: Uuid,
        write: &ProjectDataWrite,
    ) -> Result<(), RepositoryError>;
}
