//! 执行服务：模板 + 执行阶段 → 执行清单（task 列表化，非 workflow 流式调取）。
//!
//! 清单**结构**由模板节点派生（读取期，不落库）；执行人员填写的**内容**是用户
//! 产物，落库到 `execution_items.content`——两者严格分离。

use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::application::document::DocumentService;
use crate::application::ServiceError;
use crate::domain::document::DocumentRepository;
use crate::domain::execution::{
    ExecutionInstance, ExecutionItem, ExecutionRepository, InstanceStatus, ItemStatus,
};

pub struct ExecutionService {
    docs: Arc<dyn DocumentRepository>,
    execution: Arc<dyn ExecutionRepository>,
}

impl ExecutionService {
    pub fn new(docs: Arc<dyn DocumentRepository>, execution: Arc<dyn ExecutionRepository>) -> Self {
        Self { docs, execution }
    }

    /// 从模板为一个**项目**派生一个执行实例：每个模板节点 → 一条清单项。
    ///
    /// 顺序按文档序（[`DocumentService::ordered_nodes`]：大章节竖向、同级子章节
    /// 横向，横向越靠左在文档中越靠前）——即执行清单的填写顺序跟随文档从前往后。
    pub async fn create_instance(
        &self,
        project_id: Uuid,
        template_id: Uuid,
        stage: Option<String>,
    ) -> Result<ExecutionInstance, ServiceError> {
        if self.docs.find_template_by_id(template_id).await?.is_none() {
            return Err(ServiceError::NotFound("document_template_not_found".into()));
        }
        let instance = ExecutionInstance::new(project_id, template_id, stage);
        self.execution.insert_instance(&instance).await?;

        let nodes = self.docs.list_nodes_by_template(template_id).await?;
        let edges = self.docs.list_edges_by_template(template_id).await?;
        let ordered = DocumentService::ordered_nodes(nodes, &edges);
        for (i, n) in ordered.iter().enumerate() {
            let item = ExecutionItem::new(instance.id, n.id, n.title.clone(), i as i32);
            self.execution.insert_item(&item).await?;
        }
        Ok(instance)
    }

    pub async fn list_instances(&self, project_id: Uuid) -> Result<Vec<ExecutionInstance>, ServiceError> {
        Ok(self.execution.list_instances(project_id).await?)
    }

