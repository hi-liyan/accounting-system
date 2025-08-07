use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AccountBook {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub cycle_start_day: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountBook {
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub currency: String,
    pub cycle_start_day: i32,
}

impl AccountBook {
    pub async fn create(
        pool: &crate::database::DbPool,
        create_book: CreateAccountBook,
    ) -> anyhow::Result<AccountBook> {
        let result = sqlx::query(
            r#"
            INSERT INTO account_books (user_id, name, description, currency, cycle_start_day)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(create_book.user_id)
        .bind(&create_book.name)
        .bind(&create_book.description)
        .bind(&create_book.currency)
        .bind(create_book.cycle_start_day)
        .execute(pool)
        .await?;

        let book_id = result.last_insert_id() as i64;
        let book = Self::find_by_id(pool, book_id, create_book.user_id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created account book"))?;

        Ok(book)
    }

    pub async fn find_by_user(
        pool: &crate::database::DbPool,
        user_id: i64,
    ) -> anyhow::Result<Vec<AccountBook>> {
        let books = sqlx::query_as::<_, AccountBook>(
            "SELECT * FROM account_books WHERE user_id = ? AND is_active = TRUE ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(books)
    }

    pub async fn find_by_id(
        pool: &crate::database::DbPool,
        id: i64,
        user_id: i64,
    ) -> anyhow::Result<Option<AccountBook>> {
        let book = sqlx::query_as::<_, AccountBook>(
            "SELECT * FROM account_books WHERE id = ? AND user_id = ? AND is_active = TRUE"
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(book)
    }

    pub async fn update(
        pool: &crate::database::DbPool,
        id: i64,
        user_id: i64,
        name: &str,
        description: Option<&str>,
        currency: &str,
        cycle_start_day: i32,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"
            UPDATE account_books 
            SET name = ?, description = ?, currency = ?, cycle_start_day = ?, updated_at = NOW()
            WHERE id = ? AND user_id = ?
            "#,
        )
        .bind(name)
        .bind(description)
        .bind(currency)
        .bind(cycle_start_day)
        .bind(id)
        .bind(user_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(
        pool: &crate::database::DbPool,
        id: i64,
        user_id: i64,
    ) -> anyhow::Result<()> {
        sqlx::query("UPDATE account_books SET is_active = FALSE WHERE id = ? AND user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }
}