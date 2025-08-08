use axum::{
    extract::{Path, Query, State, Form},
    response::{Html, Redirect},
};
use askama::Template;
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use rust_decimal::Decimal;

use crate::middleware::{CurrentUser, AppState};
use crate::models::{AccountBook, Transaction, CreateTransaction, Category};

#[derive(Template)]
#[template(path = "transactions/list.html")]
struct TransactionListTemplate {
    user: CurrentUser,
    account_book: AccountBookDisplay,
    transactions: Vec<TransactionDisplay>,
    page: i64,
    has_next: bool,
    success: String,
    error: String,
}

#[derive(Template)]
#[template(path = "transactions/new.html")]
struct NewTransactionTemplate {
    user: CurrentUser,
    account_book: AccountBookDisplay,
    categories: Vec<CategoryDisplay>,
    error: String,
}

#[derive(Template)]
#[template(path = "transactions/edit.html")]
struct EditTransactionTemplate {
    user: CurrentUser,
    account_book: AccountBookDisplay,
    transaction: TransactionDisplay,
    categories: Vec<CategoryDisplay>,
    error: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct AccountBookDisplay {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct TransactionDisplay {
    pub id: i64,
    pub amount: Decimal,
    pub transaction_type: String,
    pub description: String,
    pub transaction_date: NaiveDate,
    pub transaction_date_str: String, // 添加格式化后的日期字符串
    pub category_id: i64,
    pub category_name: String,
    pub category_icon: String,
    pub category_color: String,
    pub tags: String,
}

#[derive(Debug, Serialize)]
pub struct CategoryDisplay {
    pub id: i64,
    pub name: String,
    pub category_type: String,
    pub icon: String,
    pub color: String,
}

#[derive(Deserialize)]
pub struct TransactionQuery {
    page: Option<i64>,
    success: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateTransactionForm {
    pub category_id: i64,
    pub amount: String,
    pub transaction_type: String,
    pub description: Option<String>,
    pub transaction_date: String,
    pub tags: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTransactionForm {
    pub category_id: i64,
    pub amount: String,
    pub description: Option<String>,
    pub transaction_date: String,
    pub tags: Option<String>,
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

impl From<crate::models::TransactionWithCategory> for TransactionDisplay {
    fn from(t: crate::models::TransactionWithCategory) -> Self {
        Self {
            id: t.id,
            amount: t.amount,
            transaction_type: t.transaction_type,
            description: t.description.unwrap_or_default(),
            transaction_date: t.transaction_date,
            transaction_date_str: t.transaction_date.format("%Y-%m-%d").to_string(),
            category_id: t.category_id,
            category_name: t.category_name,
            category_icon: t.category_icon.unwrap_or("tag".to_string()),
            category_color: t.category_color.unwrap_or("#007bff".to_string()),
            tags: t.tags.unwrap_or_default(),
        }
    }
}

impl From<crate::models::Category> for CategoryDisplay {
    fn from(c: crate::models::Category) -> Self {
        Self {
            id: c.id,
            name: c.name,
            category_type: c.category_type,
            icon: c.icon.unwrap_or("tag".to_string()),
            color: c.color.unwrap_or("#007bff".to_string()),
        }
    }
}

// 交易列表
pub async fn list(
    user: CurrentUser,
    Path(account_book_id): Path<i64>,
    Query(query): Query<TransactionQuery>,
    State(app_state): State<AppState>,
) -> Result<Html<String>, Redirect> {
    // 验证账本所有权
    let account_book = match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(book)) => AccountBookDisplay::from(book),
        _ => return Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
    };

    let page = query.page.unwrap_or(1);
    let limit = 20;
    let offset = (page - 1) * limit;

    // 获取交易列表
    let transactions = Transaction::find_by_account_book_with_category(
        &app_state.db_pool,
        account_book_id,
        limit + 1, // 多查一条用于判断是否有下一页
        offset,
    ).await.unwrap_or_default();

    let has_next = transactions.len() > limit as usize;
    let transactions = transactions
        .into_iter()
        .take(limit as usize)
        .map(TransactionDisplay::from)
        .collect();

    let template = TransactionListTemplate {
        user,
        account_book,
        transactions,
        page,
        has_next,
        success: query.success.unwrap_or_default(),
        error: query.error.unwrap_or_default(),
    };

    Ok(Html(template.render().unwrap()))
}

// 显示新建交易页面
pub async fn show_new(
    user: CurrentUser,
    Path(account_book_id): Path<i64>,
    Query(query): Query<TransactionQuery>,
    State(app_state): State<AppState>,
) -> Result<Html<String>, Redirect> {
    // 验证账本所有权
    let account_book = match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(book)) => AccountBookDisplay::from(book),
        _ => return Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
    };

    // 获取账本的分类
    let categories = Category::find_by_account_book(&app_state.db_pool, account_book_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(CategoryDisplay::from)
        .collect();

    let template = NewTransactionTemplate {
        user,
        account_book,
        categories,
        error: query.error.unwrap_or_default(),
    };

    Ok(Html(template.render().unwrap()))
}

// 创建交易
pub async fn create(
    user: CurrentUser,
    Path(account_book_id): Path<i64>,
    State(app_state): State<AppState>,
    Form(form): Form<CreateTransactionForm>,
) -> Redirect {
    // 验证账本所有权
    if AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await.is_err() {
        return Redirect::to("/account-books?error=账本不存在或无权限访问");
    }

    // 验证并解析金额
    let amount = match form.amount.parse::<Decimal>() {
        Ok(amt) if amt > Decimal::ZERO => amt,
        _ => return Redirect::to(&format!("/account-books/{}/transactions/new?error=金额必须大于0", account_book_id)),
    };

    // 验证并解析日期
    let transaction_date = match NaiveDate::parse_from_str(&form.transaction_date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return Redirect::to(&format!("/account-books/{}/transactions/new?error=日期格式错误", account_book_id)),
    };

    // 验证交易类型
    if form.transaction_type != "income" && form.transaction_type != "expense" {
        return Redirect::to(&format!("/account-books/{}/transactions/new?error=交易类型无效", account_book_id));
    }

    // 验证分类是否存在且属于该账本
    match Category::find_by_id(&app_state.db_pool, form.category_id).await {
        Ok(Some(category)) if category.account_book_id == account_book_id => {
            // 验证分类类型与交易类型是否匹配
            if category.category_type != form.transaction_type {
                return Redirect::to(&format!("/account-books/{}/transactions/new?error=分类类型与交易类型不匹配", account_book_id));
            }
        }
        _ => return Redirect::to(&format!("/account-books/{}/transactions/new?error=分类不存在或无权限访问", account_book_id)),
    }

    let create_transaction = CreateTransaction {
        account_book_id,
        category_id: form.category_id,
        amount,
        transaction_type: form.transaction_type,
        description: form.description.filter(|s| !s.trim().is_empty()),
        transaction_date,
        tags: form.tags.filter(|s| !s.trim().is_empty()),
    };

    match Transaction::create(&app_state.db_pool, create_transaction).await {
        Ok(_) => Redirect::to(&format!("/account-books/{}/transactions?success=交易记录创建成功", account_book_id)),
        Err(_) => Redirect::to(&format!("/account-books/{}/transactions/new?error=创建交易记录失败", account_book_id)),
    }
}

// 显示编辑交易页面
pub async fn show_edit(
    user: CurrentUser,
    Path((account_book_id, transaction_id)): Path<(i64, i64)>,
    Query(query): Query<TransactionQuery>,
    State(app_state): State<AppState>,
) -> Result<Html<String>, Redirect> {
    // 验证账本所有权
    let account_book = match AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await {
        Ok(Some(book)) => AccountBookDisplay::from(book),
        _ => return Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
    };

    // 获取交易信息（包含分类信息）
    let transaction = match Transaction::find_by_account_book_with_category(&app_state.db_pool, account_book_id, 1000, 0).await {
        Ok(transactions) => {
            transactions.into_iter()
                .find(|t| t.id == transaction_id)
                .map(TransactionDisplay::from)
        }
        _ => None,
    };

    let transaction = match transaction {
        Some(t) => t,
        None => return Err(Redirect::to(&format!("/account-books/{}/transactions?error=交易记录不存在", account_book_id))),
    };

    // 获取账本的分类
    let categories = Category::find_by_account_book(&app_state.db_pool, account_book_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(CategoryDisplay::from)
        .collect();

    let template = EditTransactionTemplate {
        user,
        account_book,
        transaction,
        categories,
        error: query.error.unwrap_or_default(),
    };

    Ok(Html(template.render().unwrap()))
}

// 更新交易
pub async fn update(
    user: CurrentUser,
    Path((account_book_id, transaction_id)): Path<(i64, i64)>,
    State(app_state): State<AppState>,
    Form(form): Form<UpdateTransactionForm>,
) -> Redirect {
    // 验证账本所有权
    if AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await.is_err() {
        return Redirect::to("/account-books?error=账本不存在或无权限访问");
    }

    // 验证交易是否存在且属于该账本
    match Transaction::find_by_id(&app_state.db_pool, transaction_id).await {
        Ok(Some(transaction)) if transaction.account_book_id == account_book_id => {},
        _ => return Redirect::to(&format!("/account-books/{}/transactions?error=交易记录不存在", account_book_id)),
    }

    // 验证并解析金额
    let amount = match form.amount.parse::<Decimal>() {
        Ok(amt) if amt > Decimal::ZERO => amt,
        _ => return Redirect::to(&format!("/account-books/{}/transactions/{}/edit?error=金额必须大于0", account_book_id, transaction_id)),
    };

    // 验证并解析日期
    let transaction_date = match NaiveDate::parse_from_str(&form.transaction_date, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return Redirect::to(&format!("/account-books/{}/transactions/{}/edit?error=日期格式错误", account_book_id, transaction_id)),
    };

    // 验证分类是否存在且属于该账本
    match Category::find_by_id(&app_state.db_pool, form.category_id).await {
        Ok(Some(category)) if category.account_book_id == account_book_id => {},
        _ => return Redirect::to(&format!("/account-books/{}/transactions/{}/edit?error=分类不存在或无权限访问", account_book_id, transaction_id)),
    }

    match Transaction::update(
        &app_state.db_pool,
        transaction_id,
        form.category_id,
        amount,
        form.description.as_deref(),
        transaction_date,
        form.tags.as_deref(),
    ).await {
        Ok(_) => Redirect::to(&format!("/account-books/{}/transactions?success=交易记录更新成功", account_book_id)),
        Err(_) => Redirect::to(&format!("/account-books/{}/transactions/{}/edit?error=更新交易记录失败", account_book_id, transaction_id)),
    }
}

// 删除交易
pub async fn delete(
    user: CurrentUser,
    Path((account_book_id, transaction_id)): Path<(i64, i64)>,
    State(app_state): State<AppState>,
) -> Redirect {
    // 验证账本所有权
    if AccountBook::find_by_id(&app_state.db_pool, account_book_id, user.id).await.is_err() {
        return Redirect::to("/account-books?error=账本不存在或无权限访问");
    }

    // 验证交易是否存在且属于该账本
    match Transaction::find_by_id(&app_state.db_pool, transaction_id).await {
        Ok(Some(transaction)) if transaction.account_book_id == account_book_id => {},
        _ => return Redirect::to(&format!("/account-books/{}/transactions?error=交易记录不存在", account_book_id)),
    }

    match Transaction::delete(&app_state.db_pool, transaction_id).await {
        Ok(_) => Redirect::to(&format!("/account-books/{}/transactions?success=交易记录删除成功", account_book_id)),
        Err(_) => Redirect::to(&format!("/account-books/{}/transactions?error=删除交易记录失败", account_book_id)),
    }
}