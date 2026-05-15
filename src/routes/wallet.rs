use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::mpesa::normalize_phone;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::str::FromStr;

pub fn wallet_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/:target_type/:target_id", get(get_wallet))
        .route("/:target_type/:target_id/transactions", get(get_transactions))
        .route("/:target_type/:target_id/payout", post(request_payout))
        .route("/:target_type/:target_id/payouts", get(list_payouts))
        .with_state(pool)
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct WalletRow {
    pub id: i32,
    pub balance: BigDecimal,
    pub total_earned: BigDecimal,
    pub total_paid_out: BigDecimal,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct TransactionRow {
    pub id: i32,
    pub txn_type: String,
    pub amount: BigDecimal,
    pub description: String,
    pub booking_id: Option<i32>,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct PayoutRow {
    pub id: i32,
    pub amount: BigDecimal,
    pub phone_number: String,
    pub status: String,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Deserialize, Debug)]
pub struct PageQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Deserialize, Debug)]
pub struct PayoutRequest {
    pub amount: String, // sent as string to avoid float precision issues
    pub phone_number: String,
}

// ── Ownership helper ──────────────────────────────────────────────────────────

async fn verify_owner(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    user_id: i32,
) -> AppResult<()> {
    let owner = match target_type {
        "provider" => sqlx::query_scalar!(
            "SELECT user_id FROM providers WHERE id = $1",
            target_id
        )
        .fetch_optional(pool)
        .await?,
        _ => sqlx::query_scalar!(
            "SELECT user_id FROM businesses WHERE id = $1",
            target_id
        )
        .fetch_optional(pool)
        .await?,
    };

    if owner != Some(user_id) {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }
    Ok(())
}

// ── GET /wallet/:target_type/:target_id ───────────────────────────────────────

pub async fn get_wallet(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path((target_type, target_id)): Path<(String, i32)>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("target_type must be 'provider' or 'business'".to_string()));
    }
    verify_owner(&pool, &target_type, target_id, user_id).await?;

    let wallet = sqlx::query_as::<_, WalletRow>(
        "SELECT id, balance, total_earned, total_paid_out, created_at, updated_at
         FROM wallets WHERE target_type = $1 AND target_id = $2",
    )
    .bind(&target_type)
    .bind(target_id)
    .fetch_optional(&pool)
    .await?;

    match wallet {
        Some(w) => Ok((StatusCode::OK, Json(json!({ "wallet": w })))),
        None => Ok((
            StatusCode::OK,
            Json(json!({
                "wallet": {
                    "balance": "0.00",
                    "total_earned": "0.00",
                    "total_paid_out": "0.00",
                    "message": "No earnings yet"
                }
            })),
        )),
    }
}

// ── GET /wallet/:target_type/:target_id/transactions ─────────────────────────

pub async fn get_transactions(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path((target_type, target_id)): Path<(String, i32)>,
    Query(params): Query<PageQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("target_type must be 'provider' or 'business'".to_string()));
    }
    verify_owner(&pool, &target_type, target_id, user_id).await?;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let transactions = sqlx::query_as::<_, TransactionRow>(
        r#"SELECT wt.id, wt.txn_type, wt.amount, wt.description, wt.booking_id, wt.created_at
           FROM wallet_transactions wt
           JOIN wallets w ON wt.wallet_id = w.id
           WHERE w.target_type = $1 AND w.target_id = $2
           ORDER BY wt.created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(&target_type)
    .bind(target_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&pool)
    .await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "transactions": transactions, "page": page, "per_page": per_page })),
    ))
}

// ── POST /wallet/:target_type/:target_id/payout ───────────────────────────────

pub async fn request_payout(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path((target_type, target_id)): Path<(String, i32)>,
    Json(payload): Json<PayoutRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("target_type must be 'provider' or 'business'".to_string()));
    }
    verify_owner(&pool, &target_type, target_id, user_id).await?;

    let amount = BigDecimal::from_str(payload.amount.trim())
        .map_err(|_| AppError::BadRequest("Invalid amount format".to_string()))?;

    if amount <= BigDecimal::from(0) {
        return Err(AppError::BadRequest("Payout amount must be greater than zero".to_string()));
    }

    let phone = normalize_phone(&payload.phone_number)?;

    // Fetch wallet — must exist with sufficient balance
    let wallet = sqlx::query!(
        "SELECT id, balance FROM wallets WHERE target_type = $1 AND target_id = $2",
        target_type,
        target_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("No wallet found — make sure you have received payments first".to_string()))?;

    if amount > wallet.balance {
        return Err(AppError::BadRequest(format!(
            "Insufficient balance. Available: {}",
            wallet.balance
        )));
    }

    // Deduct balance, record transaction, create payout request — all in one transaction
    let mut tx = pool.begin().await?;

    sqlx::query!(
        r#"UPDATE wallets
           SET balance = balance - $1, total_paid_out = total_paid_out + $1, updated_at = NOW()
           WHERE id = $2"#,
        amount,
        wallet.id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO wallet_transactions (wallet_id, txn_type, amount, description)
           VALUES ($1, 'payout', $2, $3)"#,
        wallet.id,
        amount,
        format!("Payout request to {}", phone)
    )
    .execute(&mut *tx)
    .await?;

    let payout = sqlx::query!(
        r#"INSERT INTO payout_requests (wallet_id, amount, phone_number)
           VALUES ($1, $2, $3) RETURNING id"#,
        wallet.id,
        amount,
        phone
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Payout request submitted. You will be notified when it is processed.",
            "payout_id": payout.id,
            "amount": amount,
            "phone_number": phone,
        })),
    ))
}

// ── GET /wallet/:target_type/:target_id/payouts ───────────────────────────────

pub async fn list_payouts(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path((target_type, target_id)): Path<(String, i32)>,
    Query(params): Query<PageQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("target_type must be 'provider' or 'business'".to_string()));
    }
    verify_owner(&pool, &target_type, target_id, user_id).await?;

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let payouts = sqlx::query_as::<_, PayoutRow>(
        r#"SELECT pr.id, pr.amount, pr.phone_number, pr.status, pr.notes,
                  pr.created_at, pr.updated_at
           FROM payout_requests pr
           JOIN wallets w ON pr.wallet_id = w.id
           WHERE w.target_type = $1 AND w.target_id = $2
           ORDER BY pr.created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(&target_type)
    .bind(target_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&pool)
    .await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "payouts": payouts, "page": page, "per_page": per_page })),
    ))
}
