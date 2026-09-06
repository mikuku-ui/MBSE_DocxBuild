//! 配置层。从 `config.toml` 与环境变量加载运行时设置；不含业务逻辑。

pub mod settings;

pub use settings::Settings;
