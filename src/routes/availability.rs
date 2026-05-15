use crate::errors::{AppError, AppResult};
use crate::extractors::current_user::CurrentUser;
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashSet;

pub fn availability_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/provider/:id", get(get_availability).put(set_availability))
        .route("/provider/:id/slots", get(get_available_slots))
        .with_state(pool)
}

// ── Constants / helpers ───────────────────────────────────────────────────────

const VALID_DAYS: &[&str] = &[
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

fn parse_time(s: &str) -> AppResult<NaiveTime> {
    NaiveTime::parse_from_str(s, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M"))
        .map_err(|_| AppError::BadRequest(format!("Invalid time '{}'. Use HH:MM or HH:MM:SS", s)))
}

fn weekday_name(w: Weekday) -> &'static str {
    match w {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.trim().chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct AvailabilityRow {
    pub id: i32,
    pub day: String,
    pub start_time: NaiveTime,
    pub end_time: NaiveTime,
    pub is_available: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct DaySchedule {
    pub day: String,
    /// Required when is_available = true. Format: HH:MM or HH:MM:SS
    pub start_time: Option<String>,
    /// Required when is_available = true. Format: HH:MM or HH:MM:SS
    pub end_time: Option<String>,
    pub is_available: bool,
}

#[derive(Deserialize, Debug)]
pub struct SlotsQuery {
    /// Date to check in YYYY-MM-DD format.
    pub date: NaiveDate,
    /// Length of each slot in minutes (default 60, range 15–480).
    pub slot_minutes: Option<i64>,
}

// ── GET /availability/provider/:id ───────────────────────────────────────────

pub async fn get_availability(
    State(pool): State<PgPool>,
    Path(provider_id): Path<i32>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    sqlx::query_scalar!("SELECT id FROM providers WHERE id = $1", provider_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    let schedule = sqlx::query_as::<_, AvailabilityRow>(
        r#"SELECT id, day, start_time, end_time, is_available
           FROM provider_availability
           WHERE provider_id = $1
           ORDER BY CASE day
               WHEN 'Monday'    THEN 1 WHEN 'Tuesday'   THEN 2
               WHEN 'Wednesday' THEN 3 WHEN 'Thursday'  THEN 4
               WHEN 'Friday'    THEN 5 WHEN 'Saturday'  THEN 6
               WHEN 'Sunday'    THEN 7 ELSE 8
           END"#,
    )
    .bind(provider_id)
    .fetch_all(&pool)
    .await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "provider_id": provider_id, "schedule": schedule })),
    ))
}

// ── PUT /availability/provider/:id ───────────────────────────────────────────

