//! 执行仓储 port。

use async_trait::async_trait;
use uuid::Uuid;

use super::entity::{ExecutionInstance, ExecutionItem};
use crate::domain::RepositoryError;

#[async_trait]
pub trait ExecutionRepository: Send + Sync {
    // —— 实例 ——
    async fn find_instance_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<ExecutionInstance>, RepositoryError>;
    async fn list_instances(
        &self,
        project_id: Uuid,
    ) -> Result<Vec<ExecutionInstance>, RepositoryError>;
    async fn insert_instance(&self, instance: &ExecutionInstance)
        -> Result<(), RepositoryError>;
    async fn update_instance(&self, instance: &ExecutionInstance)
        -> Result<(), RepositoryError>;

    // —— 清单项 ——
    async fn find_item_by_id(&self, id: Uuid) -> Result<Option<ExecutionItem>, RepositoryError>;
    async fn list_items_by_instance(
        &self,
        instance_id: Uuid,
    ) -> Result<Vec<ExecutionItem>, RepositoryError>;
    async fn insert_item(&self, item: &ExecutionItem) -> Result<(), RepositoryError>;
    async fn update_item(&self, item: &ExecutionItem) -> Result<(), RepositoryError>;
}
