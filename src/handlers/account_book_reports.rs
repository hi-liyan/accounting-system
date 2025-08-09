use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect},
};
use askama::Template;
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, Datelike};
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

#[derive(Debug, Serialize)]
pub struct MonthlyDetail {
    pub date: String,
    pub income: Decimal,
    pub expense: Decimal,
    pub balance: Decimal,
    pub is_positive_balance: bool,  // 预计算是否为正
    pub transaction_count: i64,
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
    monthly_details: Vec<MonthlyDetail>,
    total_income: Decimal,
    total_expense: Decimal,
    net_balance: Decimal,  // 预计算的净收支
    net_balance_abs: Decimal,  // 净收支的绝对值
    is_positive_balance: bool,  // 是否为正收支
    average_daily_expense: Decimal,
    start_date: String,
    end_date: String,
    error: String,
}

#[derive(Deserialize)]
pub struct ReportsQuery {
    error: Option<String>,
    start_date: Option<String>, // 起始日期 YYYY-MM-DD
    end_date: Option<String>,   // 结束日期 YYYY-MM-DD
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

    // 根据参数确定日期范围，默认使用账本的周期数据（基于起始日）
    let today = chrono::Utc::now().naive_utc().date();
    let (start_date, end_date) = match (query.start_date.as_ref(), query.end_date.as_ref()) {
        (Some(start), Some(end)) => {
            // 用户指定了起始和结束日期
            let start = NaiveDate::parse_from_str(start, "%Y-%m-%d")
                .map_err(|_| Redirect::to(&format!("/account-books/{}/reports?error=起始日期格式错误", id)))?;
            let end = NaiveDate::parse_from_str(end, "%Y-%m-%d")
                .map_err(|_| Redirect::to(&format!("/account-books/{}/reports?error=结束日期格式错误", id)))?;
            (start, end)
        },
        _ => {
            // 默认显示基于账本起始日的当前周期数据
            let cycle_start_day = book.cycle_start_day;
            let (start, end) = calculate_current_cycle_dates(today, cycle_start_day);
            (start, end)
        }
    };

    // 获取月度趋势数据（基于账本起始日）
    let monthly_trends = get_monthly_trends(&app_state.db_pool, id, book.cycle_start_day, start_date, end_date).await;

    // 获取分类统计数据
    let expense_categories = get_category_stats(&app_state.db_pool, id, "expense", start_date, end_date).await;
    let income_categories = get_category_stats(&app_state.db_pool, id, "income", start_date, end_date).await;

    // 获取每日支出数据（基于选择的时间范围）
    let daily_expenses = get_daily_expenses(&app_state.db_pool, id, start_date, end_date).await;

    // 获取月度收支明细（基于账本起始日）
    let monthly_details = get_monthly_details(&app_state.db_pool, id, book.cycle_start_day, start_date, end_date).await;

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

    // 预计算净收支相关数值
    let net_balance = total_income - total_expense;
    let is_positive_balance = total_income >= total_expense;
    let net_balance_abs = net_balance.abs(); // 绝对值用于显示

    let template = AccountBookReportsTemplate {
        user,
        book,
        monthly_trends,
        expense_categories,
        income_categories,
        daily_expenses,
        monthly_details,
        total_income,
        total_expense,
        net_balance,
        net_balance_abs,
        is_positive_balance,
        average_daily_expense,
        start_date: start_date.format("%Y-%m-%d").to_string(),
        end_date: end_date.format("%Y-%m-%d").to_string(),
        error: query.error.unwrap_or_default(),
    };

    Ok(Html(template.render().unwrap()))
}

