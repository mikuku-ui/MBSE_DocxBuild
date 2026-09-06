//! 需求追踪图（Trace）实体。
//!
//! 定位：大 MBSE 系统灌入本子系统的「数据入口」的干净投影。本系统侧不重复
//! 大系统的需求模型，只保留类型化节点 + 类型化链接 + 轻量属性透传。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::project::default_project_id;

/// 需求测试**五阶段**（可扩展枚举）。追踪图的阶段/泳道由 kind 决定：
/// 越靠右越是顶层——最右 = 需求（被测顶层），最左 = 测试报告。整个流水里
/// **左侧阶段的一切都是为了覆盖/回答其右侧阶段**：
/// 测试项满足需求 → 测试用例由测试项分析 → 测试记录是用例执行结果 → 测试报告据记录回答需求。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceNodeKind {
    /// 测试报告（最左/最低层）：据测试记录回答需求是否真被满足
    TestReport,
    /// 测试记录：对测试用例执行后获得的结果
    TestRecord,
    /// 测试用例：根据测试项分析出的用例
    TestCase,
    /// 测试项：设计出来以满足需求
    TestItem,
    /// 需求（最右/顶层）：被测需求（来自研发侧系统或测试人员据文档录入）
    SoftwareRequirement,
}

impl TraceNodeKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "software_requirement" => Some(Self::SoftwareRequirement),
            "test_item" => Some(Self::TestItem),
            "test_case" => Some(Self::TestCase),
            "test_record" => Some(Self::TestRecord),
            "test_report" => Some(Self::TestReport),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SoftwareRequirement => "software_requirement",
            Self::TestItem => "test_item",
            Self::TestCase => "test_case",
            Self::TestRecord => "test_record",
            Self::TestReport => "test_report",
        }
    }
}

/// 追踪链接类型。方向约定钉死：`source` = 下级/派生/验证方（左侧阶段），`target` =
/// 上级/被满足方（右侧阶段，如 `测试项 --derives_from--> 需求`）。
///
/// DOORS 用一对 incoming/outgoing 角色表达，我们存储时只保留**单一规范方向**
/// 的一个链接类型，避免 `parent_of`/`child_of` 之类镜像重复记账。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    /// source 派生自 target（测试项 derived from 需求）
    DerivesFrom,
    /// source 验证 target（下级用例/测试项 verifies 上级）
    Verifies,
    /// source 满足 target（下级 satisfies 上级）
    Satisfies,
    /// source 细化 target
    Refines,
    /// source 是 target 的一部分（分解：source=子，target=父容器）
    PartOf,
}

impl LinkType {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "derives_from" => Some(Self::DerivesFrom),
            "verifies" => Some(Self::Verifies),
            "satisfies" => Some(Self::Satisfies),
            "refines" => Some(Self::Refines),
            "part_of" => Some(Self::PartOf),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DerivesFrom => "derives_from",
            Self::Verifies => "verifies",
            Self::Satisfies => "satisfies",
            Self::Refines => "refines",
            Self::PartOf => "part_of",
        }
    }
}

/// 追踪图节点 = 一个需求/用例对象。
///
/// - `external_source + external_ref` 是回链大系统的幂等键（项目内 UNIQUE，
///   `UNIQUE (project_id, external_source, external_ref)`，见 0007）；
/// - `module` 是软分组（大系统里的规格说明/集合），用于追溯矩阵分组；
/// - `sort_key` 是**用户自定义顺序**（阶段内 0..n-1 序数，非全局唯一）：画布纵向
///   排布与左栏列表都按它展示，拖拽/上移下移/「整理」时写入，落库保存；
/// - `attributes` 透传大系统持有的类型化属性投影，schema 由大系统拥有；
/// - `project_id` 归属项目。构造器给了默认项目占位，**业务写入前必须由服务层
///   覆写为实际项目**（`TraceService` 每处新建后都显式赋值）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceNode {
    pub id: Uuid,
    pub project_id: Uuid,
    pub kind: TraceNodeKind,
    pub external_source: String,
    pub external_ref: String,
    pub module: Option<String>,
    pub title: String,
    pub description: Option<String>,
    pub attributes: Value,
    pub sort_key: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TraceNode {
    pub fn new(
        kind: TraceNodeKind,
        external_source: String,
        external_ref: String,
        module: Option<String>,
        title: String,
        description: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id: default_project_id(), // 占位：写入前由服务层覆写为实际项目
            kind,
            external_source,
            external_ref,
            module,
            title,
            description,
            attributes: serde_json::json!({}),
            sort_key: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 追踪链接（一条有向、类型化、带单一规范方向的边）。
///
/// `project_id` 记录归属项目（两端节点必然同项目，由服务层保证）；构造器同节点
/// 一样给默认项目占位，业务写入前由服务层覆写。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceLink {
    pub id: Uuid,
    pub project_id: Uuid,
    pub source_node_id: Uuid,
    pub target_node_id: Uuid,
    pub link_type: LinkType,
    pub attributes: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TraceLink {
    pub fn new(source_node_id: Uuid, target_node_id: Uuid, link_type: LinkType) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id: default_project_id(), // 占位：写入前由服务层覆写为实际项目
            source_node_id,
            target_node_id,
            link_type,
            attributes: serde_json::json!({}),
            created_at: now,
            updated_at: now,
        }
    }
}

/// 追踪图基线快照（对应 DOORS Baseline / 上项目 Snapshot）。
///
/// `snapshot_json` 是某时刻图的冻结副本，**不做查询数据源**，只用于比较/回溯。
/// `project_id` 让基线随项目隔离（删除项目级联清空，见 0007）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceBaseline {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub reason: Option<String>,
    pub snapshot_json: Value,
    pub created_at: DateTime<Utc>,
}
