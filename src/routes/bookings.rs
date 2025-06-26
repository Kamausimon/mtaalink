use crate::extractors::current_user::CurrentUser;
use axum::{
    Router,
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
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
        .route("/getBookings/received", get(get_bookings_received)) //can be used by both businesses and providers
        .route("/:id", get(get_booking_by_id))
        .route("/:id/status", post(update_booking)) //used to update the status of a booking to accepted, rejected, or completed
        .route("/:id/delete", post(delete_booking)) //used to delete a booking
        .route("/:id/reschedule", post(reschedule_booking)) //used to reschedule a booking
        .with_state(pool)
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Debug)]
pub struct Booking {
    pub client_id: i32,
    pub target_type: String,                   // e.g., "business", "provider"
    pub target_id: i32,                        // e.g., business_id or provider_id
    pub branch_id: Option<i32>,                // e.g., branch_id if applicable
    pub service_description: String,           // e.g., "haircut", "plumbing service"
    pub scheduled_time: chrono::NaiveDateTime, // e.g., "2023-10-01 14:00:00"
}

pub async fn create_booking(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<Booking>,
) -> impl IntoResponse {
    let target_type = payload.target_type.to_lowercase();

    if target_type != "business" && target_type != "provider" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid target type"})),
        );
    }

    let user_id = user_id.parse::<i32>().unwrap_or(0);
    let client_id = user_id; // Assuming the booking is made by the client themselves
    let target_id = payload.target_id;
    let branch_id = payload.branch_id;
    let service_description = payload.service_description.trim().to_string();
    let scheduled_time = payload.scheduled_time;

    if target_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid target ID"})),
        );
    }

   let target_exists = match target_type.as_str() {
    "business" => sqlx::query_scalar!(
        "SELECT id FROM businesses WHERE id = $1",
        target_id
    )
    .fetch_optional(&pool)
    .await,
    "provider" => sqlx::query_scalar!(
        "SELECT id FROM providers WHERE id = $1",
        target_id
    )
    .fetch_optional(&pool)
    .await,
    _ => Ok(None),
};

    match target_exists {
        Ok(Some(_)) => {}, // Target exists, proceed
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "Target ID does not exist"})),
            );
        }
        Err(e) => {
            eprintln!("Error checking target existence: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to check target existence"})),
            );
        }
    }

    if scheduled_time < chrono::Local::now().naive_local() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Scheduled time cannot be in the past"})),
        );
    }
    //check if timme has been booked if so book plus 30 minutes later 
    let existing_booking = sqlx::query!(
        "SELECT id FROM bookings WHERE target_type = $1 AND target_id = $2 AND scheduled_time = $3",
        target_type,
        target_id,
        scheduled_time
    )
    .fetch_optional(&pool)
    .await;

    match existing_booking {
        Ok(Some(_)) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "This time slot has already been booked"})),
            );
        }
        Ok(None) => {}, // No existing booking, proceed
        Err(e) => {
            eprintln!("Error checking existing booking: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to check existing booking"})),
            );
        }
    }

    let result = sqlx::query!(
        r#"
        INSERT INTO bookings (client_id, target_type, target_id, branch_id, service_description, scheduled_time, status)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
        client_id,
        target_type,
        target_id,
        branch_id,
        service_description,
        scheduled_time,
        "pending"
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(record) => (
            StatusCode::CREATED,
            Json(json!({
                "message": "Booking created successfully",
                "booking_id": record.id
            })),
        ),
        Err(e) => {
            eprintln!("Error creating booking: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to create booking"})),
            )
        }
    }
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
) -> impl IntoResponse {
    let user_id = user_id.parse::<i32>().unwrap_or(0);

    let mut sql = String::from("SELECT * FROM bookings WHERE client_id = $1");

    if let Some(ref status) = params.status {
        sql.push_str(" AND status = ");
        sql.push_str(&format!("{}", status));
    }

    if let Some(ref target_type) = params.target_type {
        sql.push_str(" AND target_type = ");
        sql.push_str(&format!("{}", target_type));
    }

    sql.push_str(" ORDER BY scheduled_time DESC");

    let bookings = sqlx::query_as::<_, Booking>(&sql)
        .bind(user_id)
        .fetch_all(&pool)
        .await;

    match bookings {
        Ok(bookings) => (
            StatusCode::OK,
            Json(json!({"all the related bookings": bookings})),
        ),
        Err(e) => {
            eprintln!("error fetching bookings {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!("There was an error fetching the results")),
            )
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookingsQueryByReceiver {
    target_type: String, //can be provider or business
    target_id: i32,
    status: String, //can be pending, confirmed, cancelled or completed
}

pub async fn get_bookings_received(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Query(params): Query<BookingsQueryByReceiver>,
) -> impl IntoResponse {
    let user_id = user_id.parse::<i32>().unwrap_or(0);
    let target_type = params.target_type.to_lowercase();
    let target_id = params.target_id;
    let status = params.status;

    if !["provider", "business"].contains(&target_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"message": "Invalid target type"})),
        );
    }

    if target_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"message": "Invalid target ID"})),
        );
    }

    let result = sqlx::query_as::<_, Booking>(
        "SELECT id, client_id, target_type, target_id, branch_id, service_description, scheduled_time, status
         FROM bookings
         WHERE target_type = $1 AND target_id = $2 AND status = $3
         ORDER BY scheduled_time DESC"
    )
    .bind(target_type)
    .bind(target_id)
    .bind(status)
    .fetch_all(&pool)
    .await;

    match result {
        Ok(bookings) => (StatusCode::OK, Json(json!({ "bookings": bookings }))),
        Err(e) => {
            eprintln!("Error fetching the bookings: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to fetch the bookings" })),
            )
        }
    }
}

