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
use handlers::{auth, dashboard, account_book, account_book_reports, category, transaction, api};

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
        .route("/", get(dashboard::index))
        .route("/dashboard", get(dashboard::dashboard))
        
        // 认证路由
        .route("/auth/login", get(auth::show_login).post(auth::login))
        .route("/auth/register", get(auth::show_register).post(auth::register))
        .route("/auth/logout", get(auth::logout).post(auth::logout))
        .route("/auth/verify/:token", get(auth::verify_email))
        .route("/auth/resend-verification", post(auth::resend_verification))
        
        // 账本路由
        .route("/account-books", get(account_book::list).post(account_book::create))
        .route("/account-books/new", get(account_book::show_new))
        .route("/account-books/:id", get(account_book::detail))
        .route("/account-books/:id/edit", get(account_book::show_edit))
        .route("/account-books/:id/update", post(account_book::update))
        .route("/account-books/:id/delete", post(account_book::delete))
        .route("/account-books/:id/reports", get(account_book_reports::reports))
        
        // 分类路由
        .route("/account-books/:id/categories", get(category::list).post(category::create))
        .route("/account-books/:id/categories/new", get(category::show_new))
        .route("/account-books/:account_book_id/categories/:category_id/edit", get(category::show_edit))
        .route("/account-books/:account_book_id/categories/:category_id/update", post(category::update))
        .route("/account-books/:account_book_id/categories/:category_id/delete", post(category::delete))
        
        // 交易路由
        .route("/account-books/:id/transactions", get(transaction::list))
        .route("/account-books/:id/transactions/new", get(transaction::show_new).post(transaction::create))
        .route("/account-books/:account_book_id/transactions/:transaction_id/edit", get(transaction::show_edit))
        .route("/account-books/:account_book_id/transactions/:transaction_id/update", post(transaction::update))
        .route("/account-books/:account_book_id/transactions/:transaction_id/delete", post(transaction::delete))
        
        // API路由
        .route("/api/preferences/account-book", post(api::update_account_book_preference))
        .route("/api/preferences/account-book/:id", post(api::update_preference_by_path))
        
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