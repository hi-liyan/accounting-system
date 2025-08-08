use axum::{
    extract::{Query, State},
    response::{Html, Redirect},
};
use askama::Template;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, Datelike};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::middleware::{CurrentUser, OptionalCurrentUser, AppState};
use crate::models::{AccountBook, Transaction, TransactionWithCategory, User};

#[derive(Template)]
#[template(path = "dashboard/index.html")]
struct DashboardTemplate {
    user: CurrentUser,
    account_books: Vec<AccountBookDisplay>,
    selected_book: Option<AccountBookDisplay>,
    monthly_stats: MonthlyStats,
    recent_transactions: Vec<TransactionDisplay>,
    category_stats: Vec<CategoryStat>,
    success: String,
    error: String,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    user: Option<CurrentUser>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AccountBookDisplay {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub currency: String,
}

#[derive(Debug, Serialize)]
pub struct MonthlyStats {
    pub income: Decimal,
    pub expense: Decimal,
    pub balance: Decimal,
    pub transaction_count: i64,
    pub month_name: String,
    pub is_positive: bool,
}

#[derive(Debug, Serialize)]
pub struct TransactionDisplay {
    pub id: i64,
    pub amount: Decimal,
    pub transaction_type: String,
    pub description: String,
    pub transaction_date: NaiveDate,
    pub category_name: String,
    pub category_icon: String,
    pub category_color: String,
}

#[derive(Debug, Serialize)]
pub struct CategoryStat {
    pub name: String,
    pub amount: Decimal,
    pub count: i64,
    pub percentage: f64,
    pub color: String,
    pub icon: String,
}

#[derive(Deserialize)]
pub struct DashboardQuery {
    book_id: Option<i64>,
    success: Option<String>,
    error: Option<String>,
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

impl From<TransactionWithCategory> for TransactionDisplay {
    fn from(t: TransactionWithCategory) -> Self {
        Self {
            id: t.id,
            amount: t.amount,
            transaction_type: t.transaction_type,
            description: t.description.unwrap_or_default(),
            transaction_date: t.transaction_date,
            category_name: t.category_name,
            category_icon: t.category_icon.unwrap_or("tag".to_string()),
            category_color: t.category_color.unwrap_or("#007bff".to_string()),
        }
    }
}

pub async fn dashboard(
    user: CurrentUser,
    State(app_state): State<AppState>,
    Query(query): Query<DashboardQuery>,
) -> Result<Html<String>, Redirect> {
    // 获取用户的所有账本
    let account_books = match AccountBook::find_by_user(&app_state.db_pool, user.id).await {
        Ok(books) => books.into_iter().map(AccountBookDisplay::from).collect(),
        Err(_) => Vec::new(),
    };

    // 获取用户完整信息（包含last_selected_account_book_id）
    let user_info = match User::find_by_id(&app_state.db_pool, user.id).await {
        Ok(Some(u)) => u,
        _ => return Err(Redirect::to("/auth/login?error=用户信息获取失败")),
    };

    // 确定选中的账本（优先级：URL参数 > 数据库偏好 > 第一个账本）
    let selected_book = if let Some(book_id) = query.book_id {
        // 1. URL参数指定的账本
        match AccountBook::find_by_id(&app_state.db_pool, book_id, user.id).await {
            Ok(Some(book)) => {
                // 异步更新用户偏好到数据库（不阻塞响应）
                let _ = User::update_last_selected_account_book(&app_state.db_pool, user.id, Some(book_id)).await;
                Some(AccountBookDisplay::from(book))
            }
            _ => {
                // URL参数指定的账本无效，使用数据库偏好或第一个账本
                get_fallback_account_book(&account_books, &user_info).await
            }
        }
    } else if let Some(last_book_id) = user_info.last_selected_account_book_id {
        // 2. 数据库中存储的用户偏好
        match AccountBook::find_by_id(&app_state.db_pool, last_book_id, user.id).await {
            Ok(Some(book)) => Some(AccountBookDisplay::from(book)),
            _ => {
                // 偏好的账本已不存在，使用第一个账本并更新偏好
                let fallback = account_books.first().cloned();
                if let Some(ref book) = fallback {
                    let _ = User::update_last_selected_account_book(&app_state.db_pool, user.id, Some(book.id)).await;
                }
                fallback
            }
        }
    } else {
        // 3. 使用第一个账本作为默认选择
        let fallback = account_books.first().cloned();
        if let Some(ref book) = fallback {
            let _ = User::update_last_selected_account_book(&app_state.db_pool, user.id, Some(book.id)).await;
        }
        fallback
    };

    // 获取统计数据
    let (monthly_stats, recent_transactions, category_stats) = if let Some(ref book) = selected_book {
        let stats = get_monthly_stats(&app_state.db_pool, book.id).await;
        let transactions = get_recent_transactions(&app_state.db_pool, book.id).await;
        let cat_stats = get_category_stats(&app_state.db_pool, book.id).await;
        (stats, transactions, cat_stats)
    } else {
        (
            MonthlyStats {
                income: Decimal::ZERO,
                expense: Decimal::ZERO,
                balance: Decimal::ZERO,
                transaction_count: 0,
                month_name: get_current_month_name(),
                is_positive: true,
            },
            Vec::new(),
            Vec::new(),
        )
    };

    let template = DashboardTemplate {
        user,
        account_books,
        selected_book,
        monthly_stats,
        recent_transactions,
        category_stats,
        success: query.success.unwrap_or_default(),
        error: query.error.unwrap_or_default(),
    };

    Ok(Html(template.render().unwrap()))
}

// 获取备选账本（用于用户偏好账本不存在时的回退逻辑）
async fn get_fallback_account_book(
    account_books: &[AccountBookDisplay], 
    user_info: &User
) -> Option<AccountBookDisplay> {
    if let Some(last_book_id) = user_info.last_selected_account_book_id {
        // 检查偏好的账本是否仍在账本列表中
        if let Some(book) = account_books.iter().find(|b| b.id == last_book_id) {
            return Some(book.clone());
        }
    }
    // 返回第一个账本作为默认选择
    account_books.first().cloned()
}

async fn get_monthly_stats(pool: &crate::database::DbPool, account_book_id: i64) -> MonthlyStats {
    let now = chrono::Utc::now().naive_utc().date();
    let start_date = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    let end_date = if now.month() == 12 {
        NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap() - chrono::Duration::days(1)
    } else {
        NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap() - chrono::Duration::days(1)
    };

    let (income, expense) = Transaction::get_monthly_summary(pool, account_book_id, start_date, end_date)
        .await
        .unwrap_or((Decimal::ZERO, Decimal::ZERO));

    let transaction_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM transactions WHERE account_book_id = ? AND transaction_date BETWEEN ? AND ?"
    )
    .bind(account_book_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    MonthlyStats {
        income,
        expense,
        balance: income - expense,
        transaction_count,
        month_name: get_current_month_name(),
        is_positive: income >= expense,
    }
}

async fn get_recent_transactions(pool: &crate::database::DbPool, account_book_id: i64) -> Vec<TransactionDisplay> {
    Transaction::find_by_account_book_with_category(pool, account_book_id, 10, 0)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(TransactionDisplay::from)
        .collect()
}

async fn get_category_stats(pool: &crate::database::DbPool, account_book_id: i64) -> Vec<CategoryStat> {
    let now = chrono::Utc::now().naive_utc().date();
    let start_date = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap();
    let end_date = if now.month() == 12 {
        NaiveDate::from_ymd_opt(now.year() + 1, 1, 1).unwrap() - chrono::Duration::days(1)
    } else {
        NaiveDate::from_ymd_opt(now.year(), now.month() + 1, 1).unwrap() - chrono::Duration::days(1)
    };

    let result: Vec<(String, Option<String>, Option<String>, Decimal, i64)> = sqlx::query_as(
        r#"
        SELECT c.name, c.icon, c.color, SUM(t.amount) as total_amount, COUNT(t.id) as transaction_count
        FROM categories c
        LEFT JOIN transactions t ON c.id = t.category_id 
            AND t.type = 'expense' 
            AND t.transaction_date BETWEEN ? AND ?
        WHERE c.account_book_id = ? AND c.type = 'expense' AND c.is_active = TRUE
        GROUP BY c.id, c.name, c.icon, c.color
        HAVING total_amount > 0
        ORDER BY total_amount DESC
        LIMIT 10
        "#
    )
    .bind(start_date)
    .bind(end_date)
    .bind(account_book_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let total_expense: Decimal = result.iter().map(|(_, _, _, amount, _)| *amount).sum();

    result
        .into_iter()
        .map(|(name, icon, color, amount, count)| {
            let percentage = if total_expense > Decimal::ZERO {
                (amount / total_expense * Decimal::from(100)).to_f64().unwrap_or(0.0)
            } else {
                0.0
            };

            CategoryStat {
                name,
                amount,
                count,
                percentage,
                color: color.unwrap_or_else(|| "#007bff".to_string()),
                icon: icon.unwrap_or_else(|| "tag".to_string()),
            }
        })
        .collect()
}

fn get_current_month_name() -> String {
    let now = chrono::Utc::now().naive_utc().date();
    let month_names = [
        "", "一月", "二月", "三月", "四月", "五月", "六月",
        "七月", "八月", "九月", "十月", "十一月", "十二月"
    ];
    format!("{}月", month_names.get(now.month() as usize).unwrap_or(&"未知"))
}

pub async fn index(OptionalCurrentUser(user): OptionalCurrentUser) -> Result<Html<String>, Redirect> {
    match user {
        Some(_) => Err(Redirect::to("/dashboard")),
        None => {
            let template = IndexTemplate { user: None };
            Ok(Html(template.render().unwrap()))
        }
    }
}