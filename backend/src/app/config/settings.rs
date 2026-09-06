//! 运行时设置。从 `config.toml` + 环境变量加载（数据库凭据不进配置）。

use std::path::PathBuf;

use anyhow::{bail, Context};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
struct RawSettings {
    server: RawServer,
    #[serde(default)]
    database: RawDatabase,
}

#[derive(Debug, Clone, Deserialize)]
struct RawServer {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct RawDatabase {
    /// 可选覆盖 `DATABASE_URL`；为空时回落到环境变量。
    url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub host: String,
    pub port: u16,
    database_url: Option<String>,
}

impl Settings {
    /// 解析数据库连接串：config 覆盖优先，其次 `DATABASE_URL` 环境变量。
    pub fn database_url(&self) -> anyhow::Result<String> {
        if let Some(url) = &self.database_url {
            if !url.trim().is_empty() {
                return Ok(url.clone());
            }
        }
        std::env::var("DATABASE_URL").context(
            "DATABASE_URL is not set. Create a .env file from .env.example \
             or set database.url in config.toml.",
        )
    }

    pub fn load() -> anyhow::Result<Settings> {
        let _ = dotenvy::dotenv();

        let args: Vec<String> = std::env::args().collect();
        let explicit = args
            .windows(2)
            .find(|w| w[0] == "--config")
            .map(|w| PathBuf::from(&w[1]));

        let path = match explicit {
            Some(p) => Some(p),
            None => locate_default()?,
        };
        let path = match path {
            Some(p) => p,
            None => {
                create_sample()?;
                bail!("No config.toml found. A sample config.toml was created; edit it and restart.");
            }
        };

        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file {}", path.display()))?;
        let raw: RawSettings =
            toml::from_str(&text).with_context(|| format!("Invalid TOML in {}", path.display()))?;

        if raw.server.host.trim().is_empty() {
            bail!("config `server.host` must not be empty");
        }
        if raw.server.port == 0 {
            bail!("config `server.port` must not be zero");
        }

        Ok(Settings {
            host: raw.server.host,
            port: raw.server.port,
            database_url: raw.database.url,
        })
    }
}

fn locate_default() -> anyhow::Result<Option<PathBuf>> {
    let cwd = std::env::current_dir().context("Failed to read current directory")?;
    let candidates = [
        cwd.join("config.toml"),
        cwd.parent()
            .map(|p| p.join("config.toml"))
            .unwrap_or_else(|| cwd.join("config.toml")),
    ];
    Ok(candidates.into_iter().find(|p| p.is_file()))
}

fn create_sample() -> anyhow::Result<()> {
    let path = std::env::current_dir()?.join("config.toml");
    let sample = r#"[server]
host = "127.0.0.1"
port = 38123

[database]
# Optional override. Empty falls back to the DATABASE_URL environment variable.
url = ""
"#;
    std::fs::write(&path, sample)
        .with_context(|| format!("Failed to create sample config at {}", path.display()))?;
    println!("Created sample config at {}", path.display());
    Ok(())
}
