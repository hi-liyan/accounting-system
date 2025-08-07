use accounting_system::{create_app, get_config};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "accounting_system=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载配置
    let config = get_config()?;
    let bind_addr = config.bind_address();
    
    tracing::info!("Starting server on {}", bind_addr);

    // 创建应用
    let app = create_app().await?;

    // 创建TCP监听器
    let listener = TcpListener::bind(&bind_addr).await?;
    
    tracing::info!("Server listening on http://{}", bind_addr);
    
    // 启动服务器
    axum::serve(listener, app).await?;

    Ok(())
}
