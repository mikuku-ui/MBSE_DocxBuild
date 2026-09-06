//! PostgreSQL 连接 + 迁移入口。

use std::str::FromStr;
use std::time::Duration;

use log::LevelFilter;
use sqlx::ConnectOptions;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};

/// 连接数据库并应用（追加式、不可变）迁移，返回连接池。
///
/// sqlx 默认把「慢语句（>1s）/慢取连接（>2s）」按 WARN 打出——本地冷启动（首次建连、
/// TLS、预热迁移）偶发越阈，纯属噪声。这里把阈值放宽到 3s / 5s 静默它们，真实性能问题
/// 仍会被捕获。
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let options = PgConnectOptions::from_str(database_url)?
        .log_slow_statements(LevelFilter::Warn, Duration::from_secs(3));
    let pool = PgPoolOptions::new()
        .acquire_slow_threshold(Duration::from_secs(5))
        .connect_with(options)
        .await?;
    sqlx::migrate!("./src/infrastructure/database/migrations")
        .run(&pool)
        .await?;
    Ok(pool)
}

/// 把 sqlx 错误翻译成领域错误字符串（仓库层共用）。
pub(crate) fn db_err(e: sqlx::Error) -> crate::domain::RepositoryError {
    crate::domain::RepositoryError::Database(e.to_string())
}