    pub async fn get_instance(&self, id: Uuid) -> Result<ExecutionInstance, ServiceError> {
        self.execution
            .find_instance_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("execution_instance_not_found".into()))
    }

    /// 实例 + 其清单项（执行人员侧视图）。
    pub async fn get_instance_detail(
        &self,
        id: Uuid,
    ) -> Result<(ExecutionInstance, Vec<ExecutionItem>), ServiceError> {
        let instance = self.get_instance(id).await?;
        let items = self.execution.list_items_by_instance(id).await?;
        Ok((instance, items))
    }

    pub async fn update_instance(
        &self,
        id: Uuid,
        status: Option<InstanceStatus>,
        stage: Option<Option<String>>,
    ) -> Result<ExecutionInstance, ServiceError> {
        let mut instance = self.get_instance(id).await?;
        if let Some(s) = status {
            instance.status = s;
        }
        if let Some(st) = stage {
            instance.stage = st;
        }
        instance.updated_at = chrono::Utc::now();
        self.execution.update_instance(&instance).await?;
        Ok(instance)
    }

    pub async fn list_items(&self, instance_id: Uuid) -> Result<Vec<ExecutionItem>, ServiceError> {
        self.get_instance(instance_id).await?;
        Ok(self.execution.list_items_by_instance(instance_id).await?)
    }

    /// 更新清单项状态 / 内容。
    pub async fn update_item(
        &self,
        id: Uuid,
        status: Option<ItemStatus>,
        content: Option<Value>,
    ) -> Result<ExecutionItem, ServiceError> {
        let mut item = self
            .execution
            .find_item_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("execution_item_not_found".into()))?;
        if let Some(s) = status {
            item.status = s;
        }
        if let Some(c) = content {
            item.content = c;
        }
        item.updated_at = chrono::Utc::now();
        self.execution.update_item(&item).await?;
        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use async_trait::async_trait;

    use crate::domain::document::{
        DocumentRepository, DocumentTemplate, TemplateEdge, TemplateNode,
    };
    use crate::domain::execution::{
        ExecutionInstance, ExecutionItem, ExecutionRepository, InstanceStatus, ItemStatus,
    };

    struct MemDocs(Mutex<Vec<DocumentTemplate>>, Mutex<Vec<TemplateNode>>);
    struct MemExec(Mutex<Vec<ExecutionInstance>>, Mutex<Vec<ExecutionItem>>);

    /// 测试项目 id（与 0007 默认示例项目常量一致）。
    fn pid() -> Uuid {
        Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0001)
    }

    #[async_trait]
    impl DocumentRepository for MemDocs {
        async fn find_template_by_id(&self, id: uuid::Uuid) -> Result<Option<DocumentTemplate>, crate::domain::RepositoryError> {
            Ok(self.0.lock().unwrap().iter().find(|t| t.id == id).cloned())
        }
        async fn list_templates(&self) -> Result<Vec<DocumentTemplate>, crate::domain::RepositoryError> {
            Ok(self.0.lock().unwrap().clone())
        }
        async fn insert_template(&self, t: &DocumentTemplate) -> Result<(), crate::domain::RepositoryError> {
            self.0.lock().unwrap().push(t.clone());
            Ok(())
        }
        async fn update_template(&self, t: &DocumentTemplate) -> Result<(), crate::domain::RepositoryError> {
            let mut g = self.0.lock().unwrap();
            if let Some(e) = g.iter_mut().find(|x| x.id == t.id) { *e = t.clone(); }
            Ok(())
        }
        async fn delete_template(&self, id: uuid::Uuid) -> Result<(), crate::domain::RepositoryError> {
            self.0.lock().unwrap().retain(|t| t.id != id);
            Ok(())
        }
        async fn find_node_by_id(&self, id: uuid::Uuid) -> Result<Option<TemplateNode>, crate::domain::RepositoryError> {
            Ok(self.1.lock().unwrap().iter().find(|n| n.id == id).cloned())
        }
        async fn list_nodes_by_template(&self, tid: uuid::Uuid) -> Result<Vec<TemplateNode>, crate::domain::RepositoryError> {
            Ok(self.1.lock().unwrap().iter().filter(|n| n.template_id == tid).cloned().collect())
        }
        async fn insert_node(&self, n: &TemplateNode) -> Result<(), crate::domain::RepositoryError> {
            self.1.lock().unwrap().push(n.clone());
            Ok(())
        }
        async fn update_node(&self, n: &TemplateNode) -> Result<(), crate::domain::RepositoryError> {
            let mut g = self.1.lock().unwrap();
            if let Some(e) = g.iter_mut().find(|x| x.id == n.id) { *e = n.clone(); }
            Ok(())
        }
        async fn delete_node(&self, id: uuid::Uuid) -> Result<(), crate::domain::RepositoryError> {
            self.1.lock().unwrap().retain(|n| n.id != id);
            Ok(())
        }
        async fn set_sort_keys(&self, orders: &[(uuid::Uuid, i32)]) -> Result<(), crate::domain::RepositoryError> {
            let mut g = self.1.lock().unwrap();
            for (id, key) in orders {
                if let Some(n) = g.iter_mut().find(|x| x.id == *id) {
                    n.sort_key = *key;
                }
            }
            Ok(())
        }
        async fn find_edge_by_id(&self, _id: uuid::Uuid) -> Result<Option<TemplateEdge>, crate::domain::RepositoryError> {
            Ok(None)
        }
        async fn list_edges_by_template(&self, _tid: uuid::Uuid) -> Result<Vec<TemplateEdge>, crate::domain::RepositoryError> {
            Ok(vec![])
        }
        async fn insert_edge(&self, _e: &TemplateEdge) -> Result<(), crate::domain::RepositoryError> {
            Ok(())
        }
        async fn delete_edge(&self, _id: uuid::Uuid) -> Result<(), crate::domain::RepositoryError> {
            Ok(())
        }
    }

    #[async_trait]
    impl ExecutionRepository for MemExec {
        async fn find_instance_by_id(&self, id: uuid::Uuid) -> Result<Option<ExecutionInstance>, crate::domain::RepositoryError> {
            Ok(self.0.lock().unwrap().iter().find(|i| i.id == id).cloned())
        }
        async fn list_instances(&self, project_id: uuid::Uuid) -> Result<Vec<ExecutionInstance>, crate::domain::RepositoryError> {
            Ok(self.0.lock().unwrap().iter().filter(|i| i.project_id == project_id).cloned().collect())
        }
        async fn insert_instance(&self, i: &ExecutionInstance) -> Result<(), crate::domain::RepositoryError> {
            self.0.lock().unwrap().push(i.clone());
            Ok(())
        }
        async fn update_instance(&self, i: &ExecutionInstance) -> Result<(), crate::domain::RepositoryError> {
            let mut g = self.0.lock().unwrap();
            if let Some(e) = g.iter_mut().find(|x| x.id == i.id) { *e = i.clone(); }
            Ok(())
        }
        async fn find_item_by_id(&self, id: uuid::Uuid) -> Result<Option<ExecutionItem>, crate::domain::RepositoryError> {
            Ok(self.1.lock().unwrap().iter().find(|x| x.id == id).cloned())
        }
        async fn list_items_by_instance(&self, iid: uuid::Uuid) -> Result<Vec<ExecutionItem>, crate::domain::RepositoryError> {
            Ok(self.1.lock().unwrap().iter().filter(|x| x.instance_id == iid).cloned().collect())
        }
        async fn insert_item(&self, x: &ExecutionItem) -> Result<(), crate::domain::RepositoryError> {
            self.1.lock().unwrap().push(x.clone());
            Ok(())
        }
        async fn update_item(&self, x: &ExecutionItem) -> Result<(), crate::domain::RepositoryError> {
            let mut g = self.1.lock().unwrap();
            if let Some(e) = g.iter_mut().find(|y| y.id == x.id) { *e = x.clone(); }
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_instance_derives_items_from_template_nodes_in_order() {
        let docs = Arc::new(MemDocs(Mutex::new(vec![]), Mutex::new(vec![])));
        let exec = Arc::new(MemExec(Mutex::new(vec![]), Mutex::new(vec![])));

        let doc_svc = crate::application::document::DocumentService::new(docs.clone());
        let tpl = doc_svc.create_template("大纲A".into(), "大纲".into(), None).await.unwrap();
        // 故意乱序的 sort_key：应被确定性排序纠正
        doc_svc.create_node(tpl.id, crate::domain::document::TemplateNodeType::Chapter, "第2章".into(), None, Some(2)).await.unwrap();
        doc_svc.create_node(tpl.id, crate::domain::document::TemplateNodeType::Chapter, "第1章".into(), None, Some(1)).await.unwrap();
        doc_svc.create_node(tpl.id, crate::domain::document::TemplateNodeType::TraceView, "需求追溯".into(), None, Some(3)).await.unwrap();

        let exec_svc = crate::application::execution::ExecutionService::new(docs, exec.clone());
        let instance = exec_svc
            .create_instance(pid(), tpl.id, Some("第一轮".into()))
            .await
            .unwrap();
        assert_eq!(instance.status, InstanceStatus::Draft);
        assert_eq!(instance.project_id, pid());
        assert_eq!(exec_svc.list_instances(pid()).await.unwrap().len(), 1);

        let items = exec_svc.list_items(instance.id).await.unwrap();
        assert_eq!(items.len(), 3);
        let titles: Vec<&str> = items.iter().map(|i| i.title.as_str()).collect();
        assert_eq!(titles, vec!["第1章", "第2章", "需求追溯"]);
        assert!(items.iter().all(|i| i.status == ItemStatus::Pending));

        // 内容落库、状态更新
        let updated = exec_svc
            .update_item(items[0].id, Some(ItemStatus::InProgress), Some(serde_json::json!({"text": "已填写"})))
            .await
            .unwrap();
        assert_eq!(updated.status, ItemStatus::InProgress);
        assert_eq!(updated.content["text"], "已填写");
    }

    #[tokio::test]
    async fn create_instance_requires_existing_template() {
        let docs = Arc::new(MemDocs(Mutex::new(vec![]), Mutex::new(vec![])));
        let exec = Arc::new(MemExec(Mutex::new(vec![]), Mutex::new(vec![])));
        let exec_svc = crate::application::execution::ExecutionService::new(docs, exec);
        let err = exec_svc.create_instance(pid(), uuid::Uuid::new_v4(), None).await.unwrap_err();
        assert!(matches!(err, ServiceError::NotFound(_)));
    }

    #[tokio::test]
    async fn execution_instances_are_isolated_by_project() {
        let docs = Arc::new(MemDocs(Mutex::new(vec![]), Mutex::new(vec![])));
        let exec = Arc::new(MemExec(Mutex::new(vec![]), Mutex::new(vec![])));
        let doc_svc = crate::application::document::DocumentService::new(docs.clone());
        let tpl = doc_svc.create_template("报告模板".into(), "报告".into(), None).await.unwrap();
        let exec_svc = crate::application::execution::ExecutionService::new(docs, exec.clone());

        let a = pid();
        let b = Uuid::new_v4();
        exec_svc.create_instance(a, tpl.id, None).await.unwrap();
        exec_svc.create_instance(a, tpl.id, Some("第二轮".into())).await.unwrap();

        // 项目 A 有 2 条实例；项目 B 看不到
        assert_eq!(exec_svc.list_instances(a).await.unwrap().len(), 2);
        assert!(exec_svc.list_instances(b).await.unwrap().is_empty());

        // 项目 B 创建后自己可见，且不回串到 A
        let inst_b = exec_svc.create_instance(b, tpl.id, None).await.unwrap();
        assert_eq!(exec_svc.list_instances(b).await.unwrap().len(), 1);
        assert_eq!(inst_b.project_id, b);
        assert_eq!(exec_svc.list_instances(a).await.unwrap().len(), 2);
    }
}
