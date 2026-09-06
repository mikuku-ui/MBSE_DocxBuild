//! 领域层：实体 + 值对象 + 仓储 port。
//!
//! ★ 核心资产。不 import 任何 DB/HTTP 类型；仓储用 trait（port）表达，
//! 实现放 infrastructure。下层永不 import 上层。

pub mod document;
pub mod error;
pub mod execution;
pub mod project;
pub mod projectdata;
pub mod scheme;
pub mod trace;

pub use error::RepositoryError;
