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
use crate::models::{AccountBook, Category, CreateCategory};

#[derive(Template)]
#[template(path = "categories/list.html")]
struct CategoryListTemplate {
    user: CurrentUser,
    account_book: AccountBookDisplay,
    income_categories: Vec<CategoryDisplay>,
    expense_categories: Vec<CategoryDisplay>,
    success: String,
    error: String,
}

#[derive(Template)]
#[template(path = "categories/new.html")]
struct NewCategoryTemplate {
    user: CurrentUser,
    account_book: AccountBookDisplay,
    error: String,
}

#[derive(Template)]
#[template(path = "categories/edit.html")]
struct EditCategoryTemplate {
    user: CurrentUser,
    account_book: AccountBookDisplay,
    category: CategoryDisplay,
    error: String,
}

#[derive(Debug, Serialize)]
pub struct AccountBookDisplay {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct CategoryDisplay {
    pub id: i64,
    pub account_book_id: i64,
    pub name: String,
    pub category_type: String,
    pub icon: String,
    pub color: String,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Category> for CategoryDisplay {
    fn from(category: Category) -> Self {
        Self {
            id: category.id,
            account_book_id: category.account_book_id,
            name: category.name,
            category_type: category.category_type,
            icon: category.icon.unwrap_or_default(),
            color: category.color.unwrap_or("#007bff".to_string()),
            sort_order: category.sort_order,
            is_active: category.is_active,
            created_at: category.created_at,
        }
    }
}

impl From<crate::models::AccountBook> for AccountBookDisplay {
    fn from(book: crate::models::AccountBook) -> Self {
        Self {
            id: book.id,
            name: book.name,
            description: book.description.unwrap_or_default(),
            currency: book.currency,
        }
    }
}

#[derive(Deserialize)]
pub struct CategoryQuery {
    error: Option<String>,
    success: Option<String>,
}

#[derive(Deserialize, Validate)]
pub struct CategoryForm {
    #[validate(length(min = 1, max = 100, message = "分类名称长度必须在1-100个字符之间"))]
    pub name: String,
    pub category_type: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(Deserialize, Validate)]
pub struct UpdateCategoryForm {
    #[validate(length(min = 1, max = 100, message = "分类名称长度必须在1-100个字符之间"))]
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

// 分类列表页面
pub async fn list(
    user: CurrentUser,
    Path(account_book_id): Path<i64>,
    State(app_state): State<AppState>,
    Query(query): Query<CategoryQuery>,
) -> Result<Html<String>, Redirect> {
    // 验证账本权限
    let account_book = match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(book)) => book,
        Ok(None) => return Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
        Err(_) => return Err(Redirect::to("/account-books?error=获取账本信息失败")),
    };

    // 获取分类列表
    match Category::find_by_account_book(&app_state.db_pool, account_book_id).await {
        Ok(categories) => {
            let mut income_categories = Vec::new();
            let mut expense_categories = Vec::new();

            for category in categories {
                let category_display = CategoryDisplay::from(category);
                if category_display.category_type == "income" {
                    income_categories.push(category_display);
                } else {
                    expense_categories.push(category_display);
                }
            }

            let template = CategoryListTemplate {
                user,
                account_book: AccountBookDisplay::from(account_book),
                income_categories,
                expense_categories,
                success: query.success.unwrap_or_default(),
                error: query.error.unwrap_or_default(),
            };
            Ok(Html(template.render().unwrap()))
        }
        Err(_) => Err(Redirect::to(&format!("/account-books/{}?error=加载分类列表失败", account_book_id))),
    }
}

// 显示新建分类页面
pub async fn show_new(
    user: CurrentUser,
    Path(account_book_id): Path<i64>,
    State(app_state): State<AppState>,
    Query(query): Query<CategoryQuery>,
) -> Result<Html<String>, Redirect> {
    // 验证账本权限
    let account_book = match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(book)) => book,
        Ok(None) => return Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
        Err(_) => return Err(Redirect::to("/account-books?error=获取账本信息失败")),
    };

    let template = NewCategoryTemplate {
        user,
        account_book: AccountBookDisplay::from(account_book),
        error: query.error.unwrap_or_default(),
    };
    Ok(Html(template.render().unwrap()))
}

// 创建新分类
pub async fn create(
    user: CurrentUser,
    Path(account_book_id): Path<i64>,
    State(app_state): State<AppState>,
    Form(form): Form<CategoryForm>,
) -> Redirect {
    // 验证账本权限
    match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(_)) => {}
        Ok(None) => return Redirect::to("/account-books?error=账本不存在或无权限访问"),
        Err(_) => return Redirect::to("/account-books?error=获取账本信息失败"),
    }

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
        return Redirect::to(&format!("/account-books/{}/categories/new?error={}", 
            account_book_id, urlencoding::encode(&final_error)));
    }

    // 验证分类类型
    if form.category_type != "income" && form.category_type != "expense" {
        return Redirect::to(&format!("/account-books/{}/categories/new?error=无效的分类类型", account_book_id));
    }

    let create_category = CreateCategory {
        account_book_id,
        name: form.name,
        category_type: form.category_type,
        icon: form.icon.filter(|s| !s.is_empty()),
        color: form.color.filter(|s| !s.is_empty()),
    };

    match Category::create(&app_state.db_pool, create_category).await {
        Ok(_) => Redirect::to(&format!("/account-books/{}/categories?success=分类创建成功！", account_book_id)),
        Err(e) => {
            let error_msg = e.to_string();
            Redirect::to(&format!("/account-books/{}/categories/new?error={}", 
                account_book_id, urlencoding::encode(&error_msg)))
        }
    }
}

