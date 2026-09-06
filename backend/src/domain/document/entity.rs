//! 文档模板（Document）实体。
//!
//! 与需求追踪图是**两张彼此独立的图**：模板图是「章节编排」，节点 = 文档结构
//! 元素，串起来 = 一篇文档模板。二者只在渲染期桥接（`trace_view` 节点派生读取
//! 追踪图 → 追溯表/章节），无 FK 强耦合（对齐 knowledge-base P10）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// 文档模板节点类型（对应 ConTeXt 组件分类的投影）。
///
/// `node_type → tex 组件骨架` 的映射常量在 `application/document` 维护，见
/// 知识库经验：文档模板最终要能生成 ConTeXt 组件（`components/{kind}/chNN-*`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateNodeType {
    /// 封面/修订页/目录
    FrontMatter,
    /// 章
    Chapter,
    /// 节
    Section,
    /// 段落
    Paragraph,
    /// 表格
    Table,
    /// 图片
    Figure,
    /// 附录
    Appendix,
    /// 追踪章节：渲染期从追踪图派生追溯表/清单（需求7的落点）
    TraceView,
}

impl TemplateNodeType {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "frontmatter" => Some(Self::FrontMatter),
            "chapter" => Some(Self::Chapter),
            "section" => Some(Self::Section),
            "paragraph" => Some(Self::Paragraph),
            "table" => Some(Self::Table),
            "figure" => Some(Self::Figure),
            "appendix" => Some(Self::Appendix),
            "trace_view" => Some(Self::TraceView),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FrontMatter => "frontmatter",
            Self::Chapter => "chapter",
            Self::Section => "section",
            Self::Paragraph => "paragraph",
            Self::Table => "table",
            Self::Figure => "figure",
            Self::Appendix => "appendix",
            Self::TraceView => "trace_view",
        }
    }
}

/// 文档模板（一篇文档的结构定义）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentTemplate {
    pub id: Uuid,
    pub name: String,
    /// 文档类别：大纲 / 报告 / …（对应 ConTeXt `kind`）
    pub kind: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DocumentTemplate {
    pub fn new(name: String, kind: String, description: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            kind,
            description,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 模板节点（扁平）。节点互挂关系走 `TemplateEdge`，不做 parent_id 硬编码列
/// （对齐 knowledge-base P1：别把图伪装成树）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateNode {
    pub id: Uuid,
    pub template_id: Uuid,
    pub node_type: TemplateNodeType,
    pub title: String,
    /// 该处需要什么内容；`TraceView` 时存追踪派生配置
    /// `{ roots, direction, link_types, group_by }`。
    pub content_spec: Value,
    /// tex 组件预留字段（暂不实现生成器，只占位钉死接口，避免日后推翻）。
    pub tex_component: Option<String>,
    /// 结构顺序键（模板节点串起来的有序依据之一，确定性排序用）。
    pub sort_key: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TemplateNode {
    pub fn new(
        template_id: Uuid,
        node_type: TemplateNodeType,
        title: String,
        content_spec: Value,
        sort_key: i32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            template_id,
            node_type,
            title,
            content_spec,
            tex_component: None,
            sort_key,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 模板节点互挂边（表结构顺序 / 包含关系）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEdge {
    pub id: Uuid,
    pub template_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub edge_type: String,
    pub created_at: DateTime<Utc>,
}
