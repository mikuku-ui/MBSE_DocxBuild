//! 执行（Execution）实体。
//!
//! 文档模板编辑完成后，执行人员选一个执行阶段 → 从模板**派生**出一份执行清单
//! （task 列表化）。模板结构是派生的；执行人员**填的内容是数据，要落库**——两
//! 者严格分离（对齐知识库 T4/P3 的边界补充）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// 执行实例状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    Draft,
    InProgress,
    Completed,
}

impl InstanceStatus {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "draft" => Some(Self::Draft),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}

/// 执行清单单项状态（task 语义，贴近上项目状态模型但本域更简单）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    Pending,
    InProgress,
    Completed,
    Skipped,
}

impl ItemStatus {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "pending" => Some(Self::Pending),
            "in_progress" => Some(Self::InProgress),
            "completed" => Some(Self::Completed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Skipped => "skipped",
        }
    }
}

/// 一次执行实例 = 某项目为某个文档模板在某个执行阶段下的实例化。
///
/// `project_id` 归属项目（大模块执行依据 = 项目，见 0008 迁移）；模板本身是全局
/// 通用资源，执行清单则按项目隔离——切项目只看到本项目的执行实例。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionInstance {
    pub id: Uuid,
    pub project_id: Uuid,
    pub template_id: Uuid,
    /// 执行阶段（如：第一轮动态测试 / 文档审查 …）
    pub stage: Option<String>,
    pub status: InstanceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExecutionInstance {
    pub fn new(project_id: Uuid, template_id: Uuid, stage: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            project_id,
            template_id,
            stage,
            status: InstanceStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 执行清单单项：由模板节点派生，执行人员按它产出内容。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionItem {
    pub id: Uuid,
    pub instance_id: Uuid,
    pub template_node_id: Uuid,
    pub title: String,
    pub status: ItemStatus,
    /// 执行人员填写的内容（用户产物，落库）
    pub content: Value,
    pub sort_key: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExecutionItem {
    pub fn new(
        instance_id: Uuid,
        template_node_id: Uuid,
        title: String,
        sort_key: i32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            instance_id,
            template_node_id,
            title,
            status: ItemStatus::Pending,
            content: serde_json::json!({}),
            sort_key,
            created_at: now,
            updated_at: now,
        }
    }
}
