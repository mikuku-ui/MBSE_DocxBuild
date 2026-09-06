//! 项目（Project）实体：追踪图与执行清单按项目隔离。
//!
//! 每个项目一套独立的追踪数据（trace_nodes / trace_links / trace_baselines 均带
//! `project_id`，见 0007 迁移），执行清单亦按项目隔离（execution_instances 带
//! `project_id`，见 0008 迁移）；删除项目级联清空两者的数据。需求只经导入新增，
//! 测试项/用例/记录/报告由别处添加或导入，追踪页只展示；「测试项编写」页创建的
//! 测试项会作为 test_item 节点挂到追踪图。唯一仍属**全局共享**的资源是文档模板
//! （document 表无 project_id，不归任何项目管）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 默认示例项目 id：0007 迁移用它回填存量追踪数据，前端切换器里作为首个可用项目。
/// 与迁移 `0007_projects.sql` 里 INSERT 的常量保持一致。
pub fn default_project_id() -> Uuid {
    Uuid::from_u128(0x0000_0000_0000_4000_8000_0000_0000_0001)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    /// 绑定的体系（= 项目执行方式）。空 = 尚未选执行方式，项目数据待绑定后才有结构。
    pub scheme_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    pub fn with_scheme(name: String, scheme_id: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            scheme_id,
            created_at: now,
            updated_at: now,
        }
    }
}
