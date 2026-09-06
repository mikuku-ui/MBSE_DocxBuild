mod api;
mod app;
mod application;
mod domain;
mod infrastructure;
mod integrations;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app::bootstrap::run().await
}
