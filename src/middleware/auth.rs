use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};

use crate::services::AuthService;
use crate::models::User;
use crate::database::DbPool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: i64,
    pub email: String,
    pub username: String,
    pub is_verified: bool,
}

impl From<User> for CurrentUser {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            is_verified: user.is_verified,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DbPool,
    pub auth_service: AuthService,
}

#[async_trait]
impl FromRequestParts<AppState> for CurrentUser
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let app_state = state;

        let cookies = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to read cookies").into_response()
            })?;

        // 尝试从cookie中获取token
        let token = cookies
            .get("auth_token")
            .map(|cookie| cookie.value())
            .or_else(|| {
                // 尝试从Authorization header获取token
                parts
                    .headers
                    .get("Authorization")
                    .and_then(|header| header.to_str().ok())
                    .and_then(|header| header.strip_prefix("Bearer "))
            })
            .ok_or_else(|| Redirect::to("/auth/login").into_response())?;

        // 验证token
        let claims = app_state
            .auth_service
            .verify_token(token)
            .map_err(|_| Redirect::to("/auth/login").into_response())?;

        // 获取用户信息
        let user = app_state
            .auth_service
            .get_current_user(&app_state.db_pool, claims.sub)
            .await
            .map_err(|_| Redirect::to("/auth/login").into_response())?;

        Ok(CurrentUser::from(user))
    }
}

// 可选的认证中间件，不会重定向
pub struct OptionalCurrentUser(pub Option<CurrentUser>);

#[async_trait]
impl FromRequestParts<AppState> for OptionalCurrentUser
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        match CurrentUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalCurrentUser(Some(user))),
            Err(_) => Ok(OptionalCurrentUser(None)),
        }
    }
}