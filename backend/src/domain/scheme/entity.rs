//! 体系（Scheme）实体：项目数据结构的「设计期定义」——可复用、可多套。
//!
//! 职责切分（对齐台账重构）：
//! - **体系**定义「这类项目需要什么数据」：标量字段 + 表（表结构 = 自定义列）。
//!   由体系/流程设计人员维护，项目按执行方式**绑定一套**（projects.scheme_id）。
//! - **项目数据**（`domain::projectdata`）存「这个项目填了什么」，值引用本模块的
//!   field/table id。
//!
//! 关键不变量：`scheme_fields` 以 `(scheme_id, doc_kind, field_key)` 为自然键、
//! `scheme_tables` 以 `(scheme_id, table_key)`、列以 `(table_id, column_key)`。
//! 编辑一套体系走**按自然键 upsert（保 id）**：只改定义不换 id → 已填的项目值不丢；
//! 真正被删掉的字段/表级联清其项目值（FK ON DELETE CASCADE，见 0010 迁移）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 体系（元数据行）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scheme {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 标量字段定义。`doc_kind`：'project' = 项目共享；其余（大纲/说明/…）= 该文档类专属。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeField {
    pub id: Uuid,
    pub scheme_id: Uuid,
    pub doc_kind: String,
    pub field_key: String,
    pub label: String,
    pub sort_key: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 表定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeTable {
    pub id: Uuid,
    pub scheme_id: Uuid,
    pub table_key: String,
    pub label: String,
    pub sort_key: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 自定义列（一行 = 一列）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeTableColumn {
    pub id: Uuid,
    pub table_id: Uuid,
    pub column_key: String,
    pub label: String,
    pub sort_key: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 一张表 + 其列（GET 完整定义的嵌套单元）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeTableDef {
    pub table: SchemeTable,
    pub columns: Vec<SchemeTableColumn>,
}

/// 体系的完整定义（含 id）：编辑页 / 装配取结构用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeFull {
    pub scheme: Scheme,
    pub fields: Vec<SchemeField>,
    pub tables: Vec<SchemeTableDef>,
}

// ---------------------------------------------------------------------------
// 写入形态（POST/PUT 请求体 / 仓储入参）：只含业务列，无 id/时间戳。
// 仓储按自然键 upsert，服务端补 id 并保已有 id。
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemeSpec {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub fields: Vec<SchemeFieldSpec>,
    #[serde(default)]
    pub tables: Vec<SchemeTableSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeFieldSpec {
    #[serde(default)]
    pub doc_kind: String,
    pub field_key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub sort_key: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeTableSpec {
    pub table_key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub sort_key: i32,
    #[serde(default)]
    pub columns: Vec<SchemeColumnSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeColumnSpec {
    pub column_key: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub sort_key: i32,
}
