use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use crate::utils::email::{EmailConfig, booking_confirmation_html, send_email};
use crate::utils::notifications::{notify_and_push, notify_target_owner_and_push};
use crate::utils::sms::{SmsConfig, booking_confirmation_sms, booking_cancelled_sms,
                        new_booking_received_sms, send_sms_best_effort};
use crate::utils::storage::{SharedStorage, generate_key};
use crate::utils::ws_state::WsConnections;
use axum::{
    Extension, Json, Router,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn booking_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/createBooking", post(create_booking))
        .route("/getBookings/me", get(get_bookings_client))
        .route("/getBookings/received", get(get_bookings_received))
        .route("/:id", get(get_booking_by_id))
        .route("/:id/status", post(update_booking))
        .route("/:id/delete", post(delete_booking))
        .route("/:id/reschedule", post(reschedule_booking))
        .route("/:id/dispute_response", post(submit_dispute_response))
        .route("/:id/evidence", post(upload_dispute_evidence))
        .route("/:id/evidence", get(get_dispute_evidence))
        .route("/:id/evidence/url", post(record_dispute_evidence_url))
        .with_state(pool)
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct Booking {
    pub id: i32,
    pub client_id: i32,
    pub target_type: String,
    pub target_id: i32,
    pub branch_id: Option<i32>,
    pub service_id: Option<i32>,
    pub service_description: Option<String>,
    pub scheduled_time: chrono::NaiveDateTime,
    pub status: String,
    pub duration: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
    pub client_address: Option<String>,
    pub client_latitude: Option<f64>,
    pub client_longitude: Option<f64>,
    pub client_phone: Option<String>,
    pub cancel_reason: Option<String>,
    pub dispute_reason: Option<String>,
    pub dispute_response: Option<String>,
    pub admin_resolution: Option<String>,
    pub reminder_sent: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct CreateBookingInput {
    pub target_type: String,
    pub target_id: i32,
    pub branch_id: Option<i32>,
    pub service_id: Option<i32>,
    pub service_description: String,
    pub scheduled_time: chrono::NaiveDateTime,
    /// Client phone (07XX / +2547XX / 2547XX) — used for SMS and stored so provider can call back.
    pub client_phone: Option<String>,
    /// Physical address where the service should be performed.
    pub client_address: Option<String>,
    pub client_latitude: Option<f64>,
    pub client_longitude: Option<f64>,
}

pub async fn create_booking(
    State(pool): State<PgPool>,
    Extension(ws_conns): Extension<WsConnections>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<CreateBookingInput>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = payload.target_type.to_lowercase();
    if target_type != "business" && target_type != "provider" {
        return Err(AppError::BadRequest("Invalid target type".to_string()));
    }

    let target_id = payload.target_id;
    if target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID".to_string()));
    }

    let target_exists = match target_type.as_str() {
        "business" => sqlx::query_scalar!("SELECT id FROM businesses WHERE id = $1", target_id)
            .fetch_optional(&pool)
            .await?,
        "provider" => sqlx::query_scalar!("SELECT id FROM providers WHERE id = $1", target_id)
            .fetch_optional(&pool)
            .await?,
        _ => None,
    };

    if target_exists.is_none() {
        return Err(AppError::BadRequest("Target ID does not exist".to_string()));
    }

    // Check if provider/business is currently suspended
    let is_suspended = match target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT 1 FROM providers WHERE id = $1 AND suspended_until IS NOT NULL AND suspended_until > NOW()",
            target_id
        ).fetch_optional(&pool).await?.is_some(),
        "business" => sqlx::query_scalar!(
            "SELECT 1 FROM businesses WHERE id = $1 AND suspended_until IS NOT NULL AND suspended_until > NOW()",
            target_id
        ).fetch_optional(&pool).await?.is_some(),
        _ => false,
    };
    if is_suspended {
        return Err(AppError::BadRequest(
            "This provider is currently suspended and cannot accept new bookings.".to_string(),
        ));
    }

    if payload.scheduled_time < chrono::Local::now().naive_local() {
        return Err(AppError::BadRequest("Scheduled time cannot be in the past".to_string()));
    }

    let existing = sqlx::query_scalar!(
        "SELECT id FROM bookings WHERE target_type = $1 AND target_id = $2 AND scheduled_time = $3",
        target_type,
        target_id,
        payload.scheduled_time
    )
    .fetch_optional(&pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("This time slot has already been booked".to_string()));
    }

    if let Some(service_id) = payload.service_id {
        let service_exists = sqlx::query_scalar!(
            "SELECT id FROM services WHERE id = $1 AND target_type = $2 AND target_id = $3",
            service_id,
            target_type,
            target_id
        )
        .fetch_optional(&pool)
        .await?;

        if service_exists.is_none() {
            return Err(AppError::BadRequest("Service ID does not exist".to_string()));
        }
    }

    let service_duration = if let Some(service_id) = payload.service_id {
        sqlx::query_scalar!("SELECT duration FROM services WHERE id = $1", service_id)
            .fetch_optional(&pool)
            .await?
            .flatten()
            .unwrap_or(60)
    } else {
        60
    };

    let record = sqlx::query!(
        r#"INSERT INTO bookings (client_id, target_type, target_id, branch_id, service_id,
           service_description, scheduled_time, duration, status,
           client_address, client_latitude, client_longitude, client_phone)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13) RETURNING id"#,
        user_id,
        target_type,
        target_id,
        payload.branch_id,
        payload.service_id,
        payload.service_description.trim(),
        payload.scheduled_time,
        service_duration,
        "pending",
        payload.client_address.as_deref(),
        payload.client_latitude,
        payload.client_longitude,
        payload.client_phone.as_deref(),
    )
    .fetch_one(&pool)
    .await?;

    let booking_id = record.id;
    let scheduled_str = payload.scheduled_time.format("%d %b %Y %H:%M").to_string();

    // ── SMS notifications (best-effort, non-blocking) ─────────────────────────
    if let Ok(sms_cfg) = SmsConfig::from_env() {
        // 1. Confirmation SMS to client (if phone provided)
        if let Some(ref phone) = payload.client_phone {
            let msg = booking_confirmation_sms(
                booking_id, &scheduled_str, payload.service_description.trim(),
            );
            send_sms_best_effort(&sms_cfg, phone, &msg).await;
        }

        // 2. New booking alert to provider/business
        let provider_phone = match target_type.as_str() {
            "provider" => sqlx::query_scalar!(
                "SELECT phone_number FROM providers WHERE id = $1", target_id
            )
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .flatten(),
            "business" => sqlx::query_scalar!(
                "SELECT phone_number FROM businesses WHERE id = $1", target_id
            )
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .flatten(),
            _ => None,
        };

        if let Some(pphone) = provider_phone {
            let client_name = sqlx::query_scalar!(
                "SELECT username FROM users WHERE id = $1", user_id
            )
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "a client".to_string());

            let msg = new_booking_received_sms(
                booking_id, &client_name, payload.service_description.trim(),
            );
            send_sms_best_effort(&sms_cfg, &pphone, &msg).await;
        }
    }

    // In-app notification + WS push to the provider/business owner
    notify_target_owner_and_push(
        &pool, &ws_conns, &target_type, target_id,
        "booking_created", "New Booking",
        &format!("You have a new booking #{} for {}", booking_id, payload.service_description.trim()),
        Some("booking"), Some(booking_id),
    ).await;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "message": "Booking created successfully", "booking_id": booking_id })),
    ))
}

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct BookingQuery {
    pub status: Option<String>,
    pub target_type: Option<String>,
}

