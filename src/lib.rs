pub mod config;
pub mod database;
pub mod models;
pub mod handlers;
pub mod services;
pub mod middleware;
pub mod utils;

use axum::{
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    services::ServeDir,
    trace::TraceLayer,
    cors::CorsLayer,
};

use config::AppConfig;
use database::create_pool;
use services::{AuthService, EmailService};
use middleware::AppState;
use handlers::*;

pub async fn create_app() -> anyhow::Result<Router> {
    // 加载配置
    let config = AppConfig::from_env()?;
    
    // 创建数据库连接池
    let db_pool = create_pool(&config.database_url).await?;
    
    // 创建邮件服务
    let email_service = EmailService::new(&config.smtp, config.app_url.clone())?;
    
    // 创建认证服务
    let auth_service = AuthService::new(config.jwt_secret.clone(), email_service);
    
    // 创建应用状态
    let app_state = AppState {
        db_pool,
        auth_service,
    };

    // 创建路由
    let app = Router::new()
        // 首页和仪表板
        .route("/", get(index))
        .route("/dashboard", get(dashboard))
        
        // 认证路由
        .route("/auth/login", get(show_login).post(login))
        .route("/auth/register", get(show_register).post(register))
        .route("/auth/logout", post(logout))
        .route("/auth/verify/:token", get(verify_email))
        .route("/auth/resend-verification", post(resend_verification))
        
        // 静态文件服务
        .nest_service("/static", ServeDir::new("static"))
        
        // 添加状态
        .with_state(app_state)
        
        // 添加中间件
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        );

    Ok(app)
}

pub fn get_config() -> anyhow::Result<AppConfig> {
    Ok(AppConfig::from_env()?)
}