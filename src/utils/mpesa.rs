use crate::errors::{AppError, AppResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;

// ── Configuration ─────────────────────────────────────────────────────────────

pub struct MpesaConfig {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub shortcode: String,
    pub passkey: String,
    pub callback_url: String,
    pub base_url: String, // sandbox or production
}

impl MpesaConfig {
    pub fn from_env() -> AppResult<Self> {
        let env_name = env::var("MPESA_ENV").unwrap_or_else(|_| "sandbox".to_string());
        let base_url = if env_name == "production" {
            "https://api.safaricom.co.ke".to_string()
        } else {
            "https://sandbox.safaricom.co.ke".to_string()
        };

        Ok(Self {
            consumer_key: env::var("MPESA_CONSUMER_KEY")
                .map_err(|_| AppError::Internal("MPESA_CONSUMER_KEY not set".to_string()))?,
            consumer_secret: env::var("MPESA_CONSUMER_SECRET")
                .map_err(|_| AppError::Internal("MPESA_CONSUMER_SECRET not set".to_string()))?,
            shortcode: env::var("MPESA_SHORTCODE")
                .map_err(|_| AppError::Internal("MPESA_SHORTCODE not set".to_string()))?,
            passkey: env::var("MPESA_PASSKEY")
                .map_err(|_| AppError::Internal("MPESA_PASSKEY not set".to_string()))?,
            callback_url: env::var("MPESA_CALLBACK_URL")
                .map_err(|_| AppError::Internal("MPESA_CALLBACK_URL not set".to_string()))?,
            base_url,
        })
    }

    /// STK Push password = base64(shortcode + passkey + timestamp)
    pub fn password(&self, timestamp: &str) -> String {
        STANDARD.encode(format!("{}{}{}", self.shortcode, self.passkey, timestamp))
    }
}

// ── OAuth token ───────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

pub async fn get_access_token(config: &MpesaConfig) -> AppResult<String> {
    let credentials = STANDARD.encode(format!(
        "{}:{}",
        config.consumer_key, config.consumer_secret
    ));

    let client = reqwest::Client::new();
    let resp = client
        .get(format!(
            "{}/oauth/v1/generate?grant_type=client_credentials",
            config.base_url
        ))
        .header("Authorization", format!("Basic {}", credentials))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("M-Pesa auth request failed: {}", e)))?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("M-Pesa auth error: {}", body)));
    }

    let token: TokenResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("M-Pesa auth parse error: {}", e)))?;

    Ok(token.access_token)
}

// ── STK Push ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct StkPushRequest {
    business_short_code: String,
    password: String,
    timestamp: String,
    transaction_type: String,
    amount: i64,
    party_a: String,
    party_b: String,
    phone_number: String,
    call_back_u_r_l: String,
    account_reference: String,
    transaction_desc: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
pub struct StkPushResponse {
    pub merchant_request_id: Option<String>,
    pub checkout_request_id: Option<String>,
    pub response_code: Option<String>,
    pub response_description: Option<String>,
    pub customer_message: Option<String>,
    // Error fields (present when request fails)
    #[serde(rename = "errorCode")]
    pub error_code: Option<String>,
    #[serde(rename = "errorMessage")]
    pub error_message: Option<String>,
}

/// Initiate M-Pesa STK Push. Returns the checkout_request_id used to track the payment.
pub async fn stk_push(
    config: &MpesaConfig,
    phone: &str,      // format: 254XXXXXXXXX
    amount: i64,      // in KES, whole number
    booking_ref: &str, // shown on customer's phone as account reference
) -> AppResult<StkPushResponse> {
    let token = get_access_token(config).await?;
    let timestamp = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let password = config.password(&timestamp);

    let body = StkPushRequest {
        business_short_code: config.shortcode.clone(),
        password,
        timestamp,
        transaction_type: "CustomerPayBillOnline".to_string(),
        amount,
        party_a: phone.to_string(),
        party_b: config.shortcode.clone(),
        phone_number: phone.to_string(),
        call_back_u_r_l: config.callback_url.clone(),
        account_reference: booking_ref.to_string(),
        transaction_desc: format!("Payment for booking {}", booking_ref),
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(format!(
            "{}/mpesa/stkpush/v1/processrequest",
            config.base_url
        ))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("STK Push request failed: {}", e)))?;

    let result: StkPushResponse = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("STK Push parse error: {}", e)))?;

    if result.error_code.is_some() {
        return Err(AppError::Internal(format!(
            "STK Push error: {}",
            result.error_message.as_deref().unwrap_or("Unknown error")
        )));
    }

    Ok(result)
}

// ── Callback parsing ──────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MpesaCallback {
    pub body: CallbackBody,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CallbackBody {
    pub stk_callback: StkCallback,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)]
pub struct StkCallback {
    pub merchant_request_id: String,
    pub checkout_request_id: String,
    pub result_code: i32,
    pub result_desc: String,
    pub callback_metadata: Option<CallbackMetadata>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct CallbackMetadata {
    pub item: Vec<MetadataItem>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MetadataItem {
    pub name: String,
    pub value: Option<serde_json::Value>,
}

impl CallbackMetadata {
    pub fn get(&self, name: &str) -> Option<&serde_json::Value> {
        self.item
            .iter()
            .find(|i| i.name == name)
            .and_then(|i| i.value.as_ref())
    }

    pub fn receipt_number(&self) -> Option<String> {
        self.get("MpesaReceiptNumber")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    #[allow(dead_code)]
    pub fn amount(&self) -> Option<f64> {
        self.get("Amount").and_then(|v| v.as_f64())
    }
}

/// Normalize phone number to Safaricom format (254XXXXXXXXX).
/// Accepts: 07XXXXXXXX, 7XXXXXXXX, +2547XXXXXXXX, 2547XXXXXXXX
pub fn normalize_phone(phone: &str) -> AppResult<String> {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

    let normalized = if digits.starts_with("254") && digits.len() == 12 {
        digits
    } else if digits.starts_with("0") && digits.len() == 10 {
        format!("254{}", &digits[1..])
    } else if digits.len() == 9 {
        format!("254{}", digits)
    } else {
        return Err(AppError::BadRequest(format!(
            "Invalid phone number: {}",
            phone
        )));
    };

    Ok(normalized)
}