pub async fn get_bookings_client(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<BookingQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let bookings = sqlx::query_as::<_, Booking>(
        r#"SELECT * FROM bookings
           WHERE client_id = $1
             AND ($2::text IS NULL OR status = $2)
             AND ($3::text IS NULL OR target_type = $3)
           ORDER BY scheduled_time DESC"#,
    )
    .bind(user_id)
    .bind(&params.status)
    .bind(&params.target_type)
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "bookings": bookings }))))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookingsQueryByReceiver {
    target_type: String,
    target_id: i32,
    /// Filter by status. Omit or pass "all" to return every status.
    status: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BookingResponse {
    pub id: i32,
    pub client_id: i32,
    pub target_type: String,
    pub target_id: i32,
    pub branch_id: Option<i32>,
    pub service_id: Option<i32>,
    pub service_description: String,
    pub scheduled_time: NaiveDateTime,
    pub status: String,
    pub duration: i32,
    pub created_at: Option<NaiveDateTime>,
    pub client_name: String,
    pub client_email: String,
    pub client_phone: Option<String>,
    pub client_address: Option<String>,
    pub client_latitude: Option<f64>,
    pub client_longitude: Option<f64>,
    pub service_name: String,
}

pub async fn get_bookings_received(
    State(pool): State<PgPool>,
    CurrentUser { user_id: _ }: CurrentUser,
    Query(params): Query<BookingsQueryByReceiver>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let target_type = params.target_type.to_lowercase();
    if !["provider", "business"].contains(&target_type.as_str()) {
        return Err(AppError::BadRequest("Invalid target type".to_string()));
    }
    if params.target_id <= 0 {
        return Err(AppError::BadRequest("Invalid target ID".to_string()));
    }

    let status_filter = params
        .status
        .as_deref()
        .filter(|s| !s.is_empty() && *s != "all");

    let rows = sqlx::query!(
        r#"SELECT b.id, b.client_id, b.target_type, b.target_id, b.branch_id, b.service_id,
               b.service_description, b.scheduled_time, b.status, b.duration, b.created_at,
               b.client_address, b.client_latitude, b.client_longitude, b.client_phone,
               u.username as client_name, u.email as client_email,
               CASE WHEN b.service_id IS NOT NULL THEN s.title ELSE b.service_description END AS service_name
        FROM bookings b
        LEFT JOIN users u ON b.client_id = u.id
        LEFT JOIN services s ON b.service_id = s.id
        WHERE b.target_type = $1 AND b.target_id = $2
          AND ($3::text IS NULL OR b.status = $3)
        ORDER BY b.scheduled_time DESC"#,
        target_type,
        params.target_id,
        status_filter
    )
    .fetch_all(&pool)
    .await?;

    let bookings: Vec<BookingResponse> = rows
        .into_iter()
        .map(|row| BookingResponse {
            id: row.id,
            client_id: row.client_id,
            target_type: row.target_type,
            target_id: row.target_id,
            branch_id: row.branch_id,
            service_id: row.service_id,
            service_description: row.service_description.unwrap_or_default(),
            scheduled_time: row.scheduled_time,
            status: row.status,
            duration: row.duration.unwrap_or(60),
            created_at: row.created_at,
            client_name: row.client_name,
            client_email: row.client_email,
            client_phone: row.client_phone,
            client_address: row.client_address,
            client_latitude: row.client_latitude,
            client_longitude: row.client_longitude,
            service_name: row.service_name.unwrap_or_default(),
        })
        .collect();

    Ok((StatusCode::OK, Json(json!({ "bookings": bookings }))))
}

pub async fn get_booking_by_id(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if id <= 0 {
        return Err(AppError::BadRequest("Invalid booking ID".to_string()));
    }

    // Accessible to the client who made it OR the provider/business who received it
    let booking = sqlx::query_as::<_, Booking>(
        r#"SELECT * FROM bookings
           WHERE id = $1
             AND (
               client_id = $2
               OR (target_type = 'provider'  AND EXISTS (SELECT 1 FROM providers  WHERE id = target_id AND user_id = $2))
               OR (target_type = 'business'  AND EXISTS (SELECT 1 FROM businesses WHERE id = target_id AND user_id = $2))
             )"#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    Ok((StatusCode::OK, Json(json!({ "booking": booking }))))
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookingUpdate {
    status: String,
    cancel_reason: Option<String>,
    dispute_reason: Option<String>,
}

pub async fn update_booking(
    State(pool): State<PgPool>,
    Extension(ws_conns): Extension<WsConnections>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<BookingUpdate>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if id <= 0 {
        return Err(AppError::BadRequest("Invalid booking ID".to_string()));
    }

    let booking = sqlx::query!(
        "SELECT target_type, target_id, client_id, status FROM bookings WHERE id = $1", id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    let target_type = booking.target_type.to_lowercase();

    let is_service_owner = match target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        "business" => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        _ => false,
    };

    let is_client = booking.client_id == user_id;

    if !is_service_owner && !is_client {
        return Err(AppError::Forbidden("You don't have permission to update this booking".to_string()));
    }

    let new_status = payload.status.to_lowercase();
    let current_status = booking.status.to_lowercase();

    // Role-based transition rules
    if is_service_owner {
        // Provider/business may confirm, cancel, or signal completion.
        // "completed" from the service side means "I'm done — pending client confirmation".
        let allowed = ["confirmed", "cancelled", "pending_confirmation"];
        if !allowed.contains(&new_status.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Providers may only set status to: {}",
                allowed.join(", ")
            )));
        }
    } else {
        // Client may only confirm or dispute, and only when awaiting confirmation.
        if current_status != "pending_confirmation" {
            return Err(AppError::BadRequest(
                "You can only confirm or dispute a booking that is awaiting confirmation".to_string(),
            ));
        }
        let allowed = ["completed", "disputed"];
        if !allowed.contains(&new_status.as_str()) {
            return Err(AppError::BadRequest(
                "Clients may only confirm completion or raise a dispute".to_string(),
            ));
        }
    }

    sqlx::query!(
        "UPDATE bookings SET status = $1, cancel_reason = $2, dispute_reason = $3 WHERE id = $4",
        new_status,
        payload.cancel_reason.as_deref(),
        payload.dispute_reason.as_deref(),
        id,
    )
    .execute(&pool)
    .await?;

    // ── SMS ─────────────────────────────────────────────────────────────────
    if new_status == "confirmed" || new_status == "cancelled" {
        if let Ok(sms_cfg) = SmsConfig::from_env() {
            let client_phone = sqlx::query_scalar!(
                "SELECT phone_number FROM payments WHERE booking_id = $1 ORDER BY created_at DESC LIMIT 1",
                id
            )
            .fetch_optional(&pool).await.ok().flatten();

            if let Some(phone) = client_phone {
                let msg = if new_status == "confirmed" {
                    let bk = sqlx::query!(
                        "SELECT service_description, scheduled_time FROM bookings WHERE id = $1", id
                    ).fetch_optional(&pool).await.ok().flatten();
                    bk.map(|b| booking_confirmation_sms(
                        id,
                        &b.scheduled_time.format("%d %b %Y %H:%M").to_string(),
                        &b.service_description.unwrap_or_default(),
                    ))
                } else {
                    Some(booking_cancelled_sms(id, "Cancelled by provider"))
                };
                if let Some(m) = msg { send_sms_best_effort(&sms_cfg, &phone, &m).await; }
            }
        }
    }

    // ── Email ────────────────────────────────────────────────────────────────
    if new_status == "confirmed" {
        if let Ok(email_cfg) = EmailConfig::from_env() {
            let details = sqlx::query!(
                r#"SELECT u.email, u.username, b.service_description, b.scheduled_time,
                          COALESCE(p.service_name, biz.business_name, 'Provider') AS provider_name
                   FROM bookings b
                   JOIN users u ON u.id = b.client_id
                   LEFT JOIN providers p ON b.target_type = 'provider' AND b.target_id = p.id
                   LEFT JOIN businesses biz ON b.target_type = 'business' AND b.target_id = biz.id
                   WHERE b.id = $1"#, id
            ).fetch_optional(&pool).await.ok().flatten();

            if let Some(d) = details {
                let html = booking_confirmation_html(
                    &d.username,
                    &d.service_description.unwrap_or_default(),
                    &d.scheduled_time.format("%d %b %Y %H:%M").to_string(),
                    &d.provider_name.unwrap_or_default(),
                );
                let _ = send_email(&email_cfg, &d.email, "Your booking is confirmed — MtaaLink", &html).await;
            }
        }
    }

    // ── In-app notifications ─────────────────────────────────────────────────
    let client_id = booking.client_id;

    // Notify client: confirmed / cancelled / pending_confirmation
    match new_status.as_str() {
        "confirmed" => {
            notify_and_push(&pool, &ws_conns, client_id, &new_status,
                "Booking Confirmed",
                &format!("Your booking #{} has been confirmed", id),
                Some("booking"), Some(id)).await;
        }
        "cancelled" => {
            notify_and_push(&pool, &ws_conns, client_id, &new_status,
                "Booking Cancelled",
                &format!("Your booking #{} has been cancelled", id),
                Some("booking"), Some(id)).await;
        }
        "pending_confirmation" => {
            notify_and_push(&pool, &ws_conns, client_id, &new_status,
                "Job Marked Complete",
                &format!("The provider says booking #{} is done. Please confirm or raise a dispute.", id),
                Some("booking"), Some(id)).await;
        }
        _ => {}
    }

    // Notify service owner: client confirmed or disputed
    if new_status == "completed" || new_status == "disputed" {
        let provider_user_id: Option<i32> = match target_type.as_str() {
            "provider" => sqlx::query_scalar!(
                "SELECT user_id FROM providers WHERE id = $1", booking.target_id
            ).fetch_optional(&pool).await.ok().flatten(),
            "business" => sqlx::query_scalar!(
                "SELECT user_id FROM businesses WHERE id = $1", booking.target_id
            ).fetch_optional(&pool).await.ok().flatten(),
            _ => None,
        };

        if let Some(puid) = provider_user_id {
            let (title, body) = if new_status == "completed" {
                ("Job Confirmed", format!("Client confirmed booking #{} is complete", id))
            } else {
                ("Dispute Raised", format!("Client raised a dispute on booking #{}", id))
            };
            notify_and_push(&pool, &ws_conns, puid, &new_status, &title, &body, Some("booking"), Some(id)).await;
        }
    }

    Ok((StatusCode::OK, Json(json!({ "message": "Status updated successfully" }))))
}

