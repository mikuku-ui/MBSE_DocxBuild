//! 项目数据（值）：项目级一份、全项目文档共享，装配/项目页/执行清单共用的取数源。
//!
//! 结构来自项目绑定的体系（字段 + 自定义列表），本模块只存值。实体 `ProjectDataView`
//! 由体系结构 × 值合并而成——既是 GET 响应形态，也是装配引擎的取数源。

pub mod entity;
pub mod repository;

pub use entity::{
    DataColumn, DataField, DataRow, DataTable, ProjectDataView, ProjectDataWrite, TableRowsWrite,
};
pub use repository::ProjectDataRepository;
