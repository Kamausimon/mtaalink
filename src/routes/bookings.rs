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
    pub id: i32,
    pub client_id: i32,
    pub target_type: String,                   // e.g., "business", "provider"
    pub target_id: i32,                        // e.g., business_id or provider_id
    pub branch_id: Option<i32>,
    pub service_id: Option<i32>,                // e.g., branch_id if applicable
    pub service_description: String,           // e.g., "haircut", "plumbing service"
    pub scheduled_time: chrono::NaiveDateTime, // e.g., "2023-10-01 14:00:00"
    pub status: String,
    pub duration: Option<i32>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
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
    let service_id = payload.service_id;
    let service_description = payload.service_description.trim().to_string();
    let scheduled_time = payload.scheduled_time;
let service_duration = if let Some(service_id) = service_id {
    // Try to get the duration from the database
    match sqlx::query!(
        "SELECT duration FROM services WHERE id = $1", service_id
    ).fetch_optional(&pool).await {
        Ok(Some(service)) => service.duration.unwrap_or(60),
        _ => 60, // Default if query fails or returns no rows
    }
} else {
    60 // Default if no service_id provided
};

    if target_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Invalid target ID"})),
        );
    }

    let target_exists = match target_type.as_str() {
        "business" => {
            sqlx::query_scalar!("SELECT id FROM businesses WHERE id = $1", target_id)
                .fetch_optional(&pool)
                .await
        }
        "provider" => {
            sqlx::query_scalar!("SELECT id FROM providers WHERE id = $1", target_id)
                .fetch_optional(&pool)
                .await
        }
        _ => Ok(None),
    };

    match target_exists {
        Ok(Some(_)) => {} // Target exists, proceed
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
        Ok(None) => {} // No existing booking, proceed
        Err(e) => {
            eprintln!("Error checking existing booking: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to check existing booking"})),
            );
        }
    }

    //check if the selected service exists 
  //check if the selected service exists
let existing_service_id_exists = if let Some(service_id_val) = service_id {
    sqlx::query!(
        "SELECT id FROM services WHERE id = $1 AND target_type = $2 AND target_id = $3",
        service_id_val,
        target_type,
        target_id   
    ).fetch_optional(&pool).await
} else {
   //skip if no service_id is provided
    Ok(None)
};

    match existing_service_id_exists {
        Ok(Some(_)) => {}, // Service exists, proceed
        Ok(None) => {
            // If service_id is provided but does not exist, return an error
            if service_id.is_some(){
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "Service ID does not exist"})),
                );
            }
        },
        Err(e) => {
            eprintln!("Error checking service existence: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Failed to check service existence"})),
            );
        }
    }

let result = sqlx::query!(
    r#"
    INSERT INTO bookings (client_id, target_type, target_id, branch_id, service_id, service_description, scheduled_time, duration, status)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
    RETURNING id
    "#,
    client_id,
    target_type,
    target_id,
    branch_id,
    service_id,
    service_description,
    scheduled_time,
    service_duration, // This is your 8th parameter
    "pending"         // This is your 9th parameter
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
    pub client_phone: Option<String>, // Assuming phone is optional
    pub service_name: String,
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

let result = sqlx::query!(
    r#"
    SELECT b.id, b.client_id, b.target_type, b.target_id, b.branch_id, b.service_id, 
           b.service_description, b.scheduled_time, b.status, b.duration, b.created_at, 
           u.username as client_name, u.email as client_email, 
           '' as client_phone,  -- Empty string since phone doesn't exist
           CASE
               WHEN b.service_id IS NOT NULL THEN s.title
               ELSE b.service_description
           END AS service_name
    FROM bookings b
    LEFT JOIN users u ON b.client_id = u.id
    LEFT JOIN services s ON b.service_id = s.id
    WHERE b.target_type = $1 AND b.target_id = $2 AND b.status = $3
    ORDER BY b.scheduled_time DESC
    "#,
    target_type,
    target_id,
    status
)
.fetch_all(&pool)
.await;

    match result {
        Ok(rows) => {
            let bookings: Vec<BookingResponse> = rows.into_iter().map(|row| BookingResponse {
                id: row.id,
                client_id: row.client_id,
                target_type: row.target_type,
                target_id: row.target_id,
                branch_id: row.branch_id,
                service_id: row.service_id,
                service_description: row.service_description.expect("Service description should not be null"),
                scheduled_time: row.scheduled_time,
                status: row.status,
                duration: row.duration.unwrap_or(60), // Default to 60 mins if not specified
                created_at: row.created_at,
                client_name: row.client_name,
                client_email: row.client_email,
                client_phone: row.client_phone,
                service_name: row.service_name.unwrap_or_default(),
            }).collect();
            (
                StatusCode::OK,
                Json(json!({"bookings": bookings})),
            )
        }
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
        "business" => {
            sqlx::query_scalar!(
                "SELECT id FROM businesses WHERE id = $1 AND user_id = $2",
                target_id,
                user_id
            )
            .fetch_optional(&pool)
            .await
        }
        "provider" => {
            sqlx::query_scalar!(
                "SELECT id FROM providers WHERE id = $1 AND user_id = $2",
                target_id,
                user_id
            )
            .fetch_optional(&pool)
            .await
        }
        _ => Ok(None),
    };

    // Check if user owns the target
    match is_owner {
        Ok(Some(_)) => {} // User owns the target, proceed
        Ok(None) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "message": "You don't have permission to update this booking" })),
            );
        }
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

    let booking = sqlx::query!(
        "SELECT id, client_id, target_type,target_id FROM bookings WHERE id = $1",
        id
    )
    .fetch_one(&pool)
    .await;

    match booking {
        Ok(booking) => {
            if booking.client_id != user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "message": "You do not have permission to delete this booking" })),
                );
            }
        }
        Err(e) => {
            eprintln!("Error fetching booking: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": "Failed to fetch the booking" })),
            );
        }
    }

    let result = sqlx::query!("DELETE FROM bookings WHERE id = $1", id)
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
