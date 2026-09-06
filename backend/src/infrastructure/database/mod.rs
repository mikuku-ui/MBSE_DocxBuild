//! Database infrastructure (PostgreSQL via SQLx)。

pub mod document;
pub mod execution;
pub mod postgres;
pub mod project;
pub mod projectdata;
pub mod scheme;
pub mod trace;

pub use document::PostgresDocumentRepository;
pub use execution::PostgresExecutionRepository;
pub use postgres::connect;
pub use project::PostgresProjectRepository;
pub use projectdata::PostgresProjectDataRepository;
pub use scheme::PostgresSchemeRepository;
pub use trace::{
    PostgresBaselineRepository, PostgresTraceLinkRepository, PostgresTraceNodeRepository,
};
