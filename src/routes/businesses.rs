use crate::extractors::current_user::CurrentUser;
use crate::utils::image_upload::save_image_to_fs;
use axum::{
    Router,
    extract::{Json, Query, State, Multipart},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use validator::Validate;

pub fn businesses_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/onboard", post(onboard_business))
        .route("/listBusinesses", get(list_businesses))
        .route("/updateProfile", post(update_business_profile))
        .route("/uploadLogo", post(upload_business_logo))
        .route("/uploadProfilePicture", post(upload_business_profile_picture))
        .route("/uploadCoverPhoto", post(upload_business_cover_photo))
        .with_state(pool.clone())
}

#[derive(Deserialize, Debug, Validate)]
pub struct BusinessOnboardRequest {
    #[validate(length(min = 3))]
    pub business_name: String,
    #[validate(length(min = 10))]
    pub description: String,
    pub category: String,
    pub location: String,
    pub license_number: String,
    #[validate(length(min = 11))]
    pub krapin: String,
    #[validate(length(min = 10))]
    pub phone_number: String,
    #[validate(email)]
    pub email: String,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
}

pub async fn onboard_business(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<BusinessOnboardRequest>,
) -> impl IntoResponse {
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        );
    }

    let exists = sqlx::query_scalar!(
        "SELECT 1 FROM businesses WHERE user_id = $1",
        user_id.parse::<i32>().unwrap()
    )
    .fetch_optional(&pool)
    .await
    .unwrap();

    if exists.is_some() {
        return (
            StatusCode::CONFLICT,
            Json(json!({"error": "Business already onboarded"})),
        );
    }

    let result = sqlx::query!(
        "INSERT INTO businesses (
        user_id, business_name, description, category, location,
        license_number, krapin, phone_number, email, website, whatsapp
    ) VALUES (
        $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
    ) RETURNING id",
        user_id.parse::<i32>().unwrap(),
        payload.business_name,
        payload.description,
        payload.category,
        payload.location,
        payload.license_number,
        payload.krapin,
        payload.phone_number,
        payload.email,
        payload.website,
        payload.whatsapp
    )
    .fetch_one(&pool)
    .await;

    match result {
        Ok(record) => (
            StatusCode::CREATED,
            Json(json!({ "message": "Business onboarded successfully"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

//filter business by category, name and location
#[derive(Deserialize, Debug)]
pub struct BusinessQuery {
    pub category: Option<String>,
    pub business_name: Option<String>,
    pub location: Option<String>,
}

#[derive(Serialize, Debug, sqlx::FromRow)]
struct BusinessProvider {
    pub id: i32,
    pub business_name: String,
    pub description: String,
    pub category: String,
    pub location: String,
    pub phone_number: String,
    pub email: String,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
}

pub async fn list_businesses(
    State(pool): State<PgPool>,
    Query(params): Query<BusinessQuery>,
) -> impl IntoResponse {
    let mut query = String::from(
        r#"
        SELECT 
         b.id, b.business_name, b.description, b.category, b.location,
         b.phone_number, b.email, b.website, b.whatsapp
        FROM businesses b
        JOIN users u ON b.user_id = u.id
        WHERE 1=1
        "#,
    );

    let mut bindings: Vec<String> = Vec::new();
    let mut param_index = 1;

    if let Some(ref category) = params.category {
        query.push_str(&format!(" AND b.category = ${}", param_index));
        param_index += 1;
        bindings.push(category.to_string());
    }

    if let Some(ref business_name) = params.business_name {
        query.push_str(&format!(" AND b.business_name ILIKE ${}", param_index));
        param_index += 1;
        bindings.push(format!("%{}%", business_name));
    }

    if let Some(ref location) = params.location {
        query.push_str(&format!(" AND b.location ILIKE ${}", param_index));
        bindings.push(format!("%{}%", location));
    }

    // Prepare query
    let mut q = sqlx::query_as::<_, BusinessProvider>(&query);
    for bind in bindings {
        q = q.bind(bind);
    }

    match q.fetch_all(&pool).await {
        Ok(bindings) => Json(json!({
            "message": "Businesses fetched successfully",
            "businesses": bindings,

        })),
        Err(e) => Json(json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize, Debug, Validate)]
pub struct BusinessUpdateRequest {
    #[validate(length(min = 3))]
    pub business_name: Option<String>,
    #[validate(length(min = 10))]
    pub description: Option<String>,
    pub location: Option<String>,
    #[validate(length(min = 10))]
    pub phone_number: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub website: Option<String>,
    pub whatsapp: Option<String>,
}

pub async fn update_business_profile(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    Json(payload): Json<BusinessUpdateRequest>,
) -> impl IntoResponse {
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        );
    }

    let mut query = String::from("UPDATE businesses SET ");
    let mut bindings: Vec<String> = Vec::new();
    let mut updates = Vec::new();
    let mut param_index = 1;

    if let Some(ref value) = payload.business_name {
        updates.push(format!("business_name = ${}", param_index));
        bindings.push(value.clone());
        param_index += 1;
    }
    if let Some(ref value) = payload.description {
        updates.push(format!("description = ${}", param_index));
        bindings.push(value.clone());
        param_index += 1;
    }
    if let Some(ref value) = payload.location {
        updates.push(format!("location = ${}", param_index));
        bindings.push(value.clone());
        param_index += 1;
    }
    if let Some(ref value) = payload.phone_number {
        updates.push(format!("phone_number = ${}", param_index));
        bindings.push(value.clone());
        param_index += 1;
    }
    if let Some(ref value) = payload.email {
        updates.push(format!("email = ${}", param_index));
        bindings.push(value.clone());
        param_index += 1;
    }
    if let Some(ref value) = payload.website {
        updates.push(format!("website = ${}", param_index));
        bindings.push(value.clone());
        param_index += 1;
    }
    if let Some(ref value) = payload.whatsapp {
        updates.push(format!("whatsapp = ${}", param_index));
        bindings.push(value.clone());
        param_index += 1;
    }
    if updates.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "No fields to update"})),
        );
    }

    query.push_str(&updates.join(", "));
    query.push_str(&format!("WHERE user_id = ${}", param_index));
   let user_id = user_id.parse::<i32>().unwrap();

    let mut q = sqlx::query(&query);
    for bind in bindings {
        q = q.bind(bind);
    }

    q = q.bind(user_id);

    match q.execute(&pool).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({"message": "Business profile updated successfully"})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

