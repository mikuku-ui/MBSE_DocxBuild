pub mod graph;
pub mod service;

// 对外公共 API 再导出（HTTP 层或集成层仍可能从 crate::application::trace 取）。
#[allow(unused_imports)]
pub use graph::Direction;
pub use service::TraceService;
