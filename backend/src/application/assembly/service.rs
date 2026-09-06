//! 装配服务：模板（视图脚本）→ 项目数据 → 有序「正文块」序列。
//!
//! 输入模板节点按其 `content_spec` 声明逐段求值：
//!   文本套话展开 `{field}` 占位符；字段查「项目数据」标量；表格查「项目数据」表
//!   （结构列 + 行，列头来自项目绑定体系的 scheme_table_columns）；追溯切片现算
//!   追踪图（matrix / test_items）；人工槽取执行清单对应项内容（无实例或未填 →
//!   回落 placeholder）。整篇文档不落库业务数据，只出投影。

use std::collections::HashMap;
use std::sync::Arc;

use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::application::assembly::content::{BlockKind, ContentSpec};
use crate::application::document::DocumentService;
use crate::application::execution::ExecutionService;
use crate::application::projectdata::ProjectDataService;
use crate::application::trace::service::{TraceMatrix, TraceService};
use crate::application::ServiceError;
use crate::domain::document::TemplateNodeType;
use crate::domain::projectdata::{DataTable, ProjectDataView};
use crate::domain::trace::{TraceNode, TraceNodeKind};

// ---------------------------------------------------------------------------
// 装配结果
// ---------------------------------------------------------------------------

/// 一篇文档的装配结果：模板元信息 + 文档序正文块序列。
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedDocument {
    pub template_id: Uuid,
    pub template_name: String,
    pub template_kind: String,
    pub project_id: Uuid,
    pub instance_id: Option<Uuid>,
    pub blocks: Vec<ResolvedBlock>,
}

