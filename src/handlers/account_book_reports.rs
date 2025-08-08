use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect},
};
use askama::Template;
use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::middleware::{AppState, CurrentUser};
use crate::models::{AccountBook, Transaction};
use crate::handlers::account_book::AccountBookDisplay;

// 统计报表相关的结构体
#[derive(Debug, Serialize)]
pub struct MonthlyTrend {
    pub month: String,
    pub income: Decimal,
    pub expense: Decimal,
}

#[derive(Debug, Serialize)]
pub struct CategoryStat {
    pub name: String,
    pub amount: Decimal,
    pub percentage: i32, // 修改为整数百分比
    pub color: String,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize)]
pub struct DailyStat {
    pub date: String,
    pub amount: Decimal,
}

#[derive(Template)]
#[template(path = "account_books/reports.html")]
struct AccountBookReportsTemplate {
    user: CurrentUser,
    book: AccountBookDisplay,
    monthly_trends: Vec<MonthlyTrend>,
    expense_categories: Vec<CategoryStat>,
    income_categories: Vec<CategoryStat>,
    daily_expenses: Vec<DailyStat>,
    total_income: Decimal,
    total_expense: Decimal,
    average_daily_expense: Decimal,
    error: String,
}

#[derive(Deserialize)]
pub struct ReportsQuery {
    error: Option<String>,
    months: Option<i32>, // 查询最近几个月的数据，默认为6个月
}

// 查看账本统计报表
pub async fn reports(
    user: CurrentUser,
    Path(id): Path<i64>,
    State(app_state): State<AppState>,
    Query(query): Query<ReportsQuery>,
) -> Result<Html<String>, Redirect> {
    // 验证账本所有权
    let book = match AccountBook::find_by_id(&app_state.db_pool, id, user.id).await {
        Ok(Some(book)) => AccountBookDisplay::from(book),
        _ => return Err(Redirect::to("/account-books?error=账本不存在或无权限访问")),
    };

    let months = query.months.unwrap_or(6);
    let end_date = chrono::Utc::now().naive_utc().date();
    let start_date = end_date - chrono::Duration::days(months as i64 * 30);

    // 获取月度趋势数据
    let monthly_trends = get_monthly_trends(&app_state.db_pool, id, start_date, end_date).await;

    // 获取分类统计数据
    let expense_categories = get_category_stats(&app_state.db_pool, id, "expense", start_date, end_date).await;
    let income_categories = get_category_stats(&app_state.db_pool, id, "income", start_date, end_date).await;

    // 获取每日支出数据（最近30天）
    let recent_start = end_date - chrono::Duration::days(30);
    let daily_expenses = get_daily_expenses(&app_state.db_pool, id, recent_start, end_date).await;

    // 获取总体统计
    let (total_income, total_expense) = Transaction::get_monthly_summary(
        &app_state.db_pool,
        id,
        start_date,
        end_date,
    ).await.unwrap_or((Decimal::ZERO, Decimal::ZERO));

    // 计算日均支出，保留两位小数
    let days_count = (end_date - start_date).num_days() as i64;
    let average_daily_expense = if days_count > 0 {
        (total_expense / Decimal::from(days_count)).round_dp(2)
    } else {
        Decimal::ZERO
    };

    let template = AccountBookReportsTemplate {
        user,
        book,
        monthly_trends,
        expense_categories,
        income_categories,
        daily_expenses,
        total_income,
        total_expense,
        average_daily_expense,
        error: query.error.unwrap_or_default(),
    };

    Ok(Html(template.render().unwrap()))
}

// 获取月度趋势数据
async fn get_monthly_trends(
    pool: &crate::database::DbPool,
    account_book_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Vec<MonthlyTrend> {
    let rows: Vec<(i32, i32, Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
        r#"
        SELECT 
            YEAR(transaction_date) as year,
            MONTH(transaction_date) as month,
            SUM(CASE WHEN `type` = 'income' THEN amount ELSE 0 END) as income,
            SUM(CASE WHEN `type` = 'expense' THEN amount ELSE 0 END) as expense
        FROM transactions 
        WHERE account_book_id = ? AND transaction_date BETWEEN ? AND ?
        GROUP BY YEAR(transaction_date), MONTH(transaction_date)
        ORDER BY year, month
        "#,
    )
    .bind(account_book_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|(year, month, income, expense)| MonthlyTrend {
            month: format!("{}-{:02}", year, month),
            income: income.unwrap_or(Decimal::ZERO),
            expense: expense.unwrap_or(Decimal::ZERO),
        })
        .collect()
}

// 获取分类统计数据
async fn get_category_stats(
    pool: &crate::database::DbPool,
    account_book_id: i64,
    category_type: &str,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Vec<CategoryStat> {
    let rows: Vec<(String, Option<Decimal>, i64, Option<String>)> = sqlx::query_as(
        r#"
        SELECT 
            c.name,
            SUM(t.amount) as total_amount,
            COUNT(t.id) as transaction_count,
            c.color
        FROM categories c
        LEFT JOIN transactions t ON c.id = t.category_id 
            AND t.transaction_date BETWEEN ? AND ?
        WHERE c.account_book_id = ? AND c.`type` = ? AND c.is_active = TRUE
        GROUP BY c.id, c.name, c.color
        HAVING total_amount > 0
        ORDER BY total_amount DESC
        LIMIT 10
        "#,
    )
    .bind(start_date)
    .bind(end_date)
    .bind(account_book_id)
    .bind(category_type)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let total: Decimal = rows.iter()
        .map(|(_, amount, _, _)| amount.unwrap_or(Decimal::ZERO))
        .sum();

    rows.into_iter()
        .map(|(name, amount, count, color)| {
            let amount = amount.unwrap_or(Decimal::ZERO);
            let percentage = if total > Decimal::ZERO {
                ((amount / total * Decimal::from(100)).to_f64().unwrap_or(0.0)).round() as i32
            } else {
                0
            };

            CategoryStat {
                name,
                amount,
                percentage,
                color: color.unwrap_or("#007bff".to_string()),
                transaction_count: count,
            }
        })
        .collect()
}

// 获取每日支出数据
async fn get_daily_expenses(
    pool: &crate::database::DbPool,
    account_book_id: i64,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Vec<DailyStat> {
    let rows: Vec<(NaiveDate, Option<Decimal>)> = sqlx::query_as(
        r#"
        SELECT 
            transaction_date,
            SUM(amount) as daily_amount
        FROM transactions 
        WHERE account_book_id = ? AND `type` = 'expense' 
            AND transaction_date BETWEEN ? AND ?
        GROUP BY transaction_date
        ORDER BY transaction_date
        "#,
    )
    .bind(account_book_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|(date, amount)| DailyStat {
            date: date.format("%m-%d").to_string(),
            amount: amount.unwrap_or(Decimal::ZERO),
        })
        .collect()
}