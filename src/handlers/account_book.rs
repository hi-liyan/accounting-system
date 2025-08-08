use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect},
    Form,
};
use askama::Template;
use serde::{Deserialize, Serialize};
use validator::Validate;
use chrono::{DateTime, Utc};

use crate::middleware::{AppState, CurrentUser};
use crate::models::{AccountBook, CreateAccountBook};
use crate::utils::AccountBookForm;

#[derive(Template)]
#[template(path = "account_books/new.html")]
struct NewAccountBookTemplate {
    error: String,
}

#[derive(Deserialize)]
pub struct AccountBookQuery {
    error: Option<String>,
    success: Option<String>,
}

pub async fn show_new(_user: CurrentUser, Query(query): Query<AccountBookQuery>) -> Html<String> {
    let template = NewAccountBookTemplate {
        error: query.error.unwrap_or_default(),
    };
    Html(template.render().unwrap())
}

pub async fn create(
    user: CurrentUser,
    State(app_state): State<AppState>,
    Form(form): Form<AccountBookForm>,
) -> Redirect {
    // 验证表单
    if let Err(errors) = form.validate() {
        let error_msg = errors
            .field_errors()
            .iter()
            .map(|(field, errs)| {
                let field_errors = errs.iter()
                    .map(|e| e.message.as_ref().unwrap_or(&"验证失败".into()).to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}: {}", field, field_errors)
            })
            .collect::<Vec<_>>()
            .join("; ");
        let final_error = if error_msg.is_empty() { "输入数据无效".to_string() } else { error_msg };
        return Redirect::to(&format!("/account-books/new?error={}", 
            urlencoding::encode(&final_error)));
    }

    let create_book = CreateAccountBook {
        user_id: user.id,
        name: form.name,
        description: form.description,
        currency: form.currency,
        cycle_start_day: form.cycle_start_day,
    };

    match AccountBook::create(&app_state.db_pool, create_book).await {
        Ok(_) => Redirect::to("/dashboard?success=账本创建成功！"),
        Err(e) => {
            let error_msg = e.to_string();
            Redirect::to(&format!("/account-books/new?error={}", 
                urlencoding::encode(&error_msg)))
        }
    }
}

pub async fn list(
    user: CurrentUser,
    State(app_state): State<AppState>,
    Query(query): Query<AccountBookQuery>,
) -> Result<Html<String>, Redirect> {
    match AccountBook::find_by_user(&app_state.db_pool, user.id).await {
        Ok(books) => {
            let books_display: Vec<AccountBookDisplay> = books.into_iter().map(AccountBookDisplay::from).collect();
            let template = AccountBookListTemplate {
                user,
                books: books_display,
                error: query.error.unwrap_or_default(),
                success: query.success.unwrap_or_default(),
            };
            Ok(Html(template.render().unwrap()))
        }
        Err(_) => Err(Redirect::to("/dashboard?error=无法加载账本列表")),
    }
}

#[derive(Debug, Serialize)]
pub struct AccountBookDisplay {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: String,
    pub currency: String,
    pub cycle_start_day: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<AccountBook> for AccountBookDisplay {
    fn from(book: AccountBook) -> Self {
        Self {
            id: book.id,
            user_id: book.user_id,
            name: book.name,
            description: book.description.unwrap_or_default(),
            currency: book.currency,
            cycle_start_day: book.cycle_start_day,
            is_active: book.is_active,
            created_at: book.created_at,
            updated_at: book.updated_at,
        }
    }
}

#[derive(Template)]
#[template(path = "account_books/list.html")]
struct AccountBookListTemplate {
    user: CurrentUser,
    books: Vec<AccountBookDisplay>,
    error: String,
    success: String,
}

#[derive(Template)]
#[template(path = "account_books/detail.html")]
struct AccountBookDetailTemplate {
    user: CurrentUser,
    book: AccountBookDisplay,
    transaction_count: i64,
    total_income: rust_decimal::Decimal,
    total_expense: rust_decimal::Decimal,
    success: String,
    error: String,
}

#[derive(Template)]
#[template(path = "account_books/edit.html")]
struct EditAccountBookTemplate {
    user: CurrentUser,
    book: AccountBookDisplay,
    error: String,
}

#[derive(Deserialize)]
pub struct UpdateAccountBook {
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub cycle_start_day: i32,
}

// 查看账本详细信息
pub async fn detail(
    user: CurrentUser,
    Path(id): Path<i64>,
    State(app_state): State<AppState>,
    Query(query): Query<AccountBookQuery>,
) -> Result<Html<String>, Redirect> {
    match AccountBook::find_by_id(&app_state.db_pool, id, user.id).await {
        Ok(Some(book)) => {
            // 获取统计信息
            let transaction_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM transactions WHERE account_book_id = ?"
            )
            .bind(id)
            .fetch_one(&app_state.db_pool)
            .await
            .unwrap_or(0);

            let income_result: Option<rust_decimal::Decimal> = sqlx::query_scalar(
                "SELECT SUM(amount) FROM transactions WHERE account_book_id = ? AND `type` = 'income'"
            )
            .bind(id)
            .fetch_optional(&app_state.db_pool)
            .await
            .unwrap_or(None);

            let expense_result: Option<rust_decimal::Decimal> = sqlx::query_scalar(
                "SELECT SUM(amount) FROM transactions WHERE account_book_id = ? AND `type` = 'expense'"
            )
            .bind(id)
            .fetch_optional(&app_state.db_pool)
            .await
            .unwrap_or(None);

            let template = AccountBookDetailTemplate {
                user,
                book: AccountBookDisplay::from(book),
                transaction_count,
                total_income: income_result.unwrap_or(rust_decimal::Decimal::ZERO),
                total_expense: expense_result.unwrap_or(rust_decimal::Decimal::ZERO),
                success: query.success.unwrap_or_default(),
                error: query.error.unwrap_or_default(),
            };
            Ok(Html(template.render().unwrap()))
        }
        Ok(None) => Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
        Err(_) => Err(Redirect::to("/account-books?error=加载账本详情失败")),
    }
}

