//! 文档模板服务：模板/节点/边的 CRUD + 确定性结构顺序。
//!
//! 文档模板与需求追踪图是**两张独立的图**；`trace_view` 节点只在渲染期桥接追踪
//! 图，这里不做任何追踪图业务（对齐 knowledge-base P10）。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::application::ServiceError;
use crate::domain::document::{
    DocumentRepository, DocumentTemplate, TemplateEdge, TemplateNode, TemplateNodeType,
};

pub struct DocumentService {
    docs: Arc<dyn DocumentRepository>,
}

impl DocumentService {
    pub fn new(docs: Arc<dyn DocumentRepository>) -> Self {
        Self { docs }
    }

    // ---- 模板 ----

    pub async fn list_templates(&self) -> Result<Vec<DocumentTemplate>, ServiceError> {
        Ok(self.docs.list_templates().await?)
    }

    pub async fn get_template(&self, id: Uuid) -> Result<DocumentTemplate, ServiceError> {
        self.docs
            .find_template_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("document_template_not_found".into()))
    }

    pub async fn create_template(
        &self,
        name: String,
        kind: String,
        description: Option<String>,
    ) -> Result<DocumentTemplate, ServiceError> {
        if name.trim().is_empty() {
            return Err(ServiceError::Validation(
                "template name must not be empty".into(),
            ));
        }
        if kind.trim().is_empty() {
            return Err(ServiceError::Validation(
                "template kind must not be empty".into(),
            ));
        }
        let tpl = DocumentTemplate::new(name.trim().to_string(), kind.trim().to_string(), description);
        self.docs.insert_template(&tpl).await?;
        Ok(tpl)
    }

    pub async fn update_template(
        &self,
        id: Uuid,
        name: Option<String>,
        kind: Option<String>,
        description: Option<String>,
    ) -> Result<DocumentTemplate, ServiceError> {
        let mut tpl = self.get_template(id).await?;
        if let Some(n) = name {
            if n.trim().is_empty() {
                return Err(ServiceError::Validation(
                    "template name must not be empty".into(),
                ));
            }
            tpl.name = n.trim().to_string();
        }
        if let Some(k) = kind {
            if k.trim().is_empty() {
                return Err(ServiceError::Validation(
                    "template kind must not be empty".into(),
                ));
            }
            tpl.kind = k.trim().to_string();
        }
        if let Some(d) = description {
            tpl.description = Some(d);
        }
        tpl.updated_at = chrono::Utc::now();
        self.docs.update_template(&tpl).await?;
        Ok(tpl)
    }

    pub async fn delete_template(&self, id: Uuid) -> Result<(), ServiceError> {
        self.get_template(id).await?;
        self.docs.delete_template(id).await?;
        Ok(())
    }

    /// 模板完整视图：模板 + 有序节点 + 边（画布渲染数据源）。
    pub async fn get_template_full(
        &self,
        id: Uuid,
    ) -> Result<(DocumentTemplate, Vec<TemplateNode>, Vec<TemplateEdge>), ServiceError> {
        let tpl = self.get_template(id).await?;
        let nodes = Self::sort_nodes(self.docs.list_nodes_by_template(id).await?);
        let edges = self.docs.list_edges_by_template(id).await?;
        Ok((tpl, nodes, edges))
    }

    // ---- 节点 ----

    /// 确定性结构顺序：sort_key → created_at → id（稳定可复现，利于 diff/导出）。
    pub fn sort_nodes(mut nodes: Vec<TemplateNode>) -> Vec<TemplateNode> {
        nodes.sort_by_key(|n| (n.sort_key, n.created_at, n.id));
        nodes
    }

    /// **文档序**（DFS 先序）：父节点在前，其直接子节点自左到右随后，再递归进
    /// 更深层级——即「大章节竖向、同级子章节横向，横向越靠左在文档中越靠前、
    /// 竖向最上方是文档最前」的线性投影。
    ///
    /// 父子关系取自 `contains` 边（**source=父、target=子**）。没有 contains 边时
    /// 退化为扁平排序（与 [`Self::sort_nodes`] 一致）；环/悬空等未被任何 root 到达
    /// 的节点兜底按扁平排序追加在末尾。
    pub fn ordered_nodes(
        nodes: Vec<TemplateNode>,
        edges: &[TemplateEdge],
    ) -> Vec<TemplateNode> {
        let cmp_node = |a: &TemplateNode, b: &TemplateNode| {
            (a.sort_key, a.created_at, a.id).cmp(&(b.sort_key, b.created_at, b.id))
        };

        let mut children: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let mut has_parent: HashSet<Uuid> = HashSet::new();
        for e in edges {
            if e.edge_type != "contains" {
                continue;
            }
            children
                .entry(e.source_node_id)
                .or_default()
                .push(e.target_node_id);
            has_parent.insert(e.target_node_id);
        }
        let by_id: HashMap<Uuid, &TemplateNode> = nodes.iter().map(|n| (n.id, n)).collect();
        for kids in children.values_mut() {
            kids.sort_by(|a, b| match (by_id.get(a), by_id.get(b)) {
                (Some(x), Some(y)) => cmp_node(x, y),
                _ => a.cmp(b),
            });
        }
        let mut roots: Vec<Uuid> = nodes
            .iter()
            .filter(|n| !has_parent.contains(&n.id))
            .map(|n| n.id)
            .collect();
        roots.sort_by(|a, b| match (by_id.get(a), by_id.get(b)) {
            (Some(x), Some(y)) => cmp_node(x, y),
            _ => a.cmp(b),
        });

        fn visit(
            id: Uuid,
            children: &HashMap<Uuid, Vec<Uuid>>,
            by_id: &HashMap<Uuid, &TemplateNode>,
            seen: &mut HashSet<Uuid>,
            out: &mut Vec<TemplateNode>,
        ) {
            if !seen.insert(id) {
                return; // 防环：已访问过的不再进入
            }
            if let Some(n) = by_id.get(&id) {
                out.push((*n).clone());
            }
            if let Some(kids) = children.get(&id) {
                for k in kids {
                    visit(*k, children, by_id, seen, out);
                }
            }
        }

        let mut out: Vec<TemplateNode> = Vec::with_capacity(nodes.len());
        let mut seen: HashSet<Uuid> = HashSet::with_capacity(nodes.len());
        for r in roots {
            visit(r, &children, &by_id, &mut seen, &mut out);
        }
        // 兜底：未被任何 root 到达（环 / 悬空）的节点按扁平排序补在末尾
        let mut leftovers: Vec<&TemplateNode> = nodes
            .iter()
            .filter(|n| !seen.contains(&n.id))
            .collect();
        leftovers.sort_by(|a, b| cmp_node(a, b));
        out.extend(leftovers.into_iter().cloned());
        out
    }

    pub async fn list_nodes(&self, template_id: Uuid) -> Result<Vec<TemplateNode>, ServiceError> {
        self.get_template(template_id).await?;
        Ok(Self::sort_nodes(
            self.docs.list_nodes_by_template(template_id).await?,
        ))
    }

    pub async fn create_node(
        &self,
        template_id: Uuid,
        node_type: TemplateNodeType,
        title: String,
        content_spec: Option<Value>,
        sort_key: Option<i32>,
    ) -> Result<TemplateNode, ServiceError> {
        self.get_template(template_id).await?;
        if title.trim().is_empty() {
            return Err(ServiceError::Validation("node title must not be empty".into()));
        }
        let node = TemplateNode::new(
            template_id,
            node_type,
            title.trim().to_string(),
            content_spec.unwrap_or_else(|| serde_json::json!({})),
            sort_key.unwrap_or(0),
        );
        self.docs.insert_node(&node).await?;
        Ok(node)
    }

    pub async fn update_node(
        &self,
        id: Uuid,
        node_type: Option<TemplateNodeType>,
        title: Option<String>,
        content_spec: Option<Value>,
        sort_key: Option<i32>,
        tex_component: Option<String>,
    ) -> Result<TemplateNode, ServiceError> {
        let mut node = self
            .docs
            .find_node_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("document_template_node_not_found".into()))?;
        if let Some(t) = node_type {
            node.node_type = t;
        }
        if let Some(t) = title {
            if t.trim().is_empty() {
                return Err(ServiceError::Validation("node title must not be empty".into()));
            }
            node.title = t.trim().to_string();
        }
        if let Some(c) = content_spec {
            node.content_spec = c;
        }
        if let Some(k) = sort_key {
            node.sort_key = k;
        }
        if let Some(t) = tex_component {
            node.tex_component = Some(t);
        }
        node.updated_at = chrono::Utc::now();
        self.docs.update_node(&node).await?;
        Ok(node)
    }

    pub async fn delete_node(&self, id: Uuid) -> Result<(), ServiceError> {
        if self.docs.find_node_by_id(id).await?.is_none() {
            return Err(ServiceError::NotFound(
                "document_template_node_not_found".into(),
            ));
        }
        self.docs.delete_node(id).await?;
        Ok(())
    }

    /// 批量重排：单条批量语句覆写 `(id, sort_key)`（导航窗格拖拽 / 画布同组纵向拖拽共用）。
    /// 非本模板节点的 id 被忽略（对齐 trace 的 `apply_order` 语义）。
    pub async fn reorder_nodes(
        &self,
        template_id: Uuid,
        orders: Vec<(Uuid, i32)>,
    ) -> Result<Vec<TemplateNode>, ServiceError> {
        self.get_template(template_id).await?;
        let mut nodes = self.docs.list_nodes_by_template(template_id).await?;
        let ids: HashSet<Uuid> = nodes.iter().map(|n| n.id).collect();
        let scoped: Vec<(Uuid, i32)> = orders
            .into_iter()
            .filter(|(id, _)| ids.contains(id))
            .collect();
        // 返回前先在内存里套用新 sort_key（返回值即最新序，前端 reload 前即可用）
        let key_by_id: HashMap<Uuid, i32> = scoped.iter().copied().collect();
        for n in nodes.iter_mut() {
            if let Some(k) = key_by_id.get(&n.id) {
                n.sort_key = *k;
            }
        }
        self.docs.set_sort_keys(&scoped).await?;
        Ok(Self::sort_nodes(nodes))
    }

    // ---- 边 ----

    pub async fn add_edge(
        &self,
        template_id: Uuid,
        source_node_id: Uuid,
        target_node_id: Uuid,
        edge_type: String,
    ) -> Result<TemplateEdge, ServiceError> {
        self.get_template(template_id).await?;
        if source_node_id == target_node_id {
            return Err(ServiceError::Validation(
                "edge source and target must differ".into(),
            ));
        }
        let nodes = self.docs.list_nodes_by_template(template_id).await?;
        let ids: Vec<Uuid> = nodes.iter().map(|n| n.id).collect();
        if !ids.contains(&source_node_id) || !ids.contains(&target_node_id) {
            return Err(ServiceError::NotFound(
                "edge endpoint node not in template".into(),
            ));
        }
        let edge = TemplateEdge {
            id: Uuid::new_v4(),
            template_id,
            source_node_id,
            target_node_id,
            edge_type: if edge_type.trim().is_empty() {
                "contains".to_string()
            } else {
                edge_type
            },
            created_at: chrono::Utc::now(),
        };
        self.docs.insert_edge(&edge).await?;
        Ok(edge)
    }

    pub async fn remove_edge(&self, id: Uuid) -> Result<(), ServiceError> {
        if self.docs.find_edge_by_id(id).await?.is_none() {
            return Err(ServiceError::NotFound("document_template_edge_not_found".into()));
        }
        self.docs.delete_edge(id).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, template_id: Uuid, title: &str, sort_key: i32) -> TemplateNode {
        let mut n = TemplateNode::new(
            template_id,
            TemplateNodeType::Section,
            title.into(),
            serde_json::json!({}),
            sort_key,
        );
        n.id = Uuid::parse_str(id).unwrap();
        n
    }

    fn contains(template_id: Uuid, parent: &TemplateNode, child: &TemplateNode) -> TemplateEdge {
        TemplateEdge {
            id: Uuid::new_v4(),
            template_id,
            source_node_id: parent.id,
            target_node_id: child.id,
            edge_type: "contains".into(),
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn ordered_nodes_without_edges_falls_back_to_flat_sort() {
        let tid = Uuid::new_v4();
        let a = node("10000000-0000-0000-0000-00000000000a", tid, "A", 5);
        let b = node("10000000-0000-0000-0000-00000000000b", tid, "B", 1);
        let c = node("10000000-0000-0000-0000-00000000000c", tid, "C", 3);
        let ordered = DocumentService::ordered_nodes(vec![a, b, c], &[]);
        let titles: Vec<&str> = ordered.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["B", "C", "A"]);
    }

    #[test]
    fn ordered_nodes_puts_parent_before_children_in_document_order() {
        let tid = Uuid::new_v4();
        // 章节（竖向）：ch1、ch2；ch1 下同级子章节（横向）：s1、s2。
        let ch1 = node("10000000-0000-0000-0000-000000000001", tid, "第1章", 0);
        let ch2 = node("10000000-0000-0000-0000-000000000002", tid, "第2章", 1);
        let s1 = node("10000000-0000-0000-0000-000000000011", tid, "1.1", 1);
        let s2 = node("10000000-0000-0000-0000-000000000012", tid, "1.2", 2);
        let edges = vec![
            contains(tid, &ch1, &s1),
            contains(tid, &ch1, &s2),
        ];
        // 乱序输入：仍应按文档序（DFS 先序）输出
        let ordered = DocumentService::ordered_nodes(vec![s2, ch1, ch2, s1], &edges);
        let titles: Vec<&str> = ordered.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["第1章", "1.1", "1.2", "第2章"]);
    }

    #[test]
    fn ordered_nodes_ignores_non_contains_edges_and_cycles() {
        // 非 contains 边不参与父子结构：references 反向边不应改变 contains 序
        let tid = Uuid::new_v4();
        let ch1 = node("10000000-0000-0000-0000-000000000001", tid, "第1章", 0);
        let s1 = node("10000000-0000-0000-0000-000000000011", tid, "1.1", 0);
        let ref_back = TemplateEdge {
            edge_type: "references".into(),
            ..contains(tid, &ch1, &s1)
        };
        let ordered = DocumentService::ordered_nodes(
            vec![s1.clone(), ch1.clone()],
            &[contains(tid, &ch1, &s1), ref_back],
        );
        let titles: Vec<&str> = ordered.iter().map(|n| n.title.as_str()).collect();
        assert_eq!(titles, vec!["第1章", "1.1"]);

        // 真环（双向 contains）也不死循环：每个节点恰好出现一次，终止正常
        let mut back = contains(tid, &ch1, &s1);
        back.source_node_id = s1.id;
        back.target_node_id = ch1.id;
        let cyc = DocumentService::ordered_nodes(vec![s1.clone(), ch1.clone()], &[
            contains(tid, &ch1, &s1),
            back,
        ]);
        assert_eq!(cyc.len(), 2);
    }
}
