//! 项目数据（值）：**项目级一份**，全项目文档共享；装配/项目页/执行清单共读共写。
//!
//! 与旧台账的分界：旧台账把「要哪些数据（schema）」和「填了什么（data）」揉在一起。
//! 现在 schema 上移到体系（`domain::scheme`），本项目只存值：
//! - `project_field_values` 引用 `scheme_fields`（标量字段值，field 级别一份）；
//! - `project_table_rows` 引用 `scheme_tables`（表行，cells jsonb 以 column_key 为键）。
//!
//! 值按**字段/表行结构**泛型存放，可容纳体系里任意自定义列；结构来自绑定体系。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 一个标量字段值（含体系给的结构信息，视图自描述、可直接渲染）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataField {
    pub id: Uuid,
    pub doc_kind: String,
    pub field_key: String,
    pub label: String,
    pub value: String,
}

/// 一列（渲染列头用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataColumn {
    pub id: Uuid,
    pub column_key: String,
    pub label: String,
}

/// 一行（cells 以 column_key 为键；缺键 = 空单元格）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRow {
    pub row_index: i32,
    pub cells: HashMap<String, String>,
}

/// 一张表：列（结构）+ 行（值）。表结构来自绑定体系。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataTable {
    pub id: Uuid,
    pub table_key: String,
    pub label: String,
    pub columns: Vec<DataColumn>,
    pub rows: Vec<DataRow>,
}

/// **项目数据视图**：由「项目绑定体系的结构 × 本项目填的值」合并而来。
/// GET /api/projects/{id}/data 的响应形态，也是装配引擎的取数源。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDataView {
    pub project_id: Uuid,
    pub scheme_id: Option<Uuid>,
    pub fields: Vec<DataField>,
    pub tables: Vec<DataTable>,
}

// ---------------------------------------------------------------------------
// 写入形态（PUT 请求体）：引用体系 field/table 的 id，值可空（空 = 清该项）。
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProjectDataWrite {
    #[serde(default)]
    pub fields: Vec<FieldValueWrite>,
    #[serde(default)]
    pub tables: Vec<TableRowsWrite>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldValueWrite {
    pub field_id: Uuid,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TableRowsWrite {
    pub table_id: Uuid,
    #[serde(default)]
    pub rows: Vec<DataRow>,
}
