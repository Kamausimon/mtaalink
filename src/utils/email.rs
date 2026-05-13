use crate::errors::{AppError, AppResult};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::authentication::Credentials,
};
use std::env;

pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_user: String,
    pub smtp_password: String,
    pub from_email: String,
    pub from_name: String,
}

impl EmailConfig {
    pub fn from_env() -> AppResult<Self> {
        Ok(Self {
            smtp_host: env::var("SMTP_HOST")
                .map_err(|_| AppError::Internal("SMTP_HOST not set".to_string()))?,
            smtp_port: env::var("SMTP_PORT")
                .unwrap_or_else(|_| "587".to_string())
                .parse::<u16>()
                .map_err(|_| AppError::Internal("Invalid SMTP_PORT".to_string()))?,
            smtp_user: env::var("SMTP_USER")
                .map_err(|_| AppError::Internal("SMTP_USER not set".to_string()))?,
            smtp_password: env::var("SMTP_PASSWORD")
                .map_err(|_| AppError::Internal("SMTP_PASSWORD not set".to_string()))?,
            from_email: env::var("FROM_EMAIL")
                .unwrap_or_else(|_| "noreply@mtaalink.com".to_string()),
            from_name: env::var("FROM_NAME")
                .unwrap_or_else(|_| "MtaaLink".to_string()),
        })
    }
}

pub async fn send_email(
    config: &EmailConfig,
    to_address: &str,
    subject: &str,
    html_body: &str,
) -> AppResult<()> {
    let from: Mailbox = format!("{} <{}>", config.from_name, config.from_email)
        .parse()
        .map_err(|e: lettre::address::AddressError| AppError::Internal(e.to_string()))?;

    let to: Mailbox = to_address
        .parse()
        .map_err(|e: lettre::address::AddressError| AppError::Internal(e.to_string()))?;

    let email = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .multipart(
            MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(ContentType::TEXT_HTML)
                    .body(html_body.to_string()),
            ),
        )
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let creds = Credentials::new(config.smtp_user.clone(), config.smtp_password.clone());

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.smtp_host)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .credentials(creds)
        .port(config.smtp_port)
        .build();

    mailer
        .send(email)
        .await
        .map_err(|e| AppError::EmailError(e.to_string()))?;

    Ok(())
}

// ── Email templates ──────────────────────────────────────────────────────────

pub fn password_reset_html(reset_url: &str, expiry_minutes: u64) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <h2 style="color: #333;">Password Reset Request</h2>
  <p>We received a request to reset your MtaaLink password.</p>
  <p>Click the button below to reset it. This link expires in <strong>{expiry_minutes} minutes</strong>.</p>
  <p style="margin: 30px 0;">
    <a href="{reset_url}"
       style="background-color: #4CAF50; color: white; padding: 12px 24px;
              text-decoration: none; border-radius: 4px; display: inline-block;">
      Reset Password
    </a>
  </p>
  <p style="color: #666; font-size: 14px;">
    If you did not request a password reset, you can safely ignore this email.
    Your password will not change.
  </p>
  <hr style="border: none; border-top: 1px solid #eee; margin: 20px 0;">
  <p style="color: #999; font-size: 12px;">MtaaLink — Connecting you to local services</p>
</body>
</html>"#,
        expiry_minutes = expiry_minutes,
        reset_url = reset_url,
    )
}

pub fn booking_confirmation_html(
    client_name: &str,
    service_description: &str,
    scheduled_time: &str,
    provider_name: &str,
) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <h2 style="color: #333;">Booking Confirmed</h2>
  <p>Hi <strong>{client_name}</strong>,</p>
  <p>Your booking has been confirmed. Here are the details:</p>
  <table style="width: 100%; border-collapse: collapse; margin: 20px 0;">
    <tr style="background-color: #f5f5f5;">
      <td style="padding: 10px; border: 1px solid #ddd;"><strong>Service</strong></td>
      <td style="padding: 10px; border: 1px solid #ddd;">{service_description}</td>
    </tr>
    <tr>
      <td style="padding: 10px; border: 1px solid #ddd;"><strong>Provider</strong></td>
      <td style="padding: 10px; border: 1px solid #ddd;">{provider_name}</td>
    </tr>
    <tr style="background-color: #f5f5f5;">
      <td style="padding: 10px; border: 1px solid #ddd;"><strong>Scheduled Time</strong></td>
      <td style="padding: 10px; border: 1px solid #ddd;">{scheduled_time}</td>
    </tr>
  </table>
  <p>If you need to reschedule or cancel, please log into your MtaaLink account.</p>
  <hr style="border: none; border-top: 1px solid #eee; margin: 20px 0;">
  <p style="color: #999; font-size: 12px;">MtaaLink — Connecting you to local services</p>
</body>
</html>"#,
        client_name = client_name,
        service_description = service_description,
        provider_name = provider_name,
        scheduled_time = scheduled_time,
    )
}