pub async fn set_availability(
    State(pool): State<PgPool>,
    Path(provider_id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    Json(schedule): Json<Vec<DaySchedule>>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    // Ownership check
    sqlx::query_scalar!(
        "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
        provider_id,
        user_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::Forbidden("You do not own this provider profile".to_string()))?;

    if schedule.is_empty() {
        return Err(AppError::BadRequest("Schedule cannot be empty".to_string()));
    }

    // Validate all entries up front before touching the DB
    let mut seen_days = HashSet::new();
    for entry in &schedule {
        let day = capitalize(&entry.day);
        if !VALID_DAYS.contains(&day.as_str()) {
            return Err(AppError::BadRequest(format!(
                "Invalid day '{}'. Use full names e.g. Monday",
                entry.day
            )));
        }
        if !seen_days.insert(day.clone()) {
            return Err(AppError::BadRequest(format!("Duplicate entry for {}", day)));
        }
        if entry.is_available {
            let start_str = entry
                .start_time
                .as_deref()
                .ok_or_else(|| AppError::BadRequest(format!("start_time required for {}", day)))?;
            let end_str = entry
                .end_time
                .as_deref()
                .ok_or_else(|| AppError::BadRequest(format!("end_time required for {}", day)))?;
            let t_start = parse_time(start_str)?;
            let t_end = parse_time(end_str)?;
            if t_end <= t_start {
                return Err(AppError::BadRequest(format!(
                    "end_time must be after start_time for {}",
                    day
                )));
            }
        }
    }

    // Full replace in a transaction
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "DELETE FROM provider_availability WHERE provider_id = $1",
        provider_id
    )
    .execute(&mut *tx)
    .await?;

    let midnight = NaiveTime::from_hms_opt(0, 0, 0).expect("valid time");

    for entry in &schedule {
        let day = capitalize(&entry.day);
        let (start_time, end_time) = if entry.is_available {
            (
                parse_time(entry.start_time.as_deref().unwrap())?,
                parse_time(entry.end_time.as_deref().unwrap())?,
            )
        } else {
            (midnight, midnight)
        };

        sqlx::query!(
            r#"INSERT INTO provider_availability (provider_id, day, start_time, end_time, is_available)
               VALUES ($1, $2, $3, $4, $5)"#,
            provider_id,
            day,
            start_time,
            end_time,
            entry.is_available
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok((
        StatusCode::OK,
        Json(json!({ "message": "Availability updated successfully" })),
    ))
}

// ── GET /availability/provider/:id/slots ─────────────────────────────────────

pub async fn get_available_slots(
    State(pool): State<PgPool>,
    Path(provider_id): Path<i32>,
    Query(params): Query<SlotsQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let slot_minutes = params.slot_minutes.unwrap_or(60).clamp(15, 480);
    let date = params.date;

    sqlx::query_scalar!("SELECT id FROM providers WHERE id = $1", provider_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Provider not found".to_string()))?;

    let day_name = weekday_name(date.weekday());

    let avail = sqlx::query!(
        "SELECT start_time, end_time, is_available
         FROM provider_availability
         WHERE provider_id = $1 AND day = $2",
        provider_id,
        day_name
    )
    .fetch_optional(&pool)
    .await?;

    let (start_time, end_time) = match avail {
        None => {
            return Ok((
                StatusCode::OK,
                Json(json!({
                    "date": date.to_string(),
                    "day": day_name,
                    "available_slots": [],
                    "message": "No availability set for this day"
                })),
            ));
        }
        Some(row) => {
            if !row.is_available.unwrap_or(true) {
                return Ok((
                    StatusCode::OK,
                    Json(json!({
                        "date": date.to_string(),
                        "day": day_name,
                        "available_slots": [],
                        "message": "Provider is not available on this day"
                    })),
                ));
            }
            (row.start_time, row.end_time)
        }
    };

    // Generate all slots within the availability window
    let slot_dur = Duration::minutes(slot_minutes);
    let mut all_slots: Vec<NaiveTime> = Vec::new();
    let mut cursor = start_time;
    while cursor + slot_dur <= end_time && all_slots.len() < 96 {
        all_slots.push(cursor);
        cursor += slot_dur;
    }

    // Fetch booked times on this date (exclude cancelled bookings)
    let booked: Vec<NaiveDateTime> = sqlx::query_scalar(
        r#"SELECT scheduled_time FROM bookings
           WHERE target_type = 'provider' AND target_id = $1
             AND status <> 'cancelled'
             AND DATE(scheduled_time) = $2"#,
    )
    .bind(provider_id)
    .bind(date)
    .fetch_all(&pool)
    .await
    .map_err(AppError::Database)?;

    let booked_times: HashSet<NaiveTime> = booked.iter().map(|dt| dt.time()).collect();

    let available_slots: Vec<String> = all_slots
        .into_iter()
        .filter(|t| !booked_times.contains(t))
        .map(|t| t.format("%H:%M").to_string())
        .collect();

    let total = available_slots.len();

    Ok((
        StatusCode::OK,
        Json(json!({
            "date": date.to_string(),
            "day": day_name,
            "slot_minutes": slot_minutes,
            "available_slots": available_slots,
            "total_available": total,
        })),
    ))
}
