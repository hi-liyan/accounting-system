use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub smtp: SmtpConfig,
    pub app_url: String,
    pub session_secret: String,
}

#[derive(Debug, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        dotenvy::dotenv().ok();
        
        let _cfg = config::Config::builder()
            .add_source(config::Environment::default().separator("_"))
            .build()?;
            
        let database_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let host = std::env::var("HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .expect("PORT must be a valid number");
        let jwt_secret = std::env::var("JWT_SECRET")
            .expect("JWT_SECRET must be set");
        let app_url = std::env::var("APP_URL")
            .unwrap_or_else(|_| format!("http://{}:{}", host, port));
        let session_secret = std::env::var("SESSION_SECRET")
            .expect("SESSION_SECRET must be set");
            
        let smtp = SmtpConfig {
            host: std::env::var("SMTP_HOST").unwrap_or_default(),
            port: std::env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse::<u16>()
                .unwrap_or(587),
            username: std::env::var("SMTP_USERNAME").unwrap_or_default(),
            password: std::env::var("SMTP_PASSWORD").unwrap_or_default(),
            from_email: std::env::var("FROM_EMAIL").unwrap_or_default(),
        };
        
        Ok(AppConfig {
            database_url,
            host,
            port,
            jwt_secret,
            smtp,
            app_url,
            session_secret,
        })
    }
    
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}