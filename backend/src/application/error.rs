//! 应用层统一错误：4 变体，由 api 层映射为 HTTP 状态码（对齐上项目）。

use std::fmt;

use crate::domain::RepositoryError;

#[derive(Debug, Clone)]
pub enum ServiceError {
    NotFound(String),
    Validation(String),
    Conflict(String),
    Repository(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::NotFound(m) => write!(f, "not found: {m}"),
            ServiceError::Validation(m) => write!(f, "validation: {m}"),
            ServiceError::Conflict(m) => write!(f, "conflict: {m}"),
            ServiceError::Repository(m) => write!(f, "repository: {m}"),
        }
    }
}

impl std::error::Error for ServiceError {}

impl From<RepositoryError> for ServiceError {
    fn from(e: RepositoryError) -> Self {
        match e {
            RepositoryError::NotFound => ServiceError::NotFound("resource_not_found".into()),
            RepositoryError::Conflict => ServiceError::Conflict("resource_conflict".into()),
            RepositoryError::Database(m) => ServiceError::Repository(m),
        }
    }
}
