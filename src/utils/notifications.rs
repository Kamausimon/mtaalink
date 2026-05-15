use sqlx::PgPool;

/// Insert a notification row. Errors are caller's responsibility.
pub async fn notify(
    pool: &PgPool,
    user_id: i32,
    notif_type: &str,
    title: &str,
    body: &str,
    target_type: Option<&str>,
    target_id: Option<i32>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO notifications (user_id, notif_type, title, body, target_type, target_id)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
        user_id,
        notif_type,
        title,
        body,
        target_type,
        target_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Best-effort notification — logs on failure, never propagates the error.
pub async fn notify_best_effort(
    pool: &PgPool,
    user_id: i32,
    notif_type: &str,
    title: &str,
    body: &str,
    target_type: Option<&str>,
    target_id: Option<i32>,
) {
    if let Err(e) = notify(pool, user_id, notif_type, title, body, target_type, target_id).await {
        tracing::warn!("Notification creation failed (non-fatal): {}", e);
    }
}

/// Looks up the owner's user_id for a provider or business, then notifies them.
pub async fn notify_target_owner(
    pool: &PgPool,
    target_type: &str,
    target_id: i32,
    notif_type: &str,
    title: &str,
    body: &str,
    ref_type: Option<&str>,
    ref_id: Option<i32>,
) {
    let owner_user_id: Option<i32> = match target_type {
        "provider" => sqlx::query_scalar!(
            "SELECT user_id FROM providers WHERE id = $1",
            target_id
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten(),
        "business" => sqlx::query_scalar!(
            "SELECT user_id FROM businesses WHERE id = $1",
            target_id
        )
        .fetch_optional(pool)
        .await
        .ok()
        .flatten(),
        _ => None,
    };

    if let Some(uid) = owner_user_id {
        notify_best_effort(pool, uid, notif_type, title, body, ref_type, ref_id).await;
    }
}
