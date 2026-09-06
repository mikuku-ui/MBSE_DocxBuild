//! 装配引擎：模板（视图脚本）逐节点求值 → 项目数据（追踪图 + 台账）→ 文档正文。
//!
//! 定位（对齐知识库「文档只是视图」）：模板是**视图脚本**，节点 `content_spec`
//! 声明「这里的内容从哪来」；装配按文档序对模板逐段求值，从**项目数据**把正文
//! 算出来——文档不存业务数据，只是投影。ConTeXt 导出走既有
//! `integrations/context` port（本轮仍只出 JSON 预览，生成器留后续迭代）。

pub mod content;
pub mod service;

pub use service::AssemblyService;
