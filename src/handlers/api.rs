use axum::{
    extract::{Path, State},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};

use crate::middleware::{AppState, CurrentUser};
use crate::models::{AccountBook, User};

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

#[derive(Serialize)]
pub struct PreferenceUpdateResponse {
    pub account_book_id: i64,
}

#[derive(Deserialize)]
pub struct UpdatePreferenceRequest {
    pub account_book_id: i64,
}

// 更新用户账本偏好
pub async fn update_account_book_preference(
    user: CurrentUser,
    State(app_state): State<AppState>,
    Json(request): Json<UpdatePreferenceRequest>,
) -> Result<Json<ApiResponse<PreferenceUpdateResponse>>, StatusCode> {
    // 验证账本是否属于当前用户
    match AccountBook::find_by_id(&app_state.db_pool, request.account_book_id, user.id).await {
        Ok(Some(_)) => {
            // 更新用户偏好
            match User::update_last_selected_account_book(
                &app_state.db_pool, 
                user.id, 
                Some(request.account_book_id)
            ).await {
                Ok(_) => {
                    Ok(Json(ApiResponse {
                        success: true,
                        message: "账本偏好更新成功".to_string(),
                        data: Some(PreferenceUpdateResponse {
                            account_book_id: request.account_book_id,
                        }),
                    }))
                }
                Err(_) => {
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => {
            Err(StatusCode::NOT_FOUND)
        }
        Err(_) => {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// 通过URL路径参数更新偏好（用于GET请求）
pub async fn update_preference_by_path(
    user: CurrentUser,
    Path(account_book_id): Path<i64>,
    State(app_state): State<AppState>,
) -> Result<Json<ApiResponse<PreferenceUpdateResponse>>, StatusCode> {
    // 验证账本是否属于当前用户
    match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(_)) => {
            // 更新用户偏好
            match User::update_last_selected_account_book(
                &app_state.db_pool, 
                user.id, 
                Some(account_book_id)
            ).await {
                Ok(_) => {
                    Ok(Json(ApiResponse {
                        success: true,
                        message: "账本偏好更新成功".to_string(),
                        data: Some(PreferenceUpdateResponse {
                            account_book_id,
                        }),
                    }))
                }
                Err(_) => {
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => {
            Err(StatusCode::NOT_FOUND)
        }
        Err(_) => {
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}