/// 一个正文块（对应一个模板节点在文档序上的投影）。
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedBlock {
    pub node_id: Uuid,
    pub node_type: TemplateNodeType,
    pub title: String,
    /// 文档层级（contains 树深度；根 = 0）。纯结构节点仍照常出块，由渲染层做标题。
    pub depth: usize,
    pub kind: BlockKind,
    /// 按 kind 有结构的值（见下）：
    /// - text/field → { text }
    /// - table       → { columns: [{key,label}], rows: [{<column_key>:…}] }
    /// - trace       → { view, rows|items }
    /// - slot        → { placeholder, role, filled, text }
    /// - empty       → {}
    pub value: Value,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct AssemblyService {
    docs: Arc<DocumentService>,
    trace: Arc<TraceService>,
    execution: Arc<ExecutionService>,
    project_data: Arc<ProjectDataService>,
}

impl AssemblyService {
    pub fn new(
        docs: Arc<DocumentService>,
        trace: Arc<TraceService>,
        execution: Arc<ExecutionService>,
        project_data: Arc<ProjectDataService>,
    ) -> Self {
        Self {
            docs,
            trace,
            execution,
            project_data,
        }
    }

    /// 装配：模板文档序节点逐个按 content_spec 求值 → ResolvedDocument。
    /// `instance_id`：人工槽取值所用执行实例（可空 = 槽全回落占位）。
    pub async fn assemble(
        &self,
        template_id: Uuid,
        project_id: Uuid,
        instance_id: Option<Uuid>,
    ) -> Result<ResolvedDocument, ServiceError> {
        let (tpl, nodes, edges) = self.docs.get_template_full(template_id).await?;
        let ordered =
            crate::application::document::DocumentService::ordered_nodes(nodes, &edges);

        // 项目数据视图 = 「项目绑定体系的结构 × 已填值」；项目未绑体系 → 空视图。
        let view = self.project_data.view(project_id).await?;
        let slot_items = self.slot_items(project_id, instance_id).await?;

        // 预取追溯数据（有 trace 节点才真的查；惰性防项目无数据时白跑一次）。
        let needs_trace = ordered
            .iter()
            .any(|n| BlockKind::from_spec(&n.content_spec) == BlockKind::Trace);

        let mut blocks = Vec::with_capacity(ordered.len());
        let mut depth_of: HashMap<Uuid, usize> = HashMap::with_capacity(ordered.len());
        let mut parent_of: HashMap<Uuid, Uuid> = HashMap::new();
        for e in &edges {
            if e.edge_type == "contains" {
                parent_of.insert(e.target_node_id, e.source_node_id);
            }
        }

        for node in &ordered {
            let depth = parent_of
                .get(&node.id)
                .and_then(|p| depth_of.get(p).copied())
                .map(|d| d + 1)
                .unwrap_or(0);
            depth_of.insert(node.id, depth);

            let kind = BlockKind::from_spec(&node.content_spec);
            let spec = ContentSpec::parse(&node.content_spec);
            let value = match (&kind, spec.as_ref()) {
                (BlockKind::Text, Some(ContentSpec::Text { text })) => {
                    let text = self.expand_fields(text, &view, tpl.kind.as_str(), None);
                    json!({ "text": text })
                }
                (BlockKind::Field, Some(ContentSpec::Field { field, doc_kind })) => {
                    let text = self.expand_fields(
                        &format!("{{{field}}}"),
                        &view,
                        tpl.kind.as_str(),
                        doc_kind.as_deref(),
                    );
                    json!({ "text": text })
                }
                (BlockKind::Table, Some(ContentSpec::Table { source, party_kind, category })) => {
                    Self::table_value(&view, source, party_kind.as_deref(), category.as_deref())
                }
                (BlockKind::Trace, Some(ContentSpec::Trace { view })) => {
                    if !needs_trace {
                        json!({ "view": view, "rows": [] })
                    } else {
                        json!(self.trace_value(project_id, view).await?)
                    }
                }
                (BlockKind::Slot, Some(ContentSpec::Slot { placeholder, role })) => {
                    let filled = slot_items.get(&node.id);
                    match filled {
                        Some(text) => json!({
                            "placeholder": placeholder,
                            "role": role,
                            "filled": true,
                            "text": text,
                        }),
                        None => json!({
                            "placeholder": placeholder,
                            "role": role,
                            "filled": false,
                            "text": placeholder,
                        }),
                    }
                }
                _ => json!({}),
            };

            blocks.push(ResolvedBlock {
                node_id: node.id,
                node_type: node.node_type,
                title: node.title.clone(),
                depth,
                kind,
                value,
            });
        }

        Ok(ResolvedDocument {
            template_id: tpl.id,
            template_name: tpl.name,
            template_kind: tpl.kind,
            project_id,
            instance_id,
            blocks,
        })
    }

    // ---- 项目数据字段/正文插值 ----

    /// 项目数据标量字段查找：作用域依次为 [显式 doc_kind] → [当前文档类] → [project]。
    /// `field_key` → 值；找不到 → None（占位保留）。
    fn view_field(
        field_key: &str,
        doc_kind: Option<&str>,
        doc_scope: &str,
        view: &ProjectDataView,
    ) -> Option<String> {
        let find = |scope: &str| -> Option<String> {
            view.fields
                .iter()
                .find(|f| f.doc_kind == scope && f.field_key == field_key)
                .map(|f| f.value.clone())
        };
        let mut tried: Vec<String> = Vec::new();
        if let Some(dk) = doc_kind {
            if !dk.is_empty() && dk != "project" {
                if let Some(v) = find(dk) {
                    return Some(v);
                }
                tried.push(dk.to_string());
            }
        }
        if doc_scope != "project" && !tried.iter().any(|t| t == doc_scope) {
            if let Some(v) = find(doc_scope) {
                return Some(v);
            }
        }
        find("project")
    }

    /// 展开 `{field_key}` 占位符（text / field 共用）。取不到 → 保留原文占位。
    fn expand_fields(
        &self,
        text: &str,
        view: &ProjectDataView,
        doc_scope: &str,
        doc_kind: Option<&str>,
    ) -> String {
        let lookup = |key: &str| Self::view_field(key, doc_kind, doc_scope, view);
        ContentSpec::interpolate(text, &lookup)
    }

    // ---- 项目数据表 ----

    /// 表块：按体系 table_key 解析 → `{ columns:[{key,label}], rows:[{<column_key>:…}] }`。
    /// 可选过滤（party_kind/category）按表内同名列单元格精确匹配；体系无该表 →
    /// 空表（作者写 source 时即引用体系表键）。
    fn table_value(
        view: &ProjectDataView,
        source: &str,
        party_kind: Option<&str>,
        category: Option<&str>,
    ) -> Value {
        let Some(table) = view.tables.iter().find(|t| t.table_key == source) else {
            return json!({ "columns": [], "rows": [] });
        };
        Self::table_projection(table, party_kind, category)
    }

    fn table_projection(
        table: &DataTable,
        party_kind: Option<&str>,
        category: Option<&str>,
    ) -> Value {
        let columns: Vec<Value> = table
            .columns
            .iter()
            .map(|c| json!({ "key": c.column_key, "label": c.label }))
            .collect();

        let filter = |cells: &HashMap<String, String>| -> bool {
            let mut ok = true;
            if let Some(pk) = party_kind {
                if !pk.is_empty() && cells.get("party_kind").map(String::as_str) != Some(pk) {
                    ok = false;
                }
            }
            if ok {
                if let Some(cat) = category {
                    if !cat.is_empty() && cells.get("category").map(String::as_str) != Some(cat) {
                        ok = false;
                    }
                }
            }
            ok
        };

        let mut rows_sorted = table.rows.clone();
        rows_sorted.sort_by_key(|r| r.row_index);
        let rows: Vec<Value> = rows_sorted
            .iter()
            .filter(|r| filter(&r.cells))
            .map(|r| {
                let mut obj = serde_json::Map::new();
                for c in &table.columns {
                    let v = r.cells.get(&c.column_key).cloned().unwrap_or_default();
                    obj.insert(c.column_key.clone(), Value::String(v));
                }
                Value::Object(obj)
            })
            .collect();

        json!({ "columns": columns, "rows": rows })
    }

    // ---- 追溯切片 ----

    async fn trace_value(&self, project_id: Uuid, view: &str) -> Result<Value, ServiceError> {
        match view {
            "matrix" => {
                let m = self.trace.matrix(project_id).await?;
                Ok(json!({ "view": "matrix", "rows": Self::matrix_rows(&m) }))
            }
            "test_items" => {
                let all = self.trace.list_nodes(project_id).await?;
                let items = Self::test_item_rows(&all);
                Ok(json!({ "view": "test_items", "items": items }))
            }
            other => Err(ServiceError::Validation(format!(
                "unknown trace view: {other}"
            ))),
        }
    }

    /// 追溯矩阵 → 行列化投影（供预览/导出：需求一行，覆盖项并排）。
    fn matrix_rows(m: &TraceMatrix) -> Vec<Value> {
        m.rows
            .iter()
            .map(|r| {
                json!({
                    "requirement": {
                        "id": r.requirement.id,
                        "external_ref": r.requirement.external_ref,
                        "title": r.requirement.title,
                        "module": r.requirement.module,
                    },
                    "covering": r.covering.iter().map(|(n, lt)| json!({
                        "id": n.id,
                        "external_ref": n.external_ref,
                        "title": n.title,
                        "link_type": lt.as_str(),
                    })).collect::<Vec<_>>(),
                })
            })
            .collect()
    }

    /// 测试项切片：只取 test_item 阶段节点，稳定排序（文档里按序罗列）。
    fn test_item_rows(nodes: &[TraceNode]) -> Vec<Value> {
        let mut items: Vec<&TraceNode> = nodes
            .iter()
            .filter(|n| n.kind == TraceNodeKind::TestItem)
            .collect();
        items.sort_by(|a, b| (a.created_at, a.id).cmp(&(b.created_at, b.id)));
        items
            .into_iter()
            .map(|n| {
                json!({
                    "id": n.id,
                    "external_ref": n.external_ref,
                    "title": n.title,
                    "module": n.module,
                })
            })
            .collect()
    }

    // ---- 人工槽 ----

    /// 执行清单里该模板节点当前填的内容（key = template_node_id）。
    /// 无实例 / 实例不属于该项目 → 空 map（全部回落占位）。
    async fn slot_items(
        &self,
        project_id: Uuid,
        instance_id: Option<Uuid>,
    ) -> Result<HashMap<Uuid, String>, ServiceError> {
        let mut out = HashMap::new();
        let Some(iid) = instance_id else { return Ok(out) };
        let instance = self.execution.get_instance(iid).await?;
        if instance.project_id != project_id {
            return Ok(out); // 选错项目实例：不泄漏，按未填处理
        }
        for item in self.execution.list_items(iid).await? {
            if let Some(text) = item.content.get("text").and_then(Value::as_str) {
                out.insert(item.template_node_id, text.to_string());
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// 纯函数测试：插值/表投影/追溯切片（无 DB）。
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::domain::projectdata::{DataColumn, DataField, DataRow};
    use crate::domain::trace::LinkType;

    fn field(doc_kind: &str, key: &str, label: &str, value: &str) -> DataField {
        DataField {
            id: Uuid::new_v4(),
            doc_kind: doc_kind.into(),
            field_key: key.into(),
            label: label.into(),
            value: value.into(),
        }
    }

    fn view_with(fields: Vec<DataField>, tables: Vec<DataTable>) -> ProjectDataView {
        ProjectDataView {
            project_id: Uuid::nil(),
            scheme_id: Some(Uuid::nil()),
            fields,
            tables,
        }
    }

    fn table(table_key: &str, rows: Vec<DataRow>) -> DataTable {
        DataTable {
            id: Uuid::new_v4(),
            table_key: table_key.into(),
            label: table_key.into(),
            columns: vec![
                DataColumn {
                    id: Uuid::new_v4(),
                    column_key: "party_kind".into(),
                    label: "角色".into(),
                },
                DataColumn {
                    id: Uuid::new_v4(),
                    column_key: "name".into(),
                    label: "单位名称".into(),
                },
            ],
            rows,
        }
    }

    fn tnode_at(
        id: &str,
        kind: TraceNodeKind,
        title: &str,
        created_at: chrono::DateTime<Utc>,
    ) -> TraceNode {
        TraceNode {
            id: Uuid::parse_str(id).unwrap(),
            project_id: Uuid::nil(),
            kind,
            external_source: "test".into(),
            external_ref: id.into(),
            module: Some("模块A".into()),
            title: title.into(),
            description: None,
            attributes: json!({}),
            sort_key: 0,
            created_at,
            updated_at: created_at,
        }
    }

    fn tnode(id: &str, kind: TraceNodeKind, title: &str) -> TraceNode {
        tnode_at(id, kind, title, Utc::now())
    }

    #[test]
    fn matrix_rows_projects_requirement_and_covering() {
        let req = tnode(
            "10000000-0000-0000-0000-000000000001",
            TraceNodeKind::SoftwareRequirement,
            "软件需求1",
        );
        let item = tnode(
            "20000000-0000-0000-0000-000000000002",
            TraceNodeKind::TestItem,
            "测试项1",
        );
        let matrix = TraceMatrix {
            rows: vec![crate::application::trace::service::TraceMatrixRow {
                requirement: req.clone(),
                covering: vec![(item.clone(), LinkType::DerivesFrom)],
            }],
        };
        let rows = AssemblyService::matrix_rows(&matrix);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["requirement"]["title"], "软件需求1");
        assert_eq!(rows[0]["covering"][0]["title"], "测试项1");
        assert_eq!(rows[0]["covering"][0]["link_type"], "derives_from");
    }

    #[test]
    fn test_item_rows_filters_and_sorts() {
        // 需求节点不应入选；测试项按 (created_at, id) 稳定序。
        let t0 = Utc::now();
        let nodes = vec![
            tnode(
                "10000000-0000-0000-0000-00000000000a",
                TraceNodeKind::SoftwareRequirement,
                "需求",
            ),
            tnode_at(
                "10000000-0000-0000-0000-00000000000b",
                TraceNodeKind::TestItem,
                "测试项B",
                t0 + chrono::Duration::seconds(5),
            ),
            tnode_at(
                "10000000-0000-0000-0000-00000000000c",
                TraceNodeKind::TestItem,
                "测试项A",
                t0,
            ),
        ];
        let rows = AssemblyService::test_item_rows(&nodes);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["title"], "测试项A");
        assert_eq!(rows[1]["title"], "测试项B");
    }

    #[test]
    fn view_field_scope_fallbacks() {
        let view = view_with(
            vec![
                field("project", "软件名称", "软件名称", "XXX系统"),
                field("大纲", "文档标识", "文档标识", "RT-FTC"),
            ],
            vec![],
        );
        // project 作用域字段
        assert_eq!(
            AssemblyService::view_field("软件名称", None, "大纲", &view),
            Some("XXX系统".into())
        );
        // 当前文档类作用域字段优先于 project
        assert_eq!(
            AssemblyService::view_field("文档标识", None, "大纲", &view),
            Some("RT-FTC".into())
        );
        // 显式 doc_kind 优先
        assert_eq!(
            AssemblyService::view_field("文档标识", Some("大纲"), "project", &view),
            Some("RT-FTC".into())
        );
        // 未知 key → None（占位保留）
        assert_eq!(AssemblyService::view_field("没有的", None, "大纲", &view), None);
    }

    #[test]
    fn table_projection_emits_columns_and_filtered_rows() {
        let mut cells = HashMap::new();
        cells.insert("party_kind".into(), "委托方".to_string());
        cells.insert("name".into(), "甲方".to_string());
        let mut cells2 = HashMap::new();
        cells2.insert("party_kind".into(), "测评机构".to_string());
        cells2.insert("name".into(), "乙方".to_string());
        let t = table(
            "parties",
            vec![
                DataRow { row_index: 0, cells },
                DataRow { row_index: 1, cells: cells2 },
            ],
        );

        // 无过滤：全出，行只含表内列键。
        let v = AssemblyService::table_projection(&t, None, None);
        assert_eq!(v["columns"].as_array().unwrap().len(), 2);
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "甲方");
        // 过滤 party_kind=测评机构 → 只剩第二行。
        let v2 = AssemblyService::table_projection(&t, Some("测评机构"), None);
        let rows2 = v2["rows"].as_array().unwrap();
        assert_eq!(rows2.len(), 1);
        assert_eq!(rows2[0]["party_kind"], "测评机构");
        // 未知表键 → 空 columns/rows。
        let empty = AssemblyService::table_value(
            &view_with(vec![], vec![t]),
            "nope",
            None,
            None,
        );
        assert_eq!(empty["columns"].as_array().unwrap().len(), 0);
        assert_eq!(empty["rows"].as_array().unwrap().len(), 0);
    }
}
