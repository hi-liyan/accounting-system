use axum::{
    extract::{Query, State},
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
pub struct AccountBookErrorQuery {
    error: Option<String>,
}

pub async fn show_new(_user: CurrentUser, Query(query): Query<AccountBookErrorQuery>) -> Html<String> {
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
) -> Result<Html<String>, Redirect> {
    match AccountBook::find_by_user(&app_state.db_pool, user.id).await {
        Ok(books) => {
            let books_display: Vec<AccountBookDisplay> = books.into_iter().map(AccountBookDisplay::from).collect();
            let template = AccountBookListTemplate {
                user,
                books: books_display,
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
}