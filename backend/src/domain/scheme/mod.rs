//! 体系（Scheme）：项目数据结构的「设计期定义」，可复用、可多套。
//!
//! 与项目数据的关系：体系定义「这类项目要哪些数据」（字段 + 自定义列表），
//! 项目按执行方式绑定一套；模板是数据消费者（引用 field/table_key），装配读
//! 「项目数据 × 体系结构」出正文。详见实体注释与 0010 迁移。

pub mod entity;
pub mod repository;

pub use entity::{
    Scheme, SchemeField, SchemeFieldSpec, SchemeFull, SchemeSpec, SchemeTable, SchemeTableColumn,
    SchemeTableDef, SchemeTableSpec,
};
pub use repository::SchemeRepository;
