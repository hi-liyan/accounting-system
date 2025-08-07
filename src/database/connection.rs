use sqlx::{MySql, Pool, MySqlPool};
use anyhow::Result;

pub type DbPool = Pool<MySql>;

pub async fn create_pool(database_url: &str) -> Result<DbPool> {
    let pool = MySqlPool::connect(database_url).await?;
    
    // 运行数据库迁移（如果需要）
    // sqlx::migrate!("./migrations").run(&pool).await?;
    
    Ok(pool)
}