// 获取月度趋势数据（基于账本起始日的月度周期）
async fn get_monthly_trends(
    pool: &crate::database::DbPool,
    account_book_id: i64,
    cycle_start_day: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Vec<MonthlyTrend> {
    // 获取所有交易数据
    let rows: Vec<(NaiveDate, String, Option<Decimal>)> = sqlx::query_as(
        r#"
        SELECT 
            transaction_date,
            `type`,
            amount
        FROM transactions 
        WHERE account_book_id = ? AND transaction_date BETWEEN ? AND ?
        ORDER BY transaction_date
        "#,
    )
    .bind(account_book_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 按照账本起始日分组统计
    let mut monthly_data: std::collections::BTreeMap<String, (Decimal, Decimal)> = std::collections::BTreeMap::new();
    
    for (transaction_date, transaction_type, amount) in rows {
        let amount = amount.unwrap_or(Decimal::ZERO);
        
        // 计算这个交易日期属于哪个账本月度周期
        let cycle_period = calculate_cycle_period(transaction_date, cycle_start_day);
        
        let entry = monthly_data.entry(cycle_period).or_insert((Decimal::ZERO, Decimal::ZERO));
        
        match transaction_type.as_str() {
            "income" => entry.0 += amount,
            "expense" => entry.1 += amount,
            _ => {}
        }
    }
    
    // 转换为返回格式
    monthly_data.into_iter()
        .map(|(period, (income, expense))| MonthlyTrend {
            month: period,
            income,
            expense,
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

// 获取月度收支明细（基于账本起始日的月度周期）
async fn get_monthly_details(
    pool: &crate::database::DbPool,
    account_book_id: i64,
    cycle_start_day: i32,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Vec<MonthlyDetail> {
    // 获取所有交易数据
    let rows: Vec<(NaiveDate, String, Option<Decimal>)> = sqlx::query_as(
        r#"
        SELECT 
            transaction_date,
            `type`,
            amount
        FROM transactions 
        WHERE account_book_id = ? AND transaction_date BETWEEN ? AND ?
        ORDER BY transaction_date
        "#,
    )
    .bind(account_book_id)
    .bind(start_date)
    .bind(end_date)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    // 按照账本起始日分组统计
    let mut monthly_data: std::collections::BTreeMap<String, (Decimal, Decimal, i64)> = std::collections::BTreeMap::new();
    
    for (transaction_date, transaction_type, amount) in rows {
        let amount = amount.unwrap_or(Decimal::ZERO);
        
        // 计算这个交易日期属于哪个账本月度周期
        let cycle_period = calculate_cycle_period(transaction_date, cycle_start_day);
        
        let entry = monthly_data.entry(cycle_period).or_insert((Decimal::ZERO, Decimal::ZERO, 0));
        
        match transaction_type.as_str() {
            "income" => entry.0 += amount,
            "expense" => entry.1 += amount,
            _ => {}
        }
        entry.2 += 1; // 交易计数
    }
    
    // 转换为返回格式
    monthly_data.into_iter()
        .map(|(period, (income, expense, count))| {
            let balance = income - expense;
            let is_positive_balance = balance >= Decimal::ZERO;
            
            MonthlyDetail {
                date: period,
                income,
                expense,
                balance,
                is_positive_balance,
                transaction_count: count,
            }
        })
        .collect()
}

// 根据账本起始日计算当前周期的开始和结束日期
fn calculate_current_cycle_dates(today: NaiveDate, cycle_start_day: i32) -> (NaiveDate, NaiveDate) {
    let start_day = cycle_start_day as u32;
    
    // 当前月份的起始日
    let current_cycle_start = if let Some(date) = today.with_day(start_day) {
        date
    } else {
        // 如果当前月没有这个日期（如2月30号），则取当前月的最后一天
        let last_day_of_month = NaiveDate::from_ymd_opt(
            today.year(),
            today.month() + 1,
            1
        ).unwrap_or_else(|| NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap())
        - chrono::Duration::days(1);
        
        if start_day > last_day_of_month.day() {
            last_day_of_month
        } else {
            today.with_day(start_day).unwrap()
        }
    };
    
    // 如果今天还没到本月的起始日，则使用上个月的周期
    let (start_date, end_date) = if today < current_cycle_start {
        // 使用上个月的周期
        let prev_month_start = if current_cycle_start.month() == 1 {
            NaiveDate::from_ymd_opt(current_cycle_start.year() - 1, 12, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(current_cycle_start.year(), current_cycle_start.month() - 1, 1).unwrap()
        };
        
        let prev_cycle_start = if let Some(date) = prev_month_start.with_day(start_day) {
            date
        } else {
            let last_day_of_prev_month = current_cycle_start - chrono::Duration::days(1);
            if start_day > last_day_of_prev_month.day() {
                last_day_of_prev_month
            } else {
                prev_month_start.with_day(start_day).unwrap()
            }
        };
        
        let prev_cycle_end = current_cycle_start - chrono::Duration::days(1);
        (prev_cycle_start, prev_cycle_end)
    } else {
        // 使用当前月的周期
        let next_month_start = if current_cycle_start.month() == 12 {
            NaiveDate::from_ymd_opt(current_cycle_start.year() + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(current_cycle_start.year(), current_cycle_start.month() + 1, 1).unwrap()
        };
        
        let next_cycle_start = if let Some(date) = next_month_start.with_day(start_day) {
            date
        } else {
            let last_day_of_next_month = if next_month_start.month() == 12 {
                NaiveDate::from_ymd_opt(next_month_start.year() + 1, 1, 1).unwrap()
            } else {
                NaiveDate::from_ymd_opt(next_month_start.year(), next_month_start.month() + 1, 1).unwrap()
            } - chrono::Duration::days(1);
            
            if start_day > last_day_of_next_month.day() {
                last_day_of_next_month
            } else {
                next_month_start.with_day(start_day).unwrap()
            }
        };
        
        let current_cycle_end = next_cycle_start - chrono::Duration::days(1);
        (current_cycle_start, current_cycle_end)
    };
    
    (start_date, end_date)
}

// 根据交易日期和账本起始日计算该交易属于哪个账本月度周期
fn calculate_cycle_period(transaction_date: NaiveDate, cycle_start_day: i32) -> String {
    let start_day = cycle_start_day as u32;
    
    // 计算当前月的起始日
    let current_month_start = if let Some(date) = transaction_date.with_day(start_day) {
        date
    } else {
        // 如果当前月没有这个日期（如2月30号），则取当前月的最后一天作为起始日
        let last_day_of_month = NaiveDate::from_ymd_opt(
            transaction_date.year(),
            transaction_date.month() + 1,
            1
        ).unwrap_or_else(|| NaiveDate::from_ymd_opt(transaction_date.year() + 1, 1, 1).unwrap())
        - chrono::Duration::days(1);
        
        if start_day > last_day_of_month.day() {
            last_day_of_month
        } else {
            transaction_date.with_day(start_day).unwrap()
        }
    };
    
    // 如果交易日期在当前月起始日之前，则属于上一个周期
    let cycle_start = if transaction_date < current_month_start {
        // 计算上个月的起始日
        let prev_month = if transaction_date.month() == 1 {
            NaiveDate::from_ymd_opt(transaction_date.year() - 1, 12, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(transaction_date.year(), transaction_date.month() - 1, 1).unwrap()
        };
        
        if let Some(date) = prev_month.with_day(start_day) {
            date
        } else {
            let last_day_of_prev_month = current_month_start - chrono::Duration::days(1);
            if start_day > last_day_of_prev_month.day() {
                last_day_of_prev_month
            } else {
                prev_month.with_day(start_day).unwrap()
            }
        }
    } else {
        current_month_start
    };
    
    // 返回周期标识：格式为 "YYYY-MM (起始日)"
    format!("{}-{:02} ({}日起)", cycle_start.year(), cycle_start.month(), cycle_start_day)
}