pub async fn delete_booking(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if id <= 0 {
        return Err(AppError::BadRequest("Invalid booking ID".to_string()));
    }

    let booking = sqlx::query!(
        "SELECT client_id FROM bookings WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    if booking.client_id != user_id {
        return Err(AppError::Forbidden("You do not have permission to delete this booking".to_string()));
    }

    sqlx::query!("DELETE FROM bookings WHERE id = $1", id)
        .execute(&pool)
        .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Booking deleted successfully" }))))
}

#[derive(Deserialize, Serialize, Debug, sqlx::FromRow)]
pub struct ReschedulePayload {
    pub scheduled_time: NaiveDateTime,
}

pub async fn reschedule_booking(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<ReschedulePayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if id <= 0 {
        return Err(AppError::BadRequest("Invalid booking ID".to_string()));
    }
    if payload.scheduled_time < chrono::Local::now().naive_local() {
        return Err(AppError::BadRequest("New scheduled time cannot be in the past".to_string()));
    }

    sqlx::query!(
        "UPDATE bookings SET scheduled_time = $1 WHERE id = $2 AND client_id = $3",
        payload.scheduled_time,
        id,
        user_id
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "message": "Booking rescheduled successfully" }))))
}

// ── Dispute response (provider/business submits their side) ──────────────────

