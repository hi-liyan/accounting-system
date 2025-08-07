use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub is_verified: bool,
    pub verification_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i64,
    pub email: String,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        UserResponse {
            id: user.id,
            email: user.email,
            is_verified: user.is_verified,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateUser {
    pub email: String,
    pub password_hash: String,
    pub verification_token: String,
}

impl User {
    pub async fn create(pool: &crate::database::DbPool, create_user: CreateUser) -> anyhow::Result<User> {
        let result = sqlx::query(
            r#"
            INSERT INTO users (email, password_hash, verification_token)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&create_user.email)
        .bind(&create_user.password_hash)
        .bind(&create_user.verification_token)
        .execute(pool)
        .await?;

        let user_id = result.last_insert_id() as i64;
        let user = Self::find_by_id(pool, user_id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created user"))?;

        Ok(user)
    }

    pub async fn find_by_email(pool: &crate::database::DbPool, email: &str) -> anyhow::Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = ?")
            .bind(email)
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }

    pub async fn find_by_id(pool: &crate::database::DbPool, id: i64) -> anyhow::Result<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(user)
    }

    pub async fn verify_email(pool: &crate::database::DbPool, token: &str) -> anyhow::Result<bool> {
        let rows_affected = sqlx::query(
            "UPDATE users SET is_verified = TRUE, verification_token = NULL WHERE verification_token = ?"
        )
        .bind(token)
        .execute(pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn update_verification_token(
        pool: &crate::database::DbPool,
        email: &str,
        token: &str,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE users SET verification_token = ? WHERE email = ?")
            .bind(token)
            .bind(email)
            .execute(pool)
            .await?;

        Ok(())
    }
}