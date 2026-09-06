//! HTTP 请求体 DTO。serde 反序列化；具体服务在 application 层。

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::application::trace::service::{IngestLink, IngestNode, UploadRequirement};
use crate::domain::document::TemplateNodeType;
use crate::domain::execution::{InstanceStatus, ItemStatus};
use crate::domain::trace::{LinkType, TraceNodeKind};

// ---- project ----

#[derive(Debug, Deserialize)]
pub struct CreateProject {
    pub name: String,
    /// 建项目即绑定的体系（= 执行方式）；可空 = 暂不绑定，到项目管理页再选。
    #[serde(default)]
    pub scheme_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct RenameProject {
    pub name: String,
}

/// 绑定/解绑体系。`scheme_id: null` = 解绑（清空该项目已填数据）。
#[derive(Debug, Deserialize)]
pub struct BindScheme {
    pub scheme_id: Option<Uuid>,
}

/// 追踪域各接口的项目作用域（query param `?project_id=<uuid>`）。
#[derive(Debug, Deserialize)]
pub struct ProjectScope {
    pub project_id: Uuid,
}

// ---- trace ----

#[derive(Debug, Deserialize)]
pub struct CreateTraceNode {
    pub kind: TraceNodeKind,
    pub external_source: String,
    pub external_ref: String,
    #[serde(default)]
    pub module: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTraceNode {
    /// 变更阶段（kind）时移动节点所属泳道
    #[serde(default)]
    pub kind: Option<TraceNodeKind>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub attributes: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTraceLink {
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub link_type: LinkType,
}

#[derive(Debug, Deserialize)]
pub struct ReorderItem {
    pub id: Uuid,
    pub sort_key: i32,
}

#[derive(Debug, Deserialize)]
pub struct ReorderRequest {
    pub orders: Vec<ReorderItem>,
}

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    #[serde(default)]
    pub nodes: Vec<IngestNode>,
    #[serde(default)]
    pub links: Vec<IngestLink>,
}

#[derive(Debug, Deserialize)]
pub struct UploadRequirementsRequest {
    pub source: String,
    #[serde(default)]
    pub requirements: Vec<UploadRequirement>,
}

#[derive(Debug, Deserialize)]
pub struct ReachParams {
    /// 溯源所在项目（追踪数据按项目隔离）
    pub project_id: Uuid,
    /// up | down；缺省 down（= 影响面：哪些下级 trace 到它）
    #[serde(default)]
    pub direction: Option<String>,
    /// 逗号分隔 link_type 过滤；缺省 = 全部
    #[serde(default)]
    pub link_types: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBaseline {
    pub name: String,
    #[serde(default)]
    pub reason: Option<String>,
}

// ---- document ----

#[derive(Debug, Deserialize)]
pub struct CreateTemplate {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplate {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTemplateNode {
    pub node_type: TemplateNodeType,
    pub title: String,
    #[serde(default)]
    pub content_spec: Option<Value>,
    #[serde(default)]
    pub sort_key: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateNode {
    #[serde(default)]
    pub node_type: Option<TemplateNodeType>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub content_spec: Option<Value>,
    #[serde(default)]
    pub sort_key: Option<i32>,
    #[serde(default)]
    pub tex_component: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTemplateEdge {
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    #[serde(default)]
    pub edge_type: Option<String>,
}

// ---- trace：测试项编写 ----

#[derive(Debug, Deserialize)]
pub struct CreateTestItem {
    pub title: String,
    #[serde(default)]
    pub module: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// 必选（≥1）的关联需求节点 id——测试项 derives_from 它们（可关联多个）
    pub requirement_ids: Vec<Uuid>,
}

// ---- execution ----

#[derive(Debug, Deserialize)]
pub struct CreateExecution {
    /// 执行清单归属项目（大模块执行依据 = 项目）
    pub project_id: Uuid,
    pub template_id: Uuid,
    #[serde(default)]
    pub stage: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExecution {
    #[serde(default)]
    pub status: Option<InstanceStatus>,
    #[serde(default)]
    pub stage: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExecutionItem {
    #[serde(default)]
    pub status: Option<ItemStatus>,
    #[serde(default)]
    pub content: Option<Value>,
}