#[derive(Deserialize, Debug)]
pub struct DisputeResponsePayload {
    pub response: String,
}

pub async fn submit_dispute_response(
    State(pool): State<PgPool>,
    Extension(ws_conns): Extension<WsConnections>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<DisputeResponsePayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    if payload.response.trim().is_empty() {
        return Err(AppError::BadRequest("Response cannot be empty".to_string()));
    }

    let booking = sqlx::query!(
        "SELECT target_type, target_id, client_id, status FROM bookings WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    if booking.status != "disputed" {
        return Err(AppError::BadRequest("This booking is not in disputed status".to_string()));
    }

    let target_type = booking.target_type.to_lowercase();
    let is_service_owner = match target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        "business" => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        _ => false,
    };

    if !is_service_owner {
        return Err(AppError::Forbidden("Only the service provider can submit a dispute response".to_string()));
    }

    sqlx::query!(
        "UPDATE bookings SET dispute_response = $1 WHERE id = $2",
        payload.response.trim(),
        id
    )
    .execute(&pool)
    .await?;

    // Notify the client that the provider has responded
    use crate::utils::notifications::notify_and_push;
    notify_and_push(
        &pool,
        &ws_conns,
        booking.client_id,
        "dispute_response",
        "Provider responded to your dispute",
        &format!("The provider has submitted their response to the dispute on booking #{}. An admin will review and mediate.", id),
        Some("booking"),
        Some(id),
    ).await;

    Ok((StatusCode::OK, Json(json!({ "message": "Dispute response submitted" }))))
}