// 显示编辑账本页面
pub async fn show_edit(
    user: CurrentUser,
    Path(id): Path<i64>,
    State(app_state): State<AppState>,
    Query(query): Query<AccountBookQuery>,
) -> Result<Html<String>, Redirect> {
    match AccountBook::find_by_id(&app_state.db_pool, id, user.id).await {
        Ok(Some(book)) => {
            let template = EditAccountBookTemplate {
                user,
                book: AccountBookDisplay::from(book),
                error: query.error.unwrap_or_default(),
            };
            Ok(Html(template.render().unwrap()))
        }
        Ok(None) => Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
        Err(_) => Err(Redirect::to("/account-books?error=加载账本失败")),
    }
}

// 更新账本
pub async fn update(
    user: CurrentUser,
    Path(id): Path<i64>,
    State(app_state): State<AppState>,
    Form(form): Form<UpdateAccountBook>,
) -> Redirect {
    // 简单验证
    if form.name.trim().is_empty() {
        return Redirect::to(&format!("/account-books/{}/edit?error=账本名称不能为空", id));
    }

    if form.cycle_start_day < 1 || form.cycle_start_day > 31 {
        return Redirect::to(&format!("/account-books/{}/edit?error=月度周期起始日必须在1-31之间", id));
    }

    // 检查账本是否存在且属于当前用户
    match AccountBook::find_by_id(&app_state.db_pool, id, user.id).await {
        Ok(Some(_)) => {
            // 更新账本
            match AccountBook::update(
                &app_state.db_pool,
                id,
                user.id,
                &form.name,
                form.description.as_deref(),
                &form.currency,
                form.cycle_start_day,
            ).await {
                Ok(_) => Redirect::to(&format!("/account-books/{}?success=账本更新成功！", id)),
                Err(e) => {
                    let error_msg = e.to_string();
                    Redirect::to(&format!("/account-books/{}/edit?error={}", id, 
                        urlencoding::encode(&error_msg)))
                }
            }
        }
        Ok(None) => Redirect::to("/account-books?error=账本不存在或无权限访问"),
        Err(_) => Redirect::to("/account-books?error=操作失败"),
    }
}

// 删除账本
pub async fn delete(
    user: CurrentUser,
    Path(id): Path<i64>,
    State(app_state): State<AppState>,
) -> Redirect {
    // 检查账本是否存在且属于当前用户
    match AccountBook::find_by_id(&app_state.db_pool, id, user.id).await {
        Ok(Some(_)) => {
            // 删除账本（软删除）
            match AccountBook::delete(&app_state.db_pool, id, user.id).await {
                Ok(_) => Redirect::to("/account-books?success=账本删除成功！"),
                Err(e) => {
                    let error_msg = e.to_string();
                    Redirect::to(&format!("/account-books?error={}", 
                        urlencoding::encode(&error_msg)))
                }
            }
        }
        Ok(None) => Redirect::to("/account-books?error=账本不存在或无权限访问"),
        Err(_) => Redirect::to("/account-books?error=操作失败"),
    }
}