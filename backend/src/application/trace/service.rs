//! 追踪服务：bulk ingest（幂等 upsert）+ CRUD + 派生视图 + 基线快照。
//!
//! 追踪数据按**项目**隔离（0007 迁移起 trace_nodes/links/baselines 均带
//! `project_id`）：所有方法第一个参数是 `project_id`，派生视图只在本项目图内现算。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::application::ServiceError;
use crate::domain::trace::{
    BaselineRepository, LinkType, TraceBaseline, TraceLink, TraceNode, TraceNodeKind,
    TraceLinkRepository, TraceNodeRepository,
};

use super::graph::{self, Direction};

// ---------------------------------------------------------------------------
// Ingest 命令（大系统 → 本子系统 的批量数据入口）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestNode {
    pub kind: TraceNodeKind,
    pub external_source: String,
    pub external_ref: String,
    #[serde(default)]
    pub module: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_obj")]
    pub attributes: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalKey {
    pub external_source: String,
    pub external_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestLink {
    pub source: ExternalKey,
    pub target: ExternalKey,
    pub link_type: LinkType,
    #[serde(default = "default_obj")]
    pub attributes: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestReport {
    pub nodes_upserted: usize,
    pub links_upserted: usize,
    pub unresolved_links: Vec<String>,
}

fn default_obj() -> Value {
    serde_json::json!({})
}

// ---------------------------------------------------------------------------
// 需求上传（结构化需求 JSON → 需求节点）
// ---------------------------------------------------------------------------

/// 一条结构化需求（研发侧系统/人员导出后交给测试侧）。对应 DOORS 里
/// 「从模型/文档筛出的需求对象」，但我们只约定节点模型，不约束来源形态。
///
/// 需求节点模型约定：
/// - `id` ↔ external_ref（回链来源的唯一键）；`title` = 标题（来源可自带"章节号+标题"）
/// - `description` = 内容/描述；`module` = 软分组
/// - `section` 等与文档章节相关的要素**不做强约束**——落入 attributes 透传
///   （章节格式各家不一，约束死了系统反而难用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadRequirement {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub attributes: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UploadReport {
    pub uploaded: usize,
    pub skipped: Vec<String>,
}

// ---------------------------------------------------------------------------
// 派生视图结果
// ---------------------------------------------------------------------------

/// 追溯矩阵行：一个软件需求 + 直接 trace 进它的 lower 项。
#[derive(Debug, Clone, Serialize)]
pub struct TraceMatrixRow {
    pub requirement: TraceNode,
    /// (来源节点, 链接类型)
    pub covering: Vec<(TraceNode, LinkType)>,
}

/// 覆盖分析结果
#[derive(Debug, Clone, Serialize)]
pub struct TraceMatrix {
    pub rows: Vec<TraceMatrixRow>,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct TraceService {
    nodes: Arc<dyn TraceNodeRepository>,
    links: Arc<dyn TraceLinkRepository>,
    baselines: Arc<dyn BaselineRepository>,
}

impl TraceService {
    pub fn new(
        nodes: Arc<dyn TraceNodeRepository>,
        links: Arc<dyn TraceLinkRepository>,
        baselines: Arc<dyn BaselineRepository>,
    ) -> Self {
        Self {
            nodes,
            links,
            baselines,
        }
    }

    // ---- 项目作用域辅助 ----

    /// 取某项目下的节点：跨项目 id 一律视作不存在（防误删/误改/跨项目连边）。
    pub async fn get_node(&self, project_id: Uuid, id: Uuid) -> Result<TraceNode, ServiceError> {
        let n = self
            .nodes
            .find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("trace_node_not_found".into()))?;
        if n.project_id != project_id {
            return Err(ServiceError::NotFound("trace_node_not_found".into()));
        }
        Ok(n)
    }

    // ---- 只读 ----

    pub async fn list_nodes(&self, project_id: Uuid) -> Result<Vec<TraceNode>, ServiceError> {
        Ok(self.nodes.list_all(project_id).await?)
    }

    pub async fn list_links(&self, project_id: Uuid) -> Result<Vec<TraceLink>, ServiceError> {
        Ok(self.links.list_all(project_id).await?)
    }

    // ---- 排序辅助 ----

    /// 某项目内各阶段当前最大 sort_key（-1 起步）→ 用 [`Self::take_next`] 取下一个号。
    /// 新节点总是落在所属阶段**末尾**；间隙无害（顺序只看相对大小，整理可重新紧凑）。
    async fn next_sort_map(&self, project_id: Uuid) -> Result<HashMap<TraceNodeKind, i32>, ServiceError> {
        let mut next: HashMap<TraceNodeKind, i32> = HashMap::new();
        for n in self.nodes.list_all(project_id).await? {
            let e = next.entry(n.kind).or_insert(-1);
            if n.sort_key > *e {
                *e = n.sort_key;
            }
        }
        Ok(next)
    }

    fn take_next(next: &mut HashMap<TraceNodeKind, i32>, kind: TraceNodeKind) -> i32 {
        let e = next.entry(kind).or_insert(-1);
        *e += 1;
        *e
    }

    /// 用户手动排序提交：逐条覆写 `(id, sort_key)`；未知/跨项目 id 静默忽略。
    pub async fn apply_order(
        &self,
        project_id: Uuid,
        orders: Vec<(Uuid, i32)>,
    ) -> Result<Vec<TraceNode>, ServiceError> {
        let allowed: HashSet<Uuid> = self
            .nodes
            .list_all(project_id)
            .await?
            .into_iter()
            .map(|n| n.id)
            .collect();
        let scoped: Vec<(Uuid, i32)> = orders
            .into_iter()
            .filter(|(id, _)| allowed.contains(id))
            .collect();
        self.nodes.set_sort_keys(&scoped).await?;
        Ok(self.nodes.list_all(project_id).await?)
    }

    /// 「整理」按钮：按需求锚定规则重排全图并落库（见 graph::requirement_anchored_order）。
    pub async fn arrange(&self, project_id: Uuid) -> Result<Vec<TraceNode>, ServiceError> {
        let nodes = self.nodes.list_all(project_id).await?;
        let edges = self.graph_edges(project_id).await?;
        let orders = graph::requirement_anchored_order(&nodes, &edges);
        self.nodes.set_sort_keys(&orders).await?;
        Ok(self.nodes.list_all(project_id).await?)
    }

    // ---- 单点 CRUD ----

    pub async fn create_node(
        &self,
        project_id: Uuid,
        kind: TraceNodeKind,
        external_source: String,
        external_ref: String,
        module: Option<String>,
        title: String,
        description: Option<String>,
    ) -> Result<TraceNode, ServiceError> {
        let source = external_source.trim();
        let reference = external_ref.trim();
        if source.is_empty() || reference.is_empty() {
            return Err(ServiceError::Validation(
                "external_source and external_ref must not be empty".into(),
            ));
        }
        if title.trim().is_empty() {
            return Err(ServiceError::Validation("title must not be empty".into()));
        }
        if self
            .nodes
            .find_by_external(project_id, source, reference)
            .await?
            .is_some()
        {
            return Err(ServiceError::Conflict(
                "trace node with this external key already exists".into(),
            ));
        }
        let mut next = self.next_sort_map(project_id).await?;
        let mut node = TraceNode::new(
            kind,
            source.into(),
            reference.into(),
            module.map(|m| m.trim().to_string()).filter(|m| !m.is_empty()),
            title.trim().to_string(),
            description,
        );
        node.project_id = project_id;
        node.sort_key = Self::take_next(&mut next, kind);
        self.nodes.upsert(&node).await?;
        Ok(node)
    }

    pub async fn update_node(
        &self,
        project_id: Uuid,
        id: Uuid,
        kind: Option<TraceNodeKind>,
        title: Option<String>,
        description: Option<String>,
        module: Option<String>,
        attributes: Option<Value>,
    ) -> Result<TraceNode, ServiceError> {
        let mut node = self.get_node(project_id, id).await?;
        if let Some(k) = kind {
            node.kind = k;
        }
        if let Some(t) = title {
            if t.trim().is_empty() {
                return Err(ServiceError::Validation("title must not be empty".into()));
            }
            node.title = t.trim().to_string();
        }
        if let Some(d) = description {
            node.description = Some(d.trim().to_string());
        }
        if let Some(m) = module {
            node.module = Some(m.trim().to_string());
        }
        if let Some(a) = attributes {
            node.attributes = a;
        }
        node.updated_at = chrono::Utc::now();
        self.nodes.upsert(&node).await?;
        Ok(node)
    }

    pub async fn delete_node(&self, project_id: Uuid, id: Uuid) -> Result<(), ServiceError> {
        self.get_node(project_id, id).await?;
        self.nodes.delete(id).await?;
        Ok(())
    }

    pub async fn create_link(
        &self,
        project_id: Uuid,
        source: Uuid,
        target: Uuid,
        link_type: LinkType,
    ) -> Result<TraceLink, ServiceError> {
        if source == target {
            return Err(ServiceError::Validation("source and target must differ".into()));
        }
        // 端点必须属于本项目，且天然同项目——跨项目连边一律按不存在处理。
        self.get_node(project_id, source).await?;
        self.get_node(project_id, target).await?;
        let mut link = TraceLink::new(source, target, link_type);
        link.project_id = project_id;
        self.links.upsert(&link).await?;
        Ok(link)
    }

    pub async fn delete_link(&self, project_id: Uuid, id: Uuid) -> Result<(), ServiceError> {
        let link = self
            .links
            .find_by_id(id)
            .await?
            .ok_or_else(|| ServiceError::NotFound("trace_link_not_found".into()))?;
        if link.project_id != project_id {
            return Err(ServiceError::NotFound("trace_link_not_found".into()));
        }
        self.links.delete(id).await?;
        Ok(())
    }

    // ---- 测试项编写 ----

    /// 创建一个测试项并直接 `derives_from` 到 ≥1 个需求节点（可关联多个）。
    ///
    /// - 需求必须存在、属于同一项目、且确实是 `SoftwareRequirement`；
    /// - 创建的 `test_item` 节点 external_source=manual、external_ref 服务端自动生成
    ///   （ASCII，防命令行/跨端非 ASCII 转义坑），落在测试项阶段末尾；
    /// - 关联是「测试项派生自需求」，链接方向测试项 → 需求（`DerivesFrom`）。
    ///
    /// 只在追踪图加节点/边，**不改文档模板**（模板是结构抽象，测试项是具体事件）。
    pub async fn create_test_item(
        &self,
        project_id: Uuid,
        title: String,
        module: Option<String>,
        description: Option<String>,
        requirement_ids: Vec<Uuid>,
    ) -> Result<TraceNode, ServiceError> {
        if title.trim().is_empty() {
            return Err(ServiceError::Validation("title must not be empty".into()));
        }
        if requirement_ids.is_empty() {
            return Err(ServiceError::Validation(
                "at least one requirement is required".into(),
            ));
        }
        // 去重（保序）并逐个校验：存在 + 同项目（get_node 已挡跨项目）+ 确为需求
        let mut seen = HashSet::new();
        for rid in requirement_ids {
            if !seen.insert(rid) {
                continue;
            }
            let n = self.get_node(project_id, rid).await?;
            if n.kind != TraceNodeKind::SoftwareRequirement {
                return Err(ServiceError::Validation(format!(
                    "node {} is not a software requirement",
                    n.external_ref
                )));
            }
        }
        let reference = Self::gen_manual_ref("ti");
        let node = self
            .create_node(
                project_id,
                TraceNodeKind::TestItem,
                "manual".into(),
                reference,
                module,
                title,
                description,
            )
            .await?;
        for rid in seen {
            self.create_link(project_id, node.id, rid, LinkType::DerivesFrom)
                .await?;
        }
        Ok(node)
    }

    /// 手工创建的节点自动生成 ASCII external_ref（`manual-<tag>-<nanos>`），避免调用方
    /// 传非 ASCII / 不唯一 ref 的转义与幂等坑。
    fn gen_manual_ref(tag: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("manual-{tag}-{nanos:x}")
    }

    // ---- 批量摄取 ----

    /// 幂等批量摄取。先 upsert 全部节点（项目内 external 键），再把链接端点解析成本地 id
    /// 后 upsert；端点缺失的链接计入 unresolved（跳过，不报错）——批量导入容忍脏引用。
    pub async fn ingest(
        &self,
        project_id: Uuid,
        nodes: Vec<IngestNode>,
        links: Vec<IngestLink>,
    ) -> Result<IngestReport, ServiceError> {
        let mut report = IngestReport::default();
        let mut next = self.next_sort_map(project_id).await?;

        for n in nodes {
            let mut node = TraceNode::new(
                n.kind,
                n.external_source.trim().to_string(),
                n.external_ref.trim().to_string(),
                n.module,
                n.title.trim().to_string(),
                n.description,
            );
            node.project_id = project_id;
            // 用 attributes 覆盖默认空对象
            node.attributes = n.attributes;
            // 新插入行落在本阶段末尾；已存在行 upsert 不覆盖 sort_key（保留手动顺序）
            node.sort_key = Self::take_next(&mut next, n.kind);
            self.nodes.upsert(&node).await?;
            report.nodes_upserted += 1;
        }

        for l in links {
            let src = self
                .nodes
                .find_by_external(project_id, &l.source.external_source, &l.source.external_ref)
                .await?;
            let tgt = self
                .nodes
                .find_by_external(project_id, &l.target.external_source, &l.target.external_ref)
                .await?;
            match (src, tgt) {
                (Some(s), Some(t)) => {
                    if s.id == t.id {
                        continue; // 自环由 ingest 忽略
                    }
                    let mut link = TraceLink::new(s.id, t.id, l.link_type);
                    link.project_id = project_id;
                    link.attributes = l.attributes;
                    self.links.upsert(&link).await?;
                    report.links_upserted += 1;
                }
                _ => report.unresolved_links.push(format!(
                    "{}:{} -> {}:{}",
                    l.source.external_source, l.source.external_ref,
                    l.target.external_source, l.target.external_ref
                )),
            }
        }
        Ok(report)
    }

    /// 结构化需求上传（研发侧导出 → 需求节点）。幂等：项目内 external_source +
    /// external_ref 冲突即覆盖；空 id 的行跳过并计入 skipped。所有需求节点归为
    /// `SoftwareRequirement`，标题缺省回退为 id。
    pub async fn upload_requirements(
        &self,
        project_id: Uuid,
        source: String,
        items: Vec<UploadRequirement>,
    ) -> Result<UploadReport, ServiceError> {
        let source = source.trim().to_string();
        if source.is_empty() {
            return Err(ServiceError::Validation(
                "source must not be empty".into(),
            ));
        }
        let mut report = UploadReport::default();
        let mut next = self.next_sort_map(project_id).await?;

        for it in items {
            let reference = it.id.trim();
            if reference.is_empty() {
                report.skipped.push("<empty id>".into());
                continue;
            }
            let title = it
                .title
                .unwrap_or_else(|| reference.to_string())
                .trim()
                .to_string();
            if title.is_empty() {
                report.skipped.push(reference.to_string());
                continue;
            }
            let module = it
                .module
                .map(|m| m.trim().to_string())
                .filter(|m| !m.is_empty());
            let mut attributes = it.attributes;
            if !attributes.is_object() {
                attributes = serde_json::json!({});
            }
            if let Some(s) = it.section {
                let s = s.trim().to_string();
                if !s.is_empty() {
                    attributes["source_section"] = Value::String(s);
                }
            }

            let mut node = TraceNode::new(
                TraceNodeKind::SoftwareRequirement,
                source.clone(),
                reference.to_string(),
                module,
                title,
                it.description,
            );
            node.project_id = project_id;
            node.attributes = attributes;
            node.sort_key = Self::take_next(&mut next, TraceNodeKind::SoftwareRequirement);
            self.nodes.upsert(&node).await?;
            report.uploaded += 1;
        }
        Ok(report)
    }

    // ---- 派生视图（读取期现算，不落库） ----

    async fn graph_edges(&self, project_id: Uuid) -> Result<Vec<(Uuid, Uuid, LinkType)>, ServiceError> {
        Ok(self
            .links
            .list_all(project_id)
            .await?
            .iter()
            .map(|l| (l.source_node_id, l.target_node_id, l.link_type))
            .collect())
    }

    /// 沿方向求闭包，返回可达节点（去重、按 id 排序保证确定可复现）。
    pub async fn reachable(
        &self,
        project_id: Uuid,
        roots: Vec<Uuid>,
        dir: Direction,
        link_types: Option<Vec<LinkType>>,
    ) -> Result<Vec<TraceNode>, ServiceError> {
        let edges = self.graph_edges(project_id).await?;
        let allowed: Option<&[LinkType]> = link_types.as_deref();
        let ids = graph::closure(&edges, &roots, dir, allowed);
        let mut by_id: HashMap<Uuid, TraceNode> = self
            .nodes
            .list_all(project_id)
            .await?
            .into_iter()
            .map(|n| (n.id, n))
            .collect();
        let mut out: Vec<TraceNode> = ids
            .into_iter()
            .filter_map(|id| by_id.remove(&id))
            .collect();
        out.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));
        Ok(out)
    }

    /// 覆盖分析：没有任何 derives_from/satisfies/verifies 灌入的软件需求。
    pub async fn coverage(&self, project_id: Uuid) -> Result<Vec<TraceNode>, ServiceError> {
        let nodes = self.nodes.list_all(project_id).await?;
        let edges = self.graph_edges(project_id).await?;
        let uncovered = graph::uncovered(&nodes, &edges);
        let set: HashSet<Uuid> = uncovered.into_iter().collect();
        let mut out: Vec<TraceNode> = nodes
            .into_iter()
            .filter(|n| set.contains(&n.id))
            .collect();
        out.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));
        Ok(out)
    }

    /// 追溯矩阵：以软件需求为行，列出直接 trace 进它的 lower 项。
    pub async fn matrix(&self, project_id: Uuid) -> Result<TraceMatrix, ServiceError> {
        let nodes = self.nodes.list_all(project_id).await?;
        let edges = self.graph_edges(project_id).await?;
        let allowed: &[LinkType] = &[LinkType::DerivesFrom, LinkType::Satisfies, LinkType::Verifies];
        let by_id: HashMap<Uuid, TraceNode> = nodes.into_iter().map(|n| (n.id, n)).collect();

        let mut rows = Vec::new();
        let mut req_ids: Vec<Uuid> = by_id
            .values()
            .filter(|n| n.kind == TraceNodeKind::SoftwareRequirement)
            .map(|n| n.id)
            .collect();
        req_ids.sort();

        for req in req_ids {
            let mut covering: Vec<(TraceNode, LinkType)> = graph::direct_sources(&edges, req, Some(allowed))
                .into_iter()
                .filter_map(|(sid, lt)| by_id.get(&sid).map(|n| (n.clone(), lt)))
                .collect();
            covering.sort_by(|a, b| (a.1.as_str(), a.0.created_at, a.0.id).cmp(&(b.1.as_str(), b.0.created_at, b.0.id)));
            if let Some(req_node) = by_id.get(&req) {
                rows.push(TraceMatrixRow {
                    requirement: req_node.clone(),
                    covering,
                });
            }
        }
        Ok(TraceMatrix { rows })
    }

    // ---- 基线 ----

    pub async fn create_baseline(
        &self,
        project_id: Uuid,
        name: String,
        reason: Option<String>,
    ) -> Result<TraceBaseline, ServiceError> {
        if name.trim().is_empty() {
            return Err(ServiceError::Validation("baseline name must not be empty".into()));
        }
        let nodes = self.nodes.list_all(project_id).await?;
        let links = self.links.list_all(project_id).await?;
        let snapshot = serde_json::json!({
            "nodes": nodes,
            "links": links,
        });
        let baseline = TraceBaseline {
            id: Uuid::new_v4(),
            project_id,
            name: name.trim().to_string(),
            reason,
            snapshot_json: snapshot,
            created_at: chrono::Utc::now(),
        };
        self.baselines.insert(&baseline).await?;
        Ok(baseline)
    }

    pub async fn list_baselines(&self, project_id: Uuid) -> Result<Vec<TraceBaseline>, ServiceError> {
        Ok(self.baselines.list_all(project_id).await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use async_trait::async_trait;

    /// 测试统一使用的项目 id（与 0007 默认示例项目常量一致，便于记忆）。
    fn pid() -> Uuid {
        Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0001)
    }

    struct MemNodes(Mutex<Vec<TraceNode>>);
    struct MemLinks(Mutex<Vec<TraceLink>>);
    struct MemBaselines(Mutex<Vec<TraceBaseline>>);

    #[async_trait]
    impl TraceNodeRepository for MemNodes {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<TraceNode>, crate::domain::RepositoryError> {
            Ok(self.0.lock().unwrap().iter().find(|n| n.id == id).cloned())
        }
        async fn find_by_external(&self, project_id: Uuid, s: &str, r: &str) -> Result<Option<TraceNode>, crate::domain::RepositoryError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .find(|n| n.project_id == project_id && n.external_source == s && n.external_ref == r)
                .cloned())
        }
        async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceNode>, crate::domain::RepositoryError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|n| n.project_id == project_id)
                .cloned()
                .collect())
        }
        async fn upsert(&self, node: &TraceNode) -> Result<Uuid, crate::domain::RepositoryError> {
            let mut g = self.0.lock().unwrap();
            if let Some(existing) = g.iter_mut().find(|n| {
                n.project_id == node.project_id
                    && n.external_source == node.external_source
                    && n.external_ref == node.external_ref
            }) {
                // 对齐 Postgres `ON CONFLICT`：项目内 external 键冲突时保留原 id/created_at，
                // 只更新其余字段——否则重摄取的节点 id 漂移，链接会重复（幂等性破坏）。
                existing.project_id = node.project_id;
                existing.kind = node.kind;
                existing.module = node.module.clone();
                existing.title = node.title.clone();
                existing.description = node.description.clone();
                existing.attributes = node.attributes.clone();
                existing.updated_at = node.updated_at;
                Ok(existing.id)
            } else {
                g.push(node.clone());
                Ok(node.id)
            }
        }
        async fn set_sort_keys(
            &self,
            orders: &[(Uuid, i32)],
        ) -> Result<(), crate::domain::RepositoryError> {
            let mut g = self.0.lock().unwrap();
            for (id, key) in orders {
                if let Some(n) = g.iter_mut().find(|n| n.id == *id) {
                    n.sort_key = *key;
                }
            }
            Ok(())
        }

        async fn delete(&self, id: Uuid) -> Result<(), crate::domain::RepositoryError> {
            self.0.lock().unwrap().retain(|n| n.id != id);
            Ok(())
        }
    }

    #[async_trait]
    impl TraceLinkRepository for MemLinks {
        async fn find_by_id(&self, id: Uuid) -> Result<Option<TraceLink>, crate::domain::RepositoryError> {
            Ok(self.0.lock().unwrap().iter().find(|l| l.id == id).cloned())
        }
        async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceLink>, crate::domain::RepositoryError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|l| l.project_id == project_id)
                .cloned()
                .collect())
        }
        async fn upsert(&self, link: &TraceLink) -> Result<Uuid, crate::domain::RepositoryError> {
            let mut g = self.0.lock().unwrap();
            if let Some(e) = g.iter_mut().find(|l| l.source_node_id == link.source_node_id && l.target_node_id == link.target_node_id && l.link_type == link.link_type) {
                // 同键冲突保留原 id/created_at，仅刷新属性（对齐 Postgres）。
                e.project_id = link.project_id;
                e.attributes = link.attributes.clone();
                e.updated_at = link.updated_at;
                Ok(e.id)
            } else {
                g.push(link.clone());
                Ok(link.id)
            }
        }
        async fn delete(&self, id: Uuid) -> Result<(), crate::domain::RepositoryError> {
            self.0.lock().unwrap().retain(|l| l.id != id);
            Ok(())
        }
    }

    #[async_trait]
    impl BaselineRepository for MemBaselines {
        async fn list_all(&self, project_id: Uuid) -> Result<Vec<TraceBaseline>, crate::domain::RepositoryError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|b| b.project_id == project_id)
                .cloned()
                .collect())
        }
        async fn insert(&self, b: &TraceBaseline) -> Result<(), crate::domain::RepositoryError> {
            self.0.lock().unwrap().push(b.clone());
            Ok(())
        }
    }

    fn svc() -> TraceService {
        TraceService::new(
            Arc::new(MemNodes(Mutex::new(vec![]))),
            Arc::new(MemLinks(Mutex::new(vec![]))),
            Arc::new(MemBaselines(Mutex::new(vec![]))),
        )
    }

    #[tokio::test]
    async fn ingest_upserts_idempotently_and_resolves_links() {
        let svc = svc();
        let nodes = vec![
            IngestNode { kind: TraceNodeKind::SoftwareRequirement, external_source: "el".into(), external_ref: "SR1".into(), module: None, title: "软需1".into(), description: None, attributes: json!({}) },
            IngestNode { kind: TraceNodeKind::TestItem, external_source: "el".into(), external_ref: "TI1".into(), module: None, title: "测项1".into(), description: None, attributes: json!({}) },
        ];
        let links = vec![IngestLink {
            source: ExternalKey { external_source: "el".into(), external_ref: "TI1".into() },
            target: ExternalKey { external_source: "el".into(), external_ref: "SR1".into() },
            link_type: LinkType::DerivesFrom,
            attributes: json!({}),
        }];
        let r1 = svc.ingest(pid(), nodes.clone(), links.clone()).await.unwrap();
        assert_eq!(r1.nodes_upserted, 2);
        assert_eq!(r1.links_upserted, 1);
        assert!(r1.unresolved_links.is_empty());

        // 幂等：再来一遍，节点仍是 2 个、链接仍是 1 条
        let r2 = svc.ingest(pid(), nodes, links).await.unwrap();
        assert_eq!(r2.nodes_upserted, 2);
        assert_eq!(r2.links_upserted, 1);
        assert_eq!(svc.list_nodes(pid()).await.unwrap().len(), 2);
        assert_eq!(svc.list_links(pid()).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn ingest_skips_unresolved_link() {
        let svc = svc();
        let nodes = vec![IngestNode { kind: TraceNodeKind::SoftwareRequirement, external_source: "el".into(), external_ref: "SR1".into(), module: None, title: "软需".into(), description: None, attributes: json!({}) }];
        let links = vec![IngestLink {
            source: ExternalKey { external_source: "el".into(), external_ref: "NOPE".into() },
            target: ExternalKey { external_source: "el".into(), external_ref: "SR1".into() },
            link_type: LinkType::DerivesFrom,
            attributes: json!({}),
        }];
        let r = svc.ingest(pid(), nodes, links).await.unwrap();
        assert_eq!(r.links_upserted, 0);
        assert_eq!(r.unresolved_links.len(), 1);
    }

    #[tokio::test]
    async fn upload_requirements_maps_to_sr_nodes_and_is_idempotent() {
        let svc = svc();
        let items = vec![
            UploadRequirement {
                id: "SR-3.2.1".into(),
                title: Some("3.2.1 支持账号密码登录".into()),
                description: Some("系统应支持账号密码登录。".into()),
                module: Some("登录".into()),
                section: Some("3.2.1".into()),
                attributes: json!({"owner": "rd"}), // 研发侧其它要素透传，不约束格式
            },
            UploadRequirement {
                id: "".into(), // 空 id → skipped
                title: None,
                description: None,
                module: None,
                section: None,
                attributes: json!({}),
            },
            UploadRequirement {
                id: "SR-3.2.2".into(), // 无标题 → 标题回退为 id
                title: None,
                description: None,
                module: None,
                section: None,
                attributes: json!({}),
            },
        ];
        let r1 = svc.upload_requirements(pid(), "reqtool".into(), items.clone()).await.unwrap();
        assert_eq!(r1.uploaded, 2);
        assert_eq!(r1.skipped.len(), 1);

        let nodes = svc.list_nodes(pid()).await.unwrap();
        assert_eq!(nodes.len(), 2);
        let sr = nodes.iter().find(|n| n.external_ref == "SR-3.2.1").unwrap();
        assert_eq!(sr.kind, TraceNodeKind::SoftwareRequirement);
        assert_eq!(sr.external_source, "reqtool");
        assert_eq!(sr.attributes["source_section"], "3.2.1");
        assert_eq!(sr.attributes["owner"], "rd"); // 透传要素仍在
        let fallback = nodes.iter().find(|n| n.external_ref == "SR-3.2.2").unwrap();
        assert_eq!(fallback.title, "SR-3.2.2");

        // 幂等重传不漂移 id
        let r2 = svc.upload_requirements(pid(), "reqtool".into(), items).await.unwrap();
        assert_eq!(r2.uploaded, 2);
        assert_eq!(svc.list_nodes(pid()).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn upload_requirements_rejects_empty_source() {
        let svc = svc();
        let err = svc.upload_requirements(pid(), "  ".into(), vec![]).await.unwrap_err();
        assert!(matches!(err, ServiceError::Validation(_)));
    }

    #[tokio::test]
    async fn create_node_duplicate_external_conflicts() {
        let svc = svc();
        svc.create_node(pid(), TraceNodeKind::SoftwareRequirement, "el".into(), "SR1".into(), None, "a".into(), None).await.unwrap();
        let err = svc.create_node(pid(), TraceNodeKind::TestItem, "el".into(), "SR1".into(), None, "b".into(), None).await.unwrap_err();
        assert!(matches!(err, ServiceError::Conflict(_)));
    }

    #[tokio::test]
    async fn create_link_guards_self_and_missing() {
        let svc = svc();
        let n = svc.create_node(pid(), TraceNodeKind::SoftwareRequirement, "el".into(), "SR1".into(), None, "a".into(), None).await.unwrap();
        let self_err = svc.create_link(pid(), n.id, n.id, LinkType::DerivesFrom).await.unwrap_err();
        assert!(matches!(self_err, ServiceError::Validation(_)));
        let other = Uuid::new_v4();
        let missing = svc.create_link(pid(), n.id, other, LinkType::DerivesFrom).await.unwrap_err();
        assert!(matches!(missing, ServiceError::NotFound(_)));
    }

    #[tokio::test]
    async fn arrange_anchors_by_requirement_and_apply_order_rewrites() {
        let svc = svc();
        async fn mk(
            svc: &TraceService,
            kind: TraceNodeKind,
            reference: &str,
        ) -> TraceNode {
            svc.create_node(pid(), kind, "s".into(), reference.into(), None, reference.into(), None)
                .await
                .unwrap()
        }
        let sr1 = mk(&svc, TraceNodeKind::SoftwareRequirement, "SR-1").await;
        let sr2 = mk(&svc, TraceNodeKind::SoftwareRequirement, "SR-2").await;
        let it1 = mk(&svc, TraceNodeKind::TestItem, "IT-1").await;
        let it2 = mk(&svc, TraceNodeKind::TestItem, "IT-2").await;
        svc.create_link(pid(), it1.id, sr1.id, LinkType::DerivesFrom).await.unwrap();
        svc.create_link(pid(), it2.id, sr2.id, LinkType::DerivesFrom).await.unwrap();

        svc.arrange(pid()).await.unwrap();
        // 需求 SR-1 rank0、SR-2 rank1；IT-1 锚 SR-1、IT-2 锚 SR-2 → 各阶段 0/1
        assert_eq!(svc.get_node(pid(), sr1.id).await.unwrap().sort_key, 0);
        assert_eq!(svc.get_node(pid(), sr2.id).await.unwrap().sort_key, 1);
        assert_eq!(svc.get_node(pid(), it1.id).await.unwrap().sort_key, 0);
        assert_eq!(svc.get_node(pid(), it2.id).await.unwrap().sort_key, 1);

        // 手动排序提交
        svc.apply_order(pid(), vec![(it1.id, 5), (it2.id, 3), (Uuid::new_v4(), 9)])
            .await
            .unwrap();
        assert_eq!(svc.get_node(pid(), it1.id).await.unwrap().sort_key, 5);
        assert_eq!(svc.get_node(pid(), it2.id).await.unwrap().sort_key, 3);

        // 整理回到锚定序
        svc.arrange(pid()).await.unwrap();
        assert_eq!(svc.get_node(pid(), it1.id).await.unwrap().sort_key, 0);
        assert_eq!(svc.get_node(pid(), it2.id).await.unwrap().sort_key, 1);
    }

    #[tokio::test]
    async fn matrix_and_coverage_report_uncovered_sr() {
        let svc = svc();
        svc.create_node(pid(), TraceNodeKind::SoftwareRequirement, "el".into(), "SR1".into(), None, "软需1".into(), None).await.unwrap();
        let sr2 = svc.create_node(pid(), TraceNodeKind::SoftwareRequirement, "el".into(), "SR2".into(), None, "软需2".into(), None).await.unwrap();
        let tr = svc.create_node(pid(), TraceNodeKind::TestItem, "el".into(), "TI1".into(), None, "测项1".into(), None).await.unwrap();
        svc.create_link(pid(), tr.id, sr2.id, LinkType::DerivesFrom).await.unwrap();

        let coverage = svc.coverage(pid()).await.unwrap();
        assert_eq!(coverage.len(), 1);
        assert_eq!(coverage[0].external_ref, "SR1");

        let m = svc.matrix(pid()).await.unwrap();
        assert_eq!(m.rows.len(), 2);
        let row_sr2 = m.rows.iter().find(|r| r.requirement.id == sr2.id).unwrap();
        assert_eq!(row_sr2.covering.len(), 1);
    }

    #[tokio::test]
    async fn create_test_item_links_derives_from_to_all_requirements() {
        let svc = svc();
        let r1 = svc
            .create_node(pid(), TraceNodeKind::SoftwareRequirement, "el".into(), "SR1".into(), None, "软需1".into(), None)
            .await
            .unwrap();
        let r2 = svc
            .create_node(pid(), TraceNodeKind::SoftwareRequirement, "el".into(), "SR2".into(), None, "软需2".into(), None)
            .await
            .unwrap();

        // 可关联多个需求；自动去重
        let ti = svc
            .create_test_item(pid(), "登录态校验".into(), Some("登录".into()), None, vec![r1.id, r2.id, r1.id])
            .await
            .unwrap();
        assert_eq!(ti.kind, TraceNodeKind::TestItem);
        assert_eq!(ti.external_source, "manual");
        assert!(ti.external_ref.starts_with("manual-ti-"));

        let links = svc.list_links(pid()).await.unwrap();
        assert_eq!(links.len(), 2);
        for l in &links {
            assert_eq!(l.source_node_id, ti.id);
            assert_eq!(l.link_type, LinkType::DerivesFrom);
        }
        let targets: std::collections::HashSet<Uuid> =
            links.iter().map(|l| l.target_node_id).collect();
        assert!(targets.contains(&r1.id) && targets.contains(&r2.id));
    }

    #[tokio::test]
    async fn create_test_item_rejects_empty_title_or_no_requirement() {
        let svc = svc();
        let err_title = svc.create_test_item(pid(), "  ".into(), None, None, vec![Uuid::new_v4()]).await.unwrap_err();
        assert!(matches!(err_title, ServiceError::Validation(_)));
        let err_empty = svc.create_test_item(pid(), "标题".into(), None, None, vec![]).await.unwrap_err();
        assert!(matches!(err_empty, ServiceError::Validation(_)));
    }

    #[tokio::test]
    async fn create_test_item_rejects_non_requirement_or_cross_project() {
        let svc = svc();
        let req = svc
            .create_node(pid(), TraceNodeKind::SoftwareRequirement, "el".into(), "SR1".into(), None, "软需1".into(), None)
            .await
            .unwrap();
        // 非需求 id（测试用例）→ Validation
        let case = svc
            .create_node(pid(), TraceNodeKind::TestCase, "el".into(), "TC1".into(), None, "用例1".into(), None)
            .await
            .unwrap();
        let err_non_req = svc.create_test_item(pid(), "标题".into(), None, None, vec![case.id]).await.unwrap_err();
        assert!(matches!(err_non_req, ServiceError::Validation(_)));
        // 不存在的 id → NotFound
        let err_missing = svc.create_test_item(pid(), "标题".into(), None, None, vec![Uuid::new_v4()]).await.unwrap_err();
        assert!(matches!(err_missing, ServiceError::NotFound(_)));
        // 其它项目的需求 → NotFound（跨项目按不存在处理）
        let other = Uuid::new_v4();
        let err_cross = svc.create_test_item(other, "标题".into(), None, None, vec![req.id]).await.unwrap_err();
        assert!(matches!(err_cross, ServiceError::NotFound(_)));
    }

    #[tokio::test]
    async fn baseline_is_scoped_to_project() {
        let svc = svc();
        svc.create_node(pid(), TraceNodeKind::SoftwareRequirement, "el".into(), "SR1".into(), None, "软需1".into(), None).await.unwrap();
        svc.create_baseline(pid(), "基线A".into(), None).await.unwrap();
        assert_eq!(svc.list_baselines(pid()).await.unwrap().len(), 1);
        // 另一项目没有基线
        let other = Uuid::new_v4();
        assert!(svc.list_baselines(other).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn trace_data_is_isolated_by_project() {
        let svc = svc();
        let a = pid();
        let b = Uuid::new_v4();

        svc.ingest(
            a,
            vec![IngestNode { kind: TraceNodeKind::SoftwareRequirement, external_source: "el".into(), external_ref: "SR1".into(), module: None, title: "需求A".into(), description: None, attributes: json!({}) }],
            vec![],
        )
        .await
        .unwrap();

        // 项目 B 起初为空
        assert!(svc.list_nodes(b).await.unwrap().is_empty());
        assert!(svc.list_links(b).await.unwrap().is_empty());

        // 项目 B 允许同 external 键——互不干扰（唯一键是「项目内」）
        let nb = svc
            .create_node(b, TraceNodeKind::SoftwareRequirement, "el".into(), "SR1".into(), None, "需求B".into(), None)
            .await
            .unwrap();
        assert_eq!(nb.project_id, b);

        let nodes_a = svc.list_nodes(a).await.unwrap();
        let nodes_b = svc.list_nodes(b).await.unwrap();
        assert_eq!(nodes_a.len(), 1);
        assert_eq!(nodes_b.len(), 1);
        assert_ne!(nodes_a[0].id, nodes_b[0].id);

        // A 的节点在 B 项目下不可见（跨项目按不存在处理）
        assert!(svc.get_node(a, nodes_a[0].id).await.is_ok());
        assert!(matches!(svc.get_node(b, nodes_a[0].id).await, Err(ServiceError::NotFound(_))));

        // 覆盖分析各自只看本项目的节点
        assert_eq!(svc.coverage(a).await.unwrap().len(), 1);
        assert_eq!(svc.coverage(b).await.unwrap().len(), 1);
    }

    use serde_json::json;
}