// ── Dispute evidence upload ────────────────────────────────────────────────────

pub async fn upload_dispute_evidence(
    State(pool): State<PgPool>,
    Extension(storage): Extension<SharedStorage>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    mut multipart: Multipart,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let booking = sqlx::query!(
        "SELECT target_type, target_id, client_id, status FROM bookings WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    if booking.status != "disputed" {
        return Err(AppError::BadRequest("Evidence can only be submitted for disputed bookings".to_string()));
    }

    let is_client = booking.client_id == user_id;
    let target_type = booking.target_type.to_lowercase();
    let is_service_owner = match target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        "business" => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        _ => false,
    };

    if !is_client && !is_service_owner {
        return Err(AppError::Forbidden("Only the client or service provider can submit evidence".to_string()));
    }

    let uploader_role = if is_client { "client" } else { "provider" };

    // Parse multipart: collect file bytes and optional caption text field
    let mut file_data: Option<bytes::Bytes> = None;
    let mut file_ext = "jpg".to_string();
    let mut caption: Option<String> = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "caption" {
            let text = field.text().await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            if !text.trim().is_empty() { caption = Some(text.trim().to_string()); }
        } else {
            // treat as the image file
            let file_name = field.file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "upload.jpg".to_string());
            file_ext = std::path::Path::new(&file_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("jpg")
                .to_lowercase();
            let data = field.bytes().await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;
            if !data.is_empty() { file_data = Some(data); }
        }
    }

    let data = file_data.ok_or_else(|| AppError::BadRequest("No image uploaded".to_string()))?;

    // Limit: max 5 evidence images per party per booking
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM dispute_evidence WHERE booking_id = $1 AND uploaded_by = $2",
        id, user_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    if count >= 5 {
        return Err(AppError::BadRequest("Maximum 5 evidence images per party".to_string()));
    }

    let key = generate_key("disputes/evidence", &file_ext);
    let url = storage.save(&key, &data).await
        .map_err(|e| { let _ = tokio::spawn(async move {}); e })?;

    sqlx::query!(
        r#"INSERT INTO dispute_evidence (booking_id, uploaded_by, uploader_role, file_url, caption)
           VALUES ($1, $2, $3, $4, $5)"#,
        id, user_id, uploader_role, url, caption.as_deref()
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "url": url, "message": "Evidence uploaded" }))))
}