pub async fn upload_business_logo(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    multipart: Multipart,
) -> impl IntoResponse {
    let upload_dir = "uploads/businesses/business_logos";
    match save_image_to_fs(multipart, upload_dir).await {
        Ok(file_name) => {
            let logo_path = format!("{}/{}", upload_dir, file_name);
            let result = sqlx::query!(
                "UPDATE businesses SET logo = $1 WHERE user_id = $2",
                logo_path,
                user_id.parse::<i32>().unwrap()
            )
            .execute(&pool)
            .await;

            match result {
                Ok(_) => (
                    StatusCode::OK,
                    Json(json!({"message": "Logo uploaded successfully", "logo": logo_path})),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e})),
        ),
    }
}

pub async fn upload_business_profile_picture(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    multipart: Multipart,
) -> impl IntoResponse {
    let upload_dir = "uploads/businesses/business_profile_pictures";
    match save_image_to_fs(multipart, upload_dir).await {
        Ok(file_name) => {
            let profile_picture_path = format!("{}/{}", upload_dir, file_name);
            let result = sqlx::query!(
                "UPDATE businesses SET profile_photo = $1 WHERE user_id = $2",
                profile_picture_path,
                user_id.parse::<i32>().unwrap()
            )
            .execute(&pool)
            .await;

            match result {
                Ok(_) => (
                    StatusCode::OK,
                    Json(json!({"message": "Profile picture uploaded successfully", "profile_picture": profile_picture_path})),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e})),
        ),
    }
}

pub async fn upload_business_cover_photo(
    CurrentUser { user_id }: CurrentUser,
    State(pool): State<PgPool>,
    multipart: Multipart,
) -> impl IntoResponse {
    let upload_dir = "uploads/businesses/business_cover_photos";
    match save_image_to_fs(multipart, upload_dir).await {
        Ok(file_name) => {
            let cover_photo_path = format!("{}/{}", upload_dir, file_name);
            let result = sqlx::query!(
                "UPDATE businesses SET cover_photo = $1 WHERE user_id = $2",
                cover_photo_path,
                user_id.parse::<i32>().unwrap()
            )
            .execute(&pool)
            .await;

            match result {
                Ok(_) => (
                    StatusCode::OK,
                    Json(json!({"message": "Cover photo uploaded successfully", "cover_photo": cover_photo_path})),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": e.to_string()})),
                ),
            }
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e})),
        ),
    }
}

