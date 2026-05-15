use crate::utils::notifications::notify;
use crate::utils::sms::{booking_reminder_sms, send_sms_best_effort, SmsConfig};
use chrono::{Duration, Utc};
use sqlx::PgPool;

/// Spawns a background loop that checks for upcoming bookings every 15 minutes
/// and sends a reminder SMS + in-app notification ~24 hours before the appointment.
pub fn start_reminder_task(pool: PgPool) {
    tokio::spawn(async move {
        loop {
            // Wait 15 minutes between each pass (first run is 15 min after startup)
            tokio::time::sleep(std::time::Duration::from_secs(15 * 60)).await;
            send_pending_reminders(&pool).await;
        }
    });
}

async fn send_pending_reminders(pool: &PgPool) {
    let now = Utc::now().naive_utc();
    // 2-hour window centred on the 24-hour mark prevents both missed and duplicate sends
    let window_start = now + Duration::hours(23);
    let window_end = now + Duration::hours(25);

    let bookings = match sqlx::query!(
        r#"SELECT b.id, b.client_id, b.scheduled_time,
                  b.service_description, b.target_type, b.target_id
           FROM bookings b
           WHERE b.scheduled_time >= $1
             AND b.scheduled_time <= $2
             AND b.status NOT IN ('cancelled', 'completed')
             AND b.reminder_sent = false"#,
        window_start,
        window_end
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Reminder query failed: {}", e);
            return;
        }
    };

    if bookings.is_empty() {
        return;
    }

    tracing::info!("Sending reminders for {} upcoming bookings", bookings.len());

    let sms_cfg = SmsConfig::from_env().ok();

    for booking in &bookings {
        let scheduled_str = booking.scheduled_time.format("%d %b %Y %H:%M").to_string();
        let service = booking.service_description.as_deref().unwrap_or("your appointment");

        // In-app notification (always attempted)
        let _ = notify(
            pool,
            booking.client_id,
            "booking_reminder",
            "Upcoming Booking Tomorrow",
            &format!(
                "Reminder: you have a booking #{} for {} scheduled for {}",
                booking.id, service, scheduled_str
            ),
            Some("booking"),
            Some(booking.id),
        )
        .await;

        // SMS notification (only if AT is configured and client has a payment phone)
        if let Some(ref cfg) = sms_cfg {
            let phone = sqlx::query_scalar!(
                "SELECT phone_number FROM payments WHERE booking_id = $1
                 ORDER BY created_at DESC LIMIT 1",
                booking.id
            )
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();

            if let Some(phone) = phone {
                let msg = booking_reminder_sms(booking.id, &scheduled_str, service);
                send_sms_best_effort(cfg, &phone, &msg).await;
            }
        }

        // Mark as reminded — if this fails the reminder will re-fire next pass,
        // which is acceptable (client receives a duplicate rather than missing it)
        if let Err(e) = sqlx::query!(
            "UPDATE bookings SET reminder_sent = true WHERE id = $1",
            booking.id
        )
        .execute(pool)
        .await
        {
            tracing::warn!("Failed to mark booking {} as reminded: {}", booking.id, e);
        }
    }
}
