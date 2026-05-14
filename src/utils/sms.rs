use crate::errors::{AppError, AppResult};
use serde::Deserialize;
use std::env;

// ── Configuration ─────────────────────────────────────────────────────────────

pub struct SmsConfig {
    pub api_key: String,
    pub username: String,
    pub sender_id: Option<String>,
    pub base_url: String,
}

impl SmsConfig {
    pub fn from_env() -> AppResult<Self> {
        let env_name = env::var("AT_ENV").unwrap_or_else(|_| "sandbox".to_string());
        let base_url = if env_name == "production" {
            "https://api.africastalking.com".to_string()
        } else {
            "https://api.sandbox.africastalking.com".to_string()
        };

        Ok(Self {
            api_key: env::var("AT_API_KEY")
                .map_err(|_| AppError::Internal("AT_API_KEY not set".to_string()))?,
            username: env::var("AT_USERNAME").unwrap_or_else(|_| "sandbox".to_string()),
            sender_id: env::var("AT_SENDER_ID").ok(),
            base_url,
        })
    }
}

// ── Send SMS ──────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct SmsResponse {
    #[serde(rename = "SMSMessageData")]
    sms_message_data: SmsMessageData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct SmsMessageData {
    message: String,
}

/// Send an SMS to `to` (phone number in international format e.g. +254712345678).
/// Gracefully logs errors rather than hard-failing — SMS is best-effort.
pub async fn send_sms(config: &SmsConfig, to: &str, message: &str) -> AppResult<()> {
    let client = reqwest::Client::new();

    let mut params = vec![
        ("username", config.username.clone()),
        ("to", to.to_string()),
        ("message", message.to_string()),
    ];
    if let Some(ref sender) = config.sender_id {
        params.push(("from", sender.clone()));
    }

    let resp = client
        .post(format!("{}/version1/messaging", config.base_url))
        .header("apiKey", &config.api_key)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("SMS send failed: {}", e)))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("SMS API error: {}", body)));
    }

    let result: SmsResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("SMS response parse error: {}", e)))?;

    tracing::info!("SMS sent: {}", result.sms_message_data.message);
    Ok(())
}

/// Send SMS, logging errors without propagating — use for non-critical notifications.
pub async fn send_sms_best_effort(config: &SmsConfig, to: &str, message: &str) {
    if let Err(e) = send_sms(config, to, message).await {
        tracing::warn!("SMS notification failed (non-fatal): {}", e);
    }
}

// ── Message templates ─────────────────────────────────────────────────────────

pub fn booking_confirmation_sms(booking_id: i32, scheduled_time: &str, service: &str) -> String {
    format!(
        "MtaaLink: Your booking #{booking_id} for {service} on {scheduled_time} is confirmed. \
         Thank you for using MtaaLink!"
    )
}

pub fn booking_cancelled_sms(booking_id: i32, reason: &str) -> String {
    format!(
        "MtaaLink: Booking #{booking_id} has been cancelled. Reason: {reason}. \
         Visit the app to rebook."
    )
}

pub fn payment_success_sms(amount: &str, receipt: &str, booking_id: i32) -> String {
    format!(
        "MtaaLink: Payment of KES {amount} received for booking #{booking_id}. \
         M-Pesa receipt: {receipt}. Thank you!"
    )
}

pub fn payment_failed_sms(booking_id: i32) -> String {
    format!(
        "MtaaLink: Payment for booking #{booking_id} was not completed. \
         Please try again in the app."
    )
}

pub fn booking_reminder_sms(booking_id: i32, scheduled_time: &str, service: &str) -> String {
    format!(
        "MtaaLink: Reminder — you have a booking #{booking_id} for {service} \
         tomorrow at {scheduled_time}. Please be ready."
    )
}

pub fn password_reset_sms(otp: &str) -> String {
    format!(
        "MtaaLink: Your password reset code is {otp}. \
         It expires in 15 minutes. Do not share this code."
    )
}

pub fn new_booking_received_sms(booking_id: i32, client_name: &str, service: &str) -> String {
    format!(
        "MtaaLink: New booking #{booking_id} from {client_name} for {service}. \
         Open the app to confirm or decline."
    )
}
