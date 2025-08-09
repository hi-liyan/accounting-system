use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: i64,
    pub account_book_id: i64,
    pub name: String,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub category_type: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategory {
    pub account_book_id: i64,
    pub name: String,
    pub category_type: String,
    pub icon: Option<String>,
    pub color: Option<String>,
}

impl Category {
    pub async fn create(
        pool: &crate::database::DbPool,
        create_category: CreateCategory,
    ) -> anyhow::Result<Category> {
        let sort_order = Self::get_next_sort_order(pool, create_category.account_book_id).await?;
        
        let result = sqlx::query(
            r#"
            INSERT INTO categories (account_book_id, name, `type`, icon, color, sort_order)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(create_category.account_book_id)
        .bind(&create_category.name)
        .bind(&create_category.category_type)
        .bind(&create_category.icon)
        .bind(&create_category.color)
        .bind(sort_order)
        .execute(pool)
        .await?;

        let category_id = result.last_insert_id() as i64;
        let category = Self::find_by_id(pool, category_id).await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created category"))?;

        Ok(category)
    }

    pub async fn find_by_account_book(
        pool: &crate::database::DbPool,
        account_book_id: i64,
    ) -> anyhow::Result<Vec<Category>> {
        let categories = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE account_book_id = ? AND is_active = TRUE ORDER BY sort_order, name"
        )
        .bind(account_book_id)
        .fetch_all(pool)
        .await?;

        Ok(categories)
    }

    pub async fn find_by_type(
        pool: &crate::database::DbPool,
        account_book_id: i64,
        category_type: &str,
    ) -> anyhow::Result<Vec<Category>> {
        let categories = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE account_book_id = ? AND `type` = ? AND is_active = TRUE ORDER BY sort_order, name"
        )
        .bind(account_book_id)
        .bind(category_type)
        .fetch_all(pool)
        .await?;

        Ok(categories)
    }

    pub async fn find_by_id(
        pool: &crate::database::DbPool,
        id: i64,
    ) -> anyhow::Result<Option<Category>> {
        let category = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE id = ? AND is_active = TRUE"
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(category)
    }

    pub async fn update(
        pool: &crate::database::DbPool,
        id: i64,
        name: &str,
        icon: Option<&str>,
        color: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE categories SET name = ?, icon = ?, color = ? WHERE id = ?"
        )
        .bind(name)
        .bind(icon)
        .bind(color)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &crate::database::DbPool, id: i64) -> anyhow::Result<()> {
        sqlx::query("UPDATE categories SET is_active = FALSE WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_sort_orders(
        pool: &crate::database::DbPool,
        updates: Vec<(i64, i32)>, // (category_id, new_sort_order)
    ) -> anyhow::Result<()> {
        let mut transaction = pool.begin().await?;
        
        for (category_id, sort_order) in updates {
            sqlx::query("UPDATE categories SET sort_order = ? WHERE id = ?")
                .bind(sort_order)
                .bind(category_id)
                .execute(&mut *transaction)
                .await?;
        }
        
        transaction.commit().await?;
        Ok(())
    }

    async fn get_next_sort_order(
        pool: &crate::database::DbPool,
        account_book_id: i64,
    ) -> anyhow::Result<i32> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM categories WHERE account_book_id = ?"
        )
        .bind(account_book_id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| r.0).unwrap_or(1))
    }
}