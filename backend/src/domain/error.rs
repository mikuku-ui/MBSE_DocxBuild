//! 领域层共享的存储错误。
//!
//! 刻意不带任何基础设施类型（无 sqlx），让 domain / application 与具体数据库
//! 解耦。基础设施层负责把 sqlx 错误翻译成 `Database(String)`。

use std::fmt;

#[derive(Debug, Clone)]
pub enum RepositoryError {
    NotFound,
    /// 底层唯一约束冲突（如项目名重复）。应用层按需翻译成更精确的 ServiceError::Conflict。
    Conflict,
    Database(String),
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepositoryError::NotFound => write!(f, "resource not found"),
            RepositoryError::Conflict => write!(f, "conflict"),
            RepositoryError::Database(m) => write!(f, "database error: {m}"),
        }
    }
}

impl std::error::Error for RepositoryError {}