// 显示编辑分类页面
pub async fn show_edit(
    user: CurrentUser,
    Path((account_book_id, category_id)): Path<(i64, i64)>,
    State(app_state): State<AppState>,
    Query(query): Query<CategoryQuery>,
) -> Result<Html<String>, Redirect> {
    // 验证账本权限
    let account_book = match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(book)) => book,
        Ok(None) => return Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
        Err(_) => return Err(Redirect::to("/account-books?error=获取账本信息失败")),
    };

    // 获取分类信息
    match Category::find_by_id(&app_state.db_pool, category_id).await {
        Ok(Some(category)) => {
            // 验证分类属于该账本
            if category.account_book_id != account_book_id {
                return Err(Redirect::to(&format!("/account-books/{}/categories?error=分类不属于该账本", account_book_id)));
            }

            let template = EditCategoryTemplate {
                user,
                account_book: AccountBookDisplay::from(account_book),
                category: CategoryDisplay::from(category),
                error: query.error.unwrap_or_default(),
            };
            Ok(Html(template.render().unwrap()))
        }
        Ok(None) => Err(Redirect::to(&format!("/account-books/{}/categories?error=分类不存在", account_book_id))),
        Err(_) => Err(Redirect::to(&format!("/account-books/{}/categories?error=获取分类信息失败", account_book_id))),
    }
}

// 更新分类
pub async fn update(
    user: CurrentUser,
    Path((account_book_id, category_id)): Path<(i64, i64)>,
    State(app_state): State<AppState>,
    Form(form): Form<UpdateCategoryForm>,
) -> Redirect {
    // 验证账本权限
    match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(_)) => {}
        Ok(None) => return Redirect::to("/account-books?error=账本不存在或无权限访问"),
        Err(_) => return Redirect::to("/account-books?error=获取账本信息失败"),
    }

    // 验证分类存在且属于该账本
    match Category::find_by_id(&app_state.db_pool, category_id).await {
        Ok(Some(category)) => {
            if category.account_book_id != account_book_id {
                return Redirect::to(&format!("/account-books/{}/categories?error=分类不属于该账本", account_book_id));
            }
        }
        Ok(None) => return Redirect::to(&format!("/account-books/{}/categories?error=分类不存在", account_book_id)),
        Err(_) => return Redirect::to(&format!("/account-books/{}/categories?error=获取分类信息失败", account_book_id)),
    }

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
        return Redirect::to(&format!("/account-books/{}/categories/{}/edit?error={}", 
            account_book_id, category_id, urlencoding::encode(&final_error)));
    }

    // 更新分类
    match Category::update(
        &app_state.db_pool,
        category_id,
        &form.name,
        form.icon.as_deref().filter(|s| !s.is_empty()),
        form.color.as_deref().filter(|s| !s.is_empty()),
    ).await {
        Ok(_) => Redirect::to(&format!("/account-books/{}/categories?success=分类更新成功！", account_book_id)),
        Err(e) => {
            let error_msg = e.to_string();
            Redirect::to(&format!("/account-books/{}/categories/{}/edit?error={}", 
                account_book_id, category_id, urlencoding::encode(&error_msg)))
        }
    }
}

// 删除分类
pub async fn delete(
    user: CurrentUser,
    Path((account_book_id, category_id)): Path<(i64, i64)>,
    State(app_state): State<AppState>,
) -> Redirect {
    // 验证账本权限
    match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(_)) => {}
        Ok(None) => return Redirect::to("/account-books?error=账本不存在或无权限访问"),
        Err(_) => return Redirect::to("/account-books?error=获取账本信息失败"),
    }

    // 验证分类存在且属于该账本
    match Category::find_by_id(&app_state.db_pool, category_id).await {
        Ok(Some(category)) => {
            if category.account_book_id != account_book_id {
                return Redirect::to(&format!("/account-books/{}/categories?error=分类不属于该账本", account_book_id));
            }
        }
        Ok(None) => return Redirect::to(&format!("/account-books/{}/categories?error=分类不存在", account_book_id)),
        Err(_) => return Redirect::to(&format!("/account-books/{}/categories?error=获取分类信息失败", account_book_id)),
    }

    // 检查是否有交易记录使用该分类
    let transaction_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions WHERE category_id = ?"
    )
    .bind(category_id)
    .fetch_one(&app_state.db_pool)
    .await
    .unwrap_or(0);

    if transaction_count > 0 {
        return Redirect::to(&format!(
            "/account-books/{}/categories?error=无法删除分类，该分类下有{}笔交易记录", 
            account_book_id, transaction_count
        ));
    }

    // 删除分类
    match Category::delete(&app_state.db_pool, category_id).await {
        Ok(_) => Redirect::to(&format!("/account-books/{}/categories?success=分类删除成功！", account_book_id)),
        Err(e) => {
            let error_msg = e.to_string();
            Redirect::to(&format!("/account-books/{}/categories?error={}", 
                account_book_id, urlencoding::encode(&error_msg)))
        }
    }
}