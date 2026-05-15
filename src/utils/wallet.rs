use bigdecimal::BigDecimal;
use sqlx::PgPool;

/// Credit a provider/business wallet after a completed payment.
/// Auto-creates the wallet row on first credit (upsert).
/// Runs both the wallet update and ledger insert in one transaction.
pub async fn credit_wallet(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    amount: &BigDecimal,
    description: &str,
    booking_id: Option<i32>,
    payment_id: Option<i32>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    let wallet_id = sqlx::query_scalar!(
        r#"INSERT INTO wallets (target_type, target_id, balance, total_earned)
           VALUES ($1, $2, $3, $3)
           ON CONFLICT (target_type, target_id) DO UPDATE
               SET balance      = wallets.balance      + $3,
                   total_earned = wallets.total_earned  + $3,
                   updated_at   = NOW()
           RETURNING id"#,
        target_type,
        target_id,
        amount
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO wallet_transactions
               (wallet_id, txn_type, amount, description, booking_id, payment_id)
           VALUES ($1, 'credit', $2, $3, $4, $5)"#,
        wallet_id,
        amount,
        description,
        booking_id,
        payment_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

/// Best-effort wallet credit — logs on failure, never propagates.
pub async fn credit_wallet_best_effort(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    amount: &BigDecimal,
    description: &str,
    booking_id: Option<i32>,
    payment_id: Option<i32>,
) {
    if let Err(e) = credit_wallet(pool, target_type, target_id, amount, description, booking_id, payment_id).await {
        tracing::warn!("Wallet credit failed (non-fatal): {}", e);
    }
}
