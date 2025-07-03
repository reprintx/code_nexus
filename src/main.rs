use code_nexus::CodeNexusServer;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // 创建并启动 MCP 服务器
    let server = CodeNexusServer::new().await
        .map_err(|e| anyhow::anyhow!("创建服务器失败: {:?}", e))?;
    let service = server.serve(stdio()).await
        .map_err(|e| anyhow::anyhow!("启动服务失败: {:?}", e))?;

    tracing::info!("CodeNexus MCP 服务器已启动");
    service.waiting().await
        .map_err(|e| anyhow::anyhow!("服务运行失败: {:?}", e))?;

    Ok(())
}
