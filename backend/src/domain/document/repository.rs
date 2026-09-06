//! 文档模板仓储 port（模板 + 节点 + 边，归一个 aggregate 管理）。

use async_trait::async_trait;
use uuid::Uuid;

use super::entity::{DocumentTemplate, TemplateEdge, TemplateNode};
use crate::domain::RepositoryError;

#[async_trait]
pub trait DocumentRepository: Send + Sync {
    // —— 模板 ——
    async fn find_template_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<DocumentTemplate>, RepositoryError>;
    async fn list_templates(&self) -> Result<Vec<DocumentTemplate>, RepositoryError>;
    async fn insert_template(&self, tpl: &DocumentTemplate) -> Result<(), RepositoryError>;
    async fn update_template(&self, tpl: &DocumentTemplate) -> Result<(), RepositoryError>;
    async fn delete_template(&self, id: Uuid) -> Result<(), RepositoryError>;

    // —— 节点 ——
    async fn find_node_by_id(&self, id: Uuid) -> Result<Option<TemplateNode>, RepositoryError>;
    async fn list_nodes_by_template(
        &self,
        template_id: Uuid,
    ) -> Result<Vec<TemplateNode>, RepositoryError>;
    async fn insert_node(&self, node: &TemplateNode) -> Result<(), RepositoryError>;
    async fn update_node(&self, node: &TemplateNode) -> Result<(), RepositoryError>;
    async fn delete_node(&self, id: Uuid) -> Result<(), RepositoryError>;
    /// 批量覆写 `(id, sort_key)`（导航窗格 / 画布拖拽重排共用，单条批量语句）。
    async fn set_sort_keys(&self, orders: &[(Uuid, i32)]) -> Result<(), RepositoryError>;

    // —— 边 ——
    async fn find_edge_by_id(&self, id: Uuid) -> Result<Option<TemplateEdge>, RepositoryError>;
    async fn list_edges_by_template(
        &self,
        template_id: Uuid,
    ) -> Result<Vec<TemplateEdge>, RepositoryError>;
    async fn insert_edge(&self, edge: &TemplateEdge) -> Result<(), RepositoryError>;
    async fn delete_edge(&self, id: Uuid) -> Result<(), RepositoryError>;
}
