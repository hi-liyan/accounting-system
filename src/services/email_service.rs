use lettre::{
    message::header::ContentType, 
    transport::smtp::authentication::Credentials, 
    Message, SmtpTransport, Transport,
};
use anyhow::Result;
use crate::config::SmtpConfig;

#[derive(Clone)]
pub struct EmailService {
    mailer: SmtpTransport,
    from_email: String,
    app_url: String,
}

impl EmailService {
    pub fn new(smtp_config: &SmtpConfig, app_url: String) -> Result<Self> {
        let creds = Credentials::new(
            smtp_config.username.clone(),
            smtp_config.password.clone(),
        );

        // 为163邮箱配置SMTP传输
        let mailer = if smtp_config.port == 465 {
            // 465端口使用SSL加密
            SmtpTransport::relay(&smtp_config.host)?
                .port(smtp_config.port)
                .credentials(creds)
                .build()
        } else {
            // 587端口使用STARTTLS加密
            SmtpTransport::starttls_relay(&smtp_config.host)?
                .port(smtp_config.port)
                .credentials(creds)
                .build()
        };

        Ok(EmailService {
            mailer,
            from_email: smtp_config.from_email.clone(),
            app_url,
        })
    }

    pub async fn send_verification_email(&self, to_email: &str, token: &str) -> Result<()> {
        let verification_url = format!("{}/auth/verify/{}", self.app_url, token);

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject("请验证您的邮箱地址")
            .header(ContentType::TEXT_HTML)
            .body(format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <meta charset="UTF-8">
                    <title>邮箱验证</title>
                </head>
                <body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
                    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
                        <h2 style="color: #007bff;">欢迎注册记账系统！</h2>
                        
                        <p>您好，</p>
                        
                        <p>感谢您注册我们的记账系统。请点击下面的链接来验证您的邮箱地址：</p>
                        
                        <div style="text-align: center; margin: 30px 0;">
                            <a href="{}" 
                               style="background-color: #007bff; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; display: inline-block;">
                                验证邮箱
                            </a>
                        </div>
                        
                        <p>或者复制以下链接到浏览器中打开：</p>
                        <p style="word-break: break-all; background-color: #f8f9fa; padding: 10px; border-radius: 5px;">
                            <a href="{}">{}</a>
                        </p>
                        
                        <p>如果您没有注册过此账户，请忽略这封邮件。</p>
                        
                        <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
                        <p style="color: #666; font-size: 12px;">
                            此邮件由系统自动发送，请勿回复。
                        </p>
                    </div>
                </body>
                </html>
                "#,
                verification_url, verification_url, verification_url
            ))?;

        self.mailer.send(&email)?;
        Ok(())
    }

    pub async fn send_password_reset_email(&self, to_email: &str, username: &str, token: &str) -> Result<()> {
        let reset_url = format!("{}/auth/reset-password/{}", self.app_url, token);

        let email = Message::builder()
            .from(self.from_email.parse()?)
            .to(to_email.parse()?)
            .subject("重置您的密码")
            .header(ContentType::TEXT_HTML)
            .body(format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <meta charset="UTF-8">
                    <title>密码重置</title>
                </head>
                <body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
                    <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
                        <h2 style="color: #dc3545;">密码重置请求</h2>
                        
                        <p>亲爱的 <strong>{}</strong>，</p>
                        
                        <p>我们收到了您的密码重置请求。请点击下面的链接来重置您的密码：</p>
                        
                        <div style="text-align: center; margin: 30px 0;">
                            <a href="{}" 
                               style="background-color: #dc3545; color: white; padding: 12px 30px; text-decoration: none; border-radius: 5px; display: inline-block;">
                                重置密码
                            </a>
                        </div>
                        
                        <p>或者复制以下链接到浏览器中打开：</p>
                        <p style="word-break: break-all; background-color: #f8f9fa; padding: 10px; border-radius: 5px;">
                            <a href="{}">{}</a>
                        </p>
                        
                        <p style="color: #856404; background-color: #fff3cd; padding: 10px; border-radius: 5px; border-left: 4px solid #ffc107;">
                            <strong>注意：</strong>如果您没有请求重置密码，请忽略这封邮件。此链接将在24小时后失效。
                        </p>
                        
                        <hr style="border: none; border-top: 1px solid #eee; margin: 30px 0;">
                        <p style="color: #666; font-size: 12px;">
                            此邮件由系统自动发送，请勿回复。
                        </p>
                    </div>
                </body>
                </html>
                "#,
                username, reset_url, reset_url, reset_url
            ))?;

        self.mailer.send(&email)?;
        Ok(())
    }
}