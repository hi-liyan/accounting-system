use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};
use anyhow::{anyhow, Result};
use uuid::Uuid;

use crate::models::{User, CreateUser};
use crate::utils::{hash_password, verify_password};
use crate::services::EmailService;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64, // subject (user id)
    pub email: String,
    pub exp: i64, // expiration time
    pub iat: i64, // issued at
}

#[derive(Clone)]
pub struct AuthService {
    jwt_secret: String,
    email_service: EmailService,
}

impl AuthService {
    pub fn new(jwt_secret: String, email_service: EmailService) -> Self {
        Self {
            jwt_secret,
            email_service,
        }
    }

    pub async fn register(
        &self,
        pool: &crate::database::DbPool,
        email: String,
        password: String,
    ) -> Result<()> {
        // 检查邮箱是否已存在
        if let Some(_) = User::find_by_email(pool, &email).await? {
            return Err(anyhow!("该邮箱已被注册"));
        }

        // 生成密码哈希
        let password_hash = hash_password(&password)?;

        // 生成验证令牌
        let verification_token = Uuid::new_v4().to_string();

        // 创建用户
        let create_user = CreateUser {
            email: email.clone(),
            password_hash,
            verification_token: verification_token.clone(),
        };

        User::create(pool, create_user).await?;

        // 尝试发送验证邮件，但不因为邮件发送失败而影响注册成功
        if let Err(e) = self.email_service
            .send_verification_email(&email, &verification_token)
            .await
        {
            // 记录邮件发送失败的日志，但不阻止注册流程
            tracing::warn!("Failed to send verification email to {}: {}", email, e);
            // 在生产环境中，你可能希望将这个错误记录到日志系统中
        }

        Ok(())
    }

    pub async fn login(
        &self,
        pool: &crate::database::DbPool,
        email: String,
        password: String,
    ) -> Result<(User, String)> {
        // 查找用户
        let user = User::find_by_email(pool, &email)
            .await?
            .ok_or_else(|| anyhow!("邮箱或密码错误"))?;

        // 验证密码
        if !verify_password(&password, &user.password_hash)? {
            return Err(anyhow!("邮箱或密码错误"));
        }

        // 检查邮箱是否已验证
        if !user.is_verified {
            return Err(anyhow!("请先验证您的邮箱"));
        }

        // 生成JWT
        let token = self.generate_token(&user)?;

        Ok((user, token))
    }

    pub async fn verify_email(&self, pool: &crate::database::DbPool, token: &str) -> Result<()> {
        if !User::verify_email(pool, token).await? {
            return Err(anyhow!("无效的验证令牌"));
        }
        Ok(())
    }

    pub async fn resend_verification(
        &self,
        pool: &crate::database::DbPool,
        email: String,
    ) -> Result<()> {
        let user = User::find_by_email(pool, &email)
            .await?
            .ok_or_else(|| anyhow!("用户不存在"))?;

        if user.is_verified {
            return Err(anyhow!("邮箱已验证"));
        }

        // 生成新的验证令牌
        let new_token = Uuid::new_v4().to_string();
        User::update_verification_token(pool, &email, &new_token).await?;

        // 发送验证邮件
        self.email_service
            .send_verification_email(&email, &new_token)
            .await?;

        Ok(())
    }

    pub fn generate_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(24); // 24小时过期

        let claims = Claims {
            sub: user.id,
            email: user.email.clone(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?;

        Ok(token)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    pub async fn get_current_user(&self, pool: &crate::database::DbPool, user_id: i64) -> Result<User> {
        User::find_by_id(pool, user_id)
            .await?
            .ok_or_else(|| anyhow!("用户不存在"))
    }
}