// ── Record evidence by URL (Cloudinary upload done on frontend) ───────────────

#[derive(Deserialize)]
pub struct EvidenceUrlPayload {
    pub file_url: String,
    pub caption: Option<String>,
}

pub async fn record_dispute_evidence_url(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<EvidenceUrlPayload>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let booking = sqlx::query!(
        "SELECT target_type, target_id, client_id, status FROM bookings WHERE id = $1",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    if booking.status != "disputed" {
        return Err(AppError::BadRequest("Evidence can only be submitted for disputed bookings".to_string()));
    }

    let is_client = booking.client_id == user_id;
    let target_type = booking.target_type.to_lowercase();
    let is_service_owner = match target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        "business" => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        _ => false,
    };

    if !is_client && !is_service_owner {
        return Err(AppError::Forbidden("Only the client or service provider can submit evidence".to_string()));
    }

    let uploader_role = if is_client { "client" } else { "provider" };

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM dispute_evidence WHERE booking_id = $1 AND uploaded_by = $2",
        id, user_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(0);

    if count >= 5 {
        return Err(AppError::BadRequest("Maximum 5 evidence images per party".to_string()));
    }

    sqlx::query!(
        r#"INSERT INTO dispute_evidence (booking_id, uploaded_by, uploader_role, file_url, caption)
           VALUES ($1, $2, $3, $4, $5)"#,
        id, user_id, uploader_role, payload.file_url, payload.caption.as_deref()
    )
    .execute(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(json!({ "url": payload.file_url, "message": "Evidence recorded" }))))
}

pub async fn get_dispute_evidence(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    // Verify the requester is a party to this booking or an admin
    let booking = sqlx::query!(
        "SELECT target_type, target_id, client_id FROM bookings WHERE id = $1", id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Booking not found".to_string()))?;

    let is_client = booking.client_id == user_id;
    let target_type = booking.target_type.to_lowercase();
    let is_service_owner = match target_type.as_str() {
        "provider" => sqlx::query_scalar!(
            "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        "business" => sqlx::query_scalar!(
            "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
            booking.target_id, user_id
        ).fetch_optional(&pool).await?.is_some(),
        _ => false,
    };
    let is_admin = sqlx::query_scalar!(
        "SELECT is_super_admin FROM admins WHERE user_id = $1", user_id
    )
    .fetch_optional(&pool)
    .await?
    .flatten()
    .unwrap_or(false);

    if !is_client && !is_service_owner && !is_admin {
        return Err(AppError::Forbidden("Access denied".to_string()));
    }

    #[derive(Serialize, sqlx::FromRow)]
    struct EvidenceRow {
        id: i32,
        uploader_role: String,
        file_url: String,
        caption: Option<String>,
        created_at: Option<chrono::NaiveDateTime>,
    }

    let evidence = sqlx::query_as!(
        EvidenceRow,
        "SELECT id, uploader_role, file_url, caption, created_at FROM dispute_evidence WHERE booking_id = $1 ORDER BY created_at ASC",
        id
    )
    .fetch_all(&pool)
    .await?;

    Ok((StatusCode::OK, Json(json!({ "evidence": evidence }))))
}
