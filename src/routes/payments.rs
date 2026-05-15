use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::mpesa::{MpesaConfig, MpesaCallback, normalize_phone, stk_push};
use crate::utils::notifications::{notify_best_effort, notify_target_owner};
use crate::utils::sms::{SmsConfig, payment_success_sms, payment_failed_sms, send_sms_best_effort};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn payment_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/initiate", post(initiate_payment))
        .route("/mpesa/callback", post(mpesa_callback))
        .route("/booking/:booking_id", get(get_payment_status))
        .with_state(pool)
}

// ── Structs ───────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct InitiatePaymentRequest {
    pub booking_id: i32,
    pub phone_number: String,
    pub amount: BigDecimal,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct PaymentRecord {
    pub id: i32,
    pub booking_id: Option<i32>,
    pub phone_number: String,
    pub amount: BigDecimal,
    pub checkout_request_id: Option<String>,
    pub transaction_id: Option<String>,
    pub status: String,
    pub result_desc: Option<String>,
    pub created_at: Option<chrono::NaiveDateTime>,
}

// ── Initiate STK Push ─────────────────────────────────────────────────────────

pub async fn initiate_payment(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<InitiatePaymentRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    // Verify booking belongs to this user
    let booking = sqlx::query!(
        "SELECT id, status FROM bookings WHERE id = $1 AND client_id = $2",
        payload.booking_id,
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    if booking.status == "completed" {
        return Err(AppError::BadRequest("Booking is already completed".to_string()));
    }

    // Prevent duplicate pending payments for the same booking
    let existing = sqlx::query_scalar!(
        "SELECT id FROM payments WHERE booking_id = $1 AND status = 'pending'",
        payload.booking_id
    )
    .fetch_optional(&pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "A payment is already pending for this booking".to_string(),
        ));
    }

    let phone = normalize_phone(&payload.phone_number)?;

    // Amount must be a whole number for M-Pesa
    let amount_i64: i64 = payload
        .amount
        .to_string()
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if amount_i64 <= 0 {
        return Err(AppError::BadRequest("Invalid payment amount".to_string()));
    }

    let config = MpesaConfig::from_env()
        .map_err(|_| AppError::Internal("M-Pesa not configured".to_string()))?;

    let booking_ref = format!("BK{}", payload.booking_id);
    let stk = stk_push(&config, &phone, amount_i64, &booking_ref).await?;

    let checkout_request_id = stk.checkout_request_id.as_deref().unwrap_or("");
    let merchant_request_id = stk.merchant_request_id.as_deref();

    // Persist the pending payment record
    let record = sqlx::query_as!(
        PaymentRecord,
        r#"INSERT INTO payments
               (booking_id, phone_number, amount, checkout_request_id, merchant_request_id, status)
           VALUES ($1, $2, $3, $4, $5, 'pending')
           RETURNING id, booking_id, phone_number, amount, checkout_request_id,
                     transaction_id, status, result_desc, created_at"#,
        payload.booking_id,
        phone,
        payload.amount,
        checkout_request_id,
        merchant_request_id
    )
    .fetch_one(&pool)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Payment initiated — check your phone for the M-Pesa prompt",
            "payment_id": record.id,
            "checkout_request_id": checkout_request_id,
            "customer_message": stk.customer_message
        })),
    ))
}

// ── M-Pesa callback (called by Safaricom, no auth) ───────────────────────────

