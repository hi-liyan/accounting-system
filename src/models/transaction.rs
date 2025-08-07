use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc, NaiveDate};
use rust_decimal::Decimal;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Transaction {
    pub id: i64,
    pub account_book_id: i64,
    pub category_id: i64,
    pub amount: Decimal,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub description: Option<String>,
    pub transaction_date: NaiveDate,
    pub tags: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TransactionWithCategory {
    pub id: i64,
    pub account_book_id: i64,
    pub category_id: i64,
    pub amount: Decimal,
    #[serde(rename = "type")]
    pub transaction_type: String,
    pub description: Option<String>,
    pub transaction_date: NaiveDate,
    pub tags: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub category_name: String,
    pub category_icon: Option<String>,
    pub category_color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTransaction {
    pub account_book_id: i64,
    pub category_id: i64,
    pub amount: Decimal,
    pub transaction_type: String,
    pub description: Option<String>,
    pub transaction_date: NaiveDate,
    pub tags: Option<String>,
}

impl Transaction {
    pub async fn create(
        pool: &crate::database::DbPool,
        create_transaction: CreateTransaction,
    ) -> anyhow::Result<Transaction> {
        let transaction = sqlx::query_as::<_, Transaction>(
            r#"
            INSERT INTO transactions (account_book_id, category_id, amount, type, description, transaction_date, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(create_transaction.account_book_id)
        .bind(create_transaction.category_id)
        .bind(create_transaction.amount)
        .bind(&create_transaction.transaction_type)
        .bind(&create_transaction.description)
        .bind(create_transaction.transaction_date)
        .bind(&create_transaction.tags)
        .fetch_one(pool)
        .await?;

        Ok(transaction)
    }

    pub async fn find_by_account_book_with_category(
        pool: &crate::database::DbPool,
        account_book_id: i64,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<TransactionWithCategory>> {
        let transactions = sqlx::query_as::<_, TransactionWithCategory>(
            r#"
            SELECT t.*, c.name as category_name, c.icon as category_icon, c.color as category_color
            FROM transactions t
            JOIN categories c ON t.category_id = c.id
            WHERE t.account_book_id = ?
            ORDER BY t.transaction_date DESC, t.created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(account_book_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(transactions)
    }

    pub async fn find_by_date_range_with_category(
        pool: &crate::database::DbPool,
        account_book_id: i64,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> anyhow::Result<Vec<TransactionWithCategory>> {
        let transactions = sqlx::query_as::<_, TransactionWithCategory>(
            r#"
            SELECT t.*, c.name as category_name, c.icon as category_icon, c.color as category_color
            FROM transactions t
            JOIN categories c ON t.category_id = c.id
            WHERE t.account_book_id = ? AND t.transaction_date BETWEEN ? AND ?
            ORDER BY t.transaction_date DESC, t.created_at DESC
            "#,
        )
        .bind(account_book_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;

        Ok(transactions)
    }

    pub async fn find_by_id(
        pool: &crate::database::DbPool,
        id: i64,
    ) -> anyhow::Result<Option<Transaction>> {
        let transaction = sqlx::query_as::<_, Transaction>("SELECT * FROM transactions WHERE id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;

        Ok(transaction)
    }

    pub async fn update(
        pool: &crate::database::DbPool,
        id: i64,
        category_id: i64,
        amount: Decimal,
        description: Option<&str>,
        transaction_date: NaiveDate,
        tags: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE transactions 
            SET category_id = ?, amount = ?, description = ?, transaction_date = ?, tags = ?, updated_at = NOW()
            WHERE id = ?
            "#,
        )
        .bind(category_id)
        .bind(amount)
        .bind(description)
        .bind(transaction_date)
        .bind(tags)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &crate::database::DbPool, id: i64) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM transactions WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_monthly_summary(
        pool: &crate::database::DbPool,
        account_book_id: i64,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> anyhow::Result<(Decimal, Decimal)> {
        let row: Option<(Option<Decimal>, Option<Decimal>)> = sqlx::query_as(
            r#"
            SELECT 
                SUM(CASE WHEN type = 'income' THEN amount ELSE 0 END) as total_income,
                SUM(CASE WHEN type = 'expense' THEN amount ELSE 0 END) as total_expense
            FROM transactions 
            WHERE account_book_id = ? AND transaction_date BETWEEN ? AND ?
            "#,
        )
        .bind(account_book_id)
        .bind(start_date)
        .bind(end_date)
        .fetch_optional(pool)
        .await?;

        let (income, expense) = row.unwrap_or((None, None));
        Ok((
            income.unwrap_or(Decimal::ZERO),
            expense.unwrap_or(Decimal::ZERO),
        ))
    }
}