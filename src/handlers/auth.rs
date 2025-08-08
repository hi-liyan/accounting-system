use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar};
use askama::Template;
use serde::Deserialize;
use validator::Validate;

use crate::middleware::AppState;
use crate::utils::{RegisterForm, LoginForm};

#[derive(Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate {
    error: String,
    success: String,
}

#[derive(Template)]
#[template(path = "auth/register.html")]
struct RegisterTemplate {
    error: String,
    success: String,
}

#[derive(Template)]
#[template(path = "auth/verify_email.html")]
struct VerifyEmailTemplate {
    message: String,
    is_success: bool,
}

#[derive(Deserialize)]
pub struct AuthQuery {
    error: Option<String>,
    success: Option<String>,
}

pub async fn show_login(Query(query): Query<AuthQuery>) -> Html<String> {
    let template = LoginTemplate {
        error: query.error.unwrap_or_default(),
        success: query.success.unwrap_or_default(),
    };
    Html(template.render().unwrap())
}

pub async fn show_register(Query(query): Query<AuthQuery>) -> Html<String> {
    let template = RegisterTemplate {
        error: query.error.unwrap_or_default(),
        success: query.success.unwrap_or_default(),
    };
    Html(template.render().unwrap())
}

pub async fn login(
    State(app_state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Result<(CookieJar, Redirect), Redirect> {
    // 验证表单
    if let Err(errors) = form.validate() {
        let error_msg = errors
            .field_errors()
            .iter()
            .flat_map(|(_, v)| v.iter())
            .next()
            .map(|e| e.message.as_ref().unwrap_or(&"验证失败".into()).to_string())
            .unwrap_or_else(|| "输入数据无效".to_string());
        return Err(Redirect::to(&format!("/auth/login?error={}", 
            urlencoding::encode(&error_msg))));
    }

    match app_state
        .auth_service
        .login(&app_state.db_pool, form.email, form.password)
        .await
    {
        Ok((_user, token)) => {
            let cookie = Cookie::build(("auth_token", token))
                .path("/")
                .http_only(true)
                .max_age(time::Duration::days(30))
                .build();
            
            Ok((jar.add(cookie), Redirect::to("/dashboard")))
        }
        Err(e) => {
            let error_msg = e.to_string();
            Err(Redirect::to(&format!("/auth/login?error={}", 
                urlencoding::encode(&error_msg))))
        }
    }
}

pub async fn register(
    State(app_state): State<AppState>,
    Form(form): Form<RegisterForm>,
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
        return Redirect::to(&format!("/auth/register?error={}", 
            urlencoding::encode(&final_error)));
    }

    match app_state
        .auth_service
        .register(&app_state.db_pool, form.email, form.password)
        .await
    {
        Ok(()) => Redirect::to("/auth/register?success=注册成功！如果您提供了正确的邮箱地址，我们已向您发送验证链接"),
        Err(e) => {
            let error_msg = e.to_string();
            Redirect::to(&format!("/auth/register?error={}", 
                urlencoding::encode(&error_msg)))
        }
    }
}

pub async fn verify_email(
    State(app_state): State<AppState>,
    Path(token): Path<String>,
) -> Html<String> {
    match app_state.auth_service.verify_email(&app_state.db_pool, &token).await {
        Ok(()) => {
            let template = VerifyEmailTemplate {
                message: "邮箱验证成功！现在您可以登录了。".to_string(),
                is_success: true,
            };
            Html(template.render().unwrap())
        }
        Err(e) => {
            let template = VerifyEmailTemplate {
                message: format!("验证失败：{}", e),
                is_success: false,
            };
            Html(template.render().unwrap())
        }
    }
}

pub async fn logout(jar: CookieJar) -> (CookieJar, Redirect) {
    // 创建一个过期的cookie来清除auth_token
    let cookie = Cookie::build(("auth_token", ""))
        .path("/")
        .max_age(time::Duration::seconds(0))
        .secure(false) // 在开发环境中设置为false，生产环境应该为true
        .http_only(true) // 防止XSS攻击
        .build();
    
    // 重定向到登录页面并显示成功消息
    (jar.add(cookie), Redirect::to("/auth/login?success=已成功退出登录"))
}

pub async fn resend_verification(
    State(app_state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Redirect {
    match app_state
        .auth_service
        .resend_verification(&app_state.db_pool, form.email)
        .await
    {
        Ok(()) => Redirect::to("/auth/login?success=验证邮件已重新发送"),
        Err(e) => {
            let error_msg = e.to_string();
            Redirect::to(&format!("/auth/login?error={}", 
                urlencoding::encode(&error_msg)))
        }
    }
}