pub async fn mpesa_callback(
    State(pool): State<PgPool>,
    Json(payload): Json<MpesaCallback>,
) -> (StatusCode, Json<serde_json::Value>) {
    let cb = &payload.body.stk_callback;

    let (status, transaction_id) = if cb.result_code == 0 {
        let receipt = cb
            .callback_metadata
            .as_ref()
            .and_then(|m| m.receipt_number());
        ("completed", receipt)
    } else {
        ("failed", None)
    };

    let update = sqlx::query!(
        r#"UPDATE payments
           SET status = $1,
               transaction_id = $2,
               result_code = $3,
               result_desc = $4,
               updated_at = NOW()
           WHERE checkout_request_id = $5"#,
        status,
        transaction_id,
        cb.result_code,
        cb.result_desc,
        cb.checkout_request_id
    )
    .execute(&pool)
    .await;

    if let Err(e) = update {
        tracing::error!("Failed to update payment from M-Pesa callback: {}", e);
        // Always return 200 to M-Pesa — retries are not useful here
        return (
            StatusCode::OK,
            Json(json!({ "ResultCode": 0, "ResultDesc": "Accepted" })),
        );
    }

    // If payment succeeded, mark the booking as confirmed
    if status == "completed" {
        let _ = sqlx::query!(
            r#"UPDATE bookings b
               SET status = 'confirmed', updated_at = NOW()
               FROM payments p
               WHERE p.checkout_request_id = $1
                 AND p.booking_id = b.id
                 AND b.status = 'pending'"#,
            cb.checkout_request_id
        )
        .execute(&pool)
        .await;
    }

    tracing::info!(
        "M-Pesa callback: checkout_id={} result={} desc={}",
        cb.checkout_request_id,
        cb.result_code,
        cb.result_desc
    );

    // SMS receipt/failure notification (best-effort)
    if let Ok(sms_cfg) = SmsConfig::from_env() {
        let payment_row = sqlx::query!(
            "SELECT phone_number, amount, booking_id FROM payments WHERE checkout_request_id = $1",
            cb.checkout_request_id
        )
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

        if let Some(p) = payment_row {
            let msg = if status == "completed" {
                let receipt = cb
                    .callback_metadata
                    .as_ref()
                    .and_then(|m| m.receipt_number())
                    .unwrap_or_else(|| "N/A".to_string());
                payment_success_sms(
                    &p.amount.to_string(),
                    &receipt,
                    p.booking_id.unwrap_or(0),
                )
            } else {
                payment_failed_sms(p.booking_id.unwrap_or(0))
            };
            send_sms_best_effort(&sms_cfg, &p.phone_number, &msg).await;
        }
    }

    // In-app notification (best-effort)
    if let Some(p) = sqlx::query!(
        "SELECT b.id AS booking_id, b.client_id, b.target_type, b.target_id
         FROM bookings b
         JOIN payments p ON p.booking_id = b.id
         WHERE p.checkout_request_id = $1
         LIMIT 1",
        cb.checkout_request_id
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten()
    {
        if status == "completed" {
            notify_target_owner(
                &pool, &p.target_type, p.target_id,
                "payment_received", "Payment Received",
                "A payment was completed for one of your bookings",
                Some("booking"), Some(p.booking_id),
            ).await;
        } else {
            notify_best_effort(
                &pool, p.client_id,
                "payment_failed", "Payment Failed",
                &format!("Payment for booking #{} could not be processed. Please try again.", p.booking_id),
                Some("booking"), Some(p.booking_id),
            ).await;
        }
    }

    (
        StatusCode::OK,
        Json(json!({ "ResultCode": 0, "ResultDesc": "Accepted" })),
    )
}

// ── Payment status ────────────────────────────────────────────────────────────

pub async fn get_payment_status(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(booking_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    // Check booking belongs to this user
    sqlx::query_scalar!(
        "SELECT id FROM bookings WHERE id = $1 AND client_id = $2",
        booking_id,
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    let payment = sqlx::query_as!(
        PaymentRecord,
        r#"SELECT id, booking_id, phone_number, amount, checkout_request_id,
                  transaction_id, status, result_desc, created_at
           FROM payments
           WHERE booking_id = $1
           ORDER BY created_at DESC
           LIMIT 1"#,
        booking_id
    )
    .fetch_optional(&pool)
    .await?;

    match payment {
        Some(p) => Ok((StatusCode::OK, Json(json!({ "payment": p })))),
        None => Err(AppError::NotFound("No payment found for this booking".to_string())),
    }
}
