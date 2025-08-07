use validator::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterForm {
    #[validate(email(message = "请输入有效的邮箱地址"))]
    pub email: String,
    
    #[validate(length(min = 6, message = "密码长度至少6个字符"))]
    pub password: String,
    
    #[validate(must_match(other = "password", message = "两次输入的密码不一致"))]
    pub confirm_password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginForm {
    #[validate(email(message = "请输入有效的邮箱地址"))]
    pub email: String,
    
    #[validate(length(min = 1, message = "请输入密码"))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct AccountBookForm {
    #[validate(length(min = 1, max = 100, message = "账本名称长度必须在1-100字符之间"))]
    pub name: String,
    
    #[validate(length(max = 500, message = "描述长度不能超过500字符"))]
    pub description: Option<String>,
    
    #[validate(length(equal = 3, message = "货币代码必须为3个字符"))]
    pub currency: String,
    
    #[validate(range(min = 1, max = 31, message = "起始日必须在1-31之间"))]
    pub cycle_start_day: i32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CategoryForm {
    #[validate(length(min = 1, max = 50, message = "分类名称长度必须在1-50字符之间"))]
    pub name: String,
    
    pub category_type: String,
    
    pub icon: Option<String>,
    
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct TransactionForm {
    pub category_id: i64,
    
    pub amount: rust_decimal::Decimal,
    
    pub transaction_type: String,
    
    #[validate(length(max = 500, message = "描述长度不能超过500字符"))]
    pub description: Option<String>,
    
    pub transaction_date: chrono::NaiveDate,
    
    #[validate(length(max = 500, message = "标签长度不能超过500字符"))]
    pub tags: Option<String>,
}