use crate::errors::{AppError, AppResult};
use serde_json::json;
use std::env;

pub async fn send_email(
    to_address: &str,
    subject: &str,
    html_body: &str,
) -> AppResult<()> {
    let api_key = env::var("BREVO_API_KEY")
        .map_err(|_| AppError::Internal("BREVO_API_KEY not set".to_string()))?;
    let from = env::var("FROM_EMAIL")
        .unwrap_or_else(|_| "kamausimon217@gmail.com".to_string());
    let from_name = env::var("FROM_NAME")
        .unwrap_or_else(|_| "Sokavi".to_string());

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", &api_key)
        .header("content-type", "application/json")
        .json(&json!({
            "sender": { "name": from_name, "email": from },
            "to": [{ "email": to_address }],
            "subject": subject,
            "htmlContent": html_body,
        }))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Brevo request failed: {}", e)))?;

    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("Brevo error {}: {}", status, body)));
    }

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
  <p>We received a request to reset your Sokavi password.</p>
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
  <p style="color: #999; font-size: 12px;">Sokavi — Connecting you to local services</p>
</body>
</html>"#,
        expiry_minutes = expiry_minutes,
        reset_url = reset_url,
    )
}

pub fn email_verification_html(verify_url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html>
<head><meta charset="UTF-8"></head>
<body style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px;">
  <h2 style="color: #333;">Verify your email address</h2>
  <p>Thanks for joining Sokavi! Please verify your email address to complete your registration.</p>
  <p style="margin: 30px 0;">
    <a href="{verify_url}"
       style="background-color: #4CAF50; color: white; padding: 12px 24px;
              text-decoration: none; border-radius: 4px; display: inline-block;">
      Verify Email Address
    </a>
  </p>
  <p style="color: #666; font-size: 14px;">This link expires in 24 hours.</p>
  <p style="color: #666; font-size: 14px;">
    If you did not create a Sokavi account, you can safely ignore this email.
  </p>
  <hr style="border: none; border-top: 1px solid #eee; margin: 20px 0;">
  <p style="color: #999; font-size: 12px;">Sokavi — Connecting you to local services</p>
</body>
</html>"#,
        verify_url = verify_url,
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
  <p>If you need to reschedule or cancel, please log into your Sokavi account.</p>
  <hr style="border: none; border-top: 1px solid #eee; margin: 20px 0;">
  <p style="color: #999; font-size: 12px;">Sokavi — Connecting you to local services</p>
</body>
</html>"#,
        client_name = client_name,
        service_description = service_description,
        provider_name = provider_name,
        scheduled_time = scheduled_time,
    )
}
