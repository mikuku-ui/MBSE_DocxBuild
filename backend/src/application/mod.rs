//! 应用层：用例编排 + 规则 + 领域事务。
//!
//! 只抛 `ServiceError`，由 api 层统一映射成状态码；不 import 任何 DB/HTTP 类型。

pub mod assembly;
pub mod document;
pub mod error;
pub mod execution;
pub mod project;
pub mod projectdata;
pub mod scheme;
pub mod trace;

pub use error::ServiceError;
