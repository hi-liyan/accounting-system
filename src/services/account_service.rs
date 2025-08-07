use crate::models::{AccountBook, CreateAccountBook};
use crate::database::DbPool;
use anyhow::Result;

pub struct AccountService;

impl AccountService {
    pub async fn create_account_book(
        pool: &DbPool,
        user_id: i64,
        name: String,
        description: Option<String>,
        currency: String,
        cycle_start_day: i32,
    ) -> Result<AccountBook> {
        let create_book = CreateAccountBook {
            user_id,
            name,
            description,
            currency,
            cycle_start_day,
        };

        AccountBook::create(pool, create_book).await
    }

    pub async fn get_user_account_books(
        pool: &DbPool,
        user_id: i64,
    ) -> Result<Vec<AccountBook>> {
        AccountBook::find_by_user(pool, user_id).await
    }

    pub async fn get_account_book(
        pool: &DbPool,
        id: i64,
        user_id: i64,
    ) -> Result<Option<AccountBook>> {
        AccountBook::find_by_id(pool, id, user_id).await
    }

    pub async fn update_account_book(
        pool: &DbPool,
        id: i64,
        user_id: i64,
        name: String,
        description: Option<String>,
        currency: String,
        cycle_start_day: i32,
    ) -> Result<()> {
        AccountBook::update(
            pool,
            id,
            user_id,
            &name,
            description.as_deref(),
            &currency,
            cycle_start_day,
        ).await
    }

    pub async fn delete_account_book(
        pool: &DbPool,
        id: i64,
        user_id: i64,
    ) -> Result<()> {
        AccountBook::delete(pool, id, user_id).await
    }
}