#[derive(Deserialize, Serialize, Debug, sqlx::FromRow)]
pub struct BookingByIdQuery {
    pub id: i32,
}

pub async fn get_booking_by_id(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    if id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid booking ID" })),
        );
    }

    let result =
        sqlx::query_as::<_, Booking>("SELECT * FROM bookings WHERE client_id = $1 AND id = $2")
            .bind(user_id.parse::<i32>().unwrap_or(0))
            .bind(id)
            .fetch_one(&pool)
            .await;

    match result {
        Ok(booking) => (StatusCode::OK, Json(json!({ "booking": booking }))),
        Err(e) => {
            eprintln!("There was an error getting the booking: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to get the booking" })),
            )
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BookingUpdate {
    status: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UpdateQuery {
    target_id: i32,
    target_type: String,
}

pub async fn update_booking(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Query(params): Query<UpdateQuery>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<BookingUpdate>,
) -> impl IntoResponse {
    let target_type = params.target_type.to_lowercase();
    let target_id = params.target_id;
    let user_id = user_id.parse::<i32>().unwrap_or(0);
    let status = payload.status.to_lowercase();
    // status: String, //can be pending, confirmed, cancelled or completed

    if !["provider", "business"].contains(&target_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target type" })),
        );
    }

    if target_id <= 0 || id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid target ID or booking ID" })),
        );
    }

  // First check if the current user owns this business/provider
let is_owner = match target_type.as_str() {
    "business" => sqlx::query_scalar!(
        "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
        target_id,
        user_id
    )
    .fetch_optional(&pool)
    .await,
    "provider" => sqlx::query_scalar!(
        "SELECT id FROM providers WHERE id = $1 AND user_id = $2", 
        target_id,
        user_id
    )
    .fetch_optional(&pool)
    .await,
    _ => Ok(None),
};

// Check if user owns the target
match is_owner {
    Ok(Some(_)) => {}, // User owns the target, proceed
    Ok(None) => {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "message": "You don't have permission to update this booking" })),
        );
    },
    Err(e) => {
        eprintln!("Error checking ownership: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "message": "Failed to check permissions" })),
        );
    }
}

// Then check if the booking exists (without the client_id check)
let booking_exists = sqlx::query!(
    "SELECT id FROM bookings WHERE id = $1 AND target_type = $2 AND target_id = $3",
    id,
    target_type,
    target_id
)
.fetch_optional(&pool)
.await;

// Finally update without the client_id restriction
let result = sqlx::query!(
    r#"
    UPDATE bookings
    SET status = $1
    WHERE id = $2 AND target_type = $3 AND target_id = $4
    "#,
    status,
    id,
    target_type,
    target_id
)
.execute(&pool)
.await;



    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Status updated successfully" })),
        ),
        Err(e) => {
            eprintln!("There was an error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to update the booking status" })),
            )
        }
    }
}

pub async fn delete_booking(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
) -> impl IntoResponse {
    if id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid booking ID" })),
        );

    }

    let user_id = user_id.parse::<i32>().unwrap_or(0);

    let booking  = sqlx::query!(
        "SELECT id, client_id, target_type,target_id FROM bookings WHERE id = $1",
        id
    ).fetch_one(&pool).await;

    match booking {
        Ok(booking) => {
            if booking.client_id != user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "message": "You do not have permission to delete this booking" })),
                );
            }
        },
        Err(e) => {
            eprintln!("Error fetching booking: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to fetch the booking" })),
            );
        }
    }

    let result = sqlx::query!(
        "DELETE FROM bookings WHERE id = $1",
        id
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Booking deleted successfully" })),
        ),
        Err(e) => {
            eprintln!("There was an error deleting the booking: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to delete the booking" })),
            )
        }
    }
}

#[derive(Deserialize, Serialize, Debug, sqlx::FromRow)]
pub struct ReschedulePayload {
    pub scheduled_time: NaiveDateTime, // e.g., "2023-10-01 15:00:00"
}

pub async fn reschedule_booking(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<ReschedulePayload>,
) -> impl IntoResponse {
    if id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "Invalid booking ID" })),
        );
    }

    let user_id = user_id.parse::<i32>().unwrap_or(0);
    let new_scheduled_time = payload.scheduled_time;

    if new_scheduled_time < chrono::Local::now().naive_local() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "message": "New scheduled time cannot be in the past" })),
        );
    }

    let result = sqlx::query!(
        r#"
        UPDATE bookings
        SET scheduled_time = $1
        WHERE id = $2 AND client_id = $3
        "#,
        new_scheduled_time,
        id,
        user_id
    )
    .execute(&pool)
    .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "message": "Booking rescheduled successfully" })),
        ),
        Err(e) => {
            eprintln!("There was an error rescheduling the booking: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to reschedule the booking" })),
            )
        }
    }
}
