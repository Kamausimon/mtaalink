use axum::{
    Router,
    http::StatusCode,
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use serde_json::json;
use chrono::NaiveDateTime;
use validator::Validate;
use crate::extractors::current_user::CurrentUser;

pub fn location_routes(pool:PgPool) -> Router {
    Router ::new ()
        .route("/allcounties", get(get_locations_counties))
        .route("/counties/:county_id/constituencies", get(get_constituencies_by_county))
        .route("/constituencies/:constituency_id/wards", get(get_wards_by_constituency))
        .route("/branches/:branch_id/location", post(create_branch_location))
        .route("/branches/:branch_id/locations", get(get_branch_locations))   
        .route("/providers/:provider_id", get(create_provider_location))
         .route("/search", get(search_business_or_provider_by_location))
        .route("/:id", get(get_location_by_id).put(update_location).delete(delete_location))
        .with_state(pool)
}

#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct Counties{
  pub  id: i32,
   pub  name: String,
}

pub async fn get_locations_counties(State(pool) : State<PgPool>) -> impl IntoResponse {
     let counties_query = "SELECT id, name FROM counties";

     let counties = sqlx::query_as::<_, Counties>(&counties_query)
        .fetch_all(&pool)
        .await;

       match counties {
        Ok(counties) => (StatusCode::OK, Json(json!({
            "status": "success",
            "data": counties
        }))),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": format!("Failed to fetch counties: {}", err)
            }))
        ),
       }
}

// Get constituencies by county
#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct Constituency {
   pub id: i32,
   pub name: String,
}

pub async fn get_constituencies_by_county(
    Path(county_id): Path<i32>,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    let query = "SELECT id, name FROM constituencies WHERE county_id = $1";
    
    let constituencies = sqlx::query_as::<_, Constituency>(query)
        .bind(county_id)
        .fetch_all(&pool)
        .await;

    match constituencies {
        Ok(constituencies) => (
            StatusCode::OK,
          Json(
              json!({
                "status": "success",
                "data": constituencies
            })
          )
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({
                    "status": "error",
                    "message": format!("Failed to fetch constituencies: {}", err)
                })
            )
        ),
    }
}

// Get wards by constituency
#[derive(Serialize, Deserialize, Debug, Clone, Validate)]
pub struct Ward {
  pub  id: i32,
   pub name: String,
}

pub async fn get_wards_by_constituency(
    Path(constituency_id): Path<i32>,
    State(pool): State<PgPool>,
)-> impl IntoResponse {
    let query = "SELECT id, name FROM wards WHERE constituency_id = $1";
    
    let wards = sqlx::query_as::<_, Ward>(query)
        .bind(constituency_id)
        .fetch_all(&pool)
        .await;

    match wards {
        Ok(wards) => (
            StatusCode::OK,
          Json(
              json!({
                "status": "success",
                "data": wards
            }))
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({
                    "status": "error",
                    "message": format!("Failed to fetch wards: {}", err)
                })
            )
        ),
    }
}

// Create a branch location
#[derive(Serialize, Deserialize, Debug, Clone, Validate, sqlx::FromRow)]
pub struct BusinessBranchLocation {
    id: i32,
    business_id: i32,
    #[validate(length(min = 1, max = 100))]
    name: String,
    latitude: f64,
    longitude: f64,
    ward_id: i32,
    #[validate(length(min = 1, max = 10))]
    phone: String,
    #[validate(length(min = 1, max = 255))]
    address: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

pub async fn create_branch_location (
    Path(branch_id): Path<i32>,
    State(pool): State<PgPool>,
     CurrentUser{user_id}: CurrentUser,
     Json(payload): Json<BusinessBranchLocation>
) -> impl IntoResponse {
    //validate the payload
    if let Err(e) = payload.validate(){
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({
                    "status": "error",
                    "message": format!("Validation error: {}", e)
                })
            )
        );
    }

    //check the business id 
    if payload.business_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({
                    "status": "error",
                    "message": "Invalid business ID"
                })
            )
        );
    }

    // Insert the new branch location into the database
    let query = r#"
        INSERT INTO business_branch_locations (business_id, name, latitude, longitude, ward_id, phone, address, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, created_at, updated_at,business_id, name, latitude, longitude,ward_id,phone,address"#;

        let result = sqlx::query_as::<_, BusinessBranchLocation>(query)
        .bind(payload.business_id)
        .bind(payload.name)
        .bind(payload.latitude)
        .bind(payload.longitude)
        .bind(payload.ward_id)
        .bind(payload.phone)
        .bind(payload.address)
        .bind(user_id)
        .fetch_one(&pool)
        .await;

        match result {
            Ok(location) => (
                StatusCode::CREATED,
                Json(
                    json!({
                        "status": "success",
                        "data": location
                    }))
            ),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(
                    json!({
                        "status": "error",
                        "message": format!("Failed to create branch location: {}", err)
                    })
                )
            ),
        }
}

//get the full branch location details
#[derive(Serialize, Deserialize, Debug, Clone, Validate, sqlx::FromRow)]
 pub struct BranchLocationResponse {
    pub id: i32,
    pub business_id: i32,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub ward_id: i32,
    pub ward_name: String, // Assuming you want to include the ward name
    pub constituency_name: String, // Assuming you want to include the constituency name
    pub county_name: String, // Assuming you want to include the county name
    pub phone: String,
    pub address: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,

 }

pub async fn get_branch_locations(
    Path(branch_id): Path<i32>,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    let query = r#"
        SELECT bl.id, bl.business_id, bl.name, bl.latitude, bl.longitude, 
               bl.ward_id, w.name AS ward_name, c.name AS constituency_name, 
               co.name AS county_name, bl.phone, bl.address, 
               bl.created_at, bl.updated_at
        FROM business_branches AS bl
        JOIN wards AS w ON bl.ward_id = w.id
        JOIN constituencies AS c ON w.constituency_id = c.id
        JOIN counties AS co ON c.county_id = co.id
        WHERE bl.business_id = $1"#;

    let locations = sqlx::query_as::<_, BranchLocationResponse>(query)
        .bind(branch_id)
        .fetch_all(&pool)
        .await;

    match locations {
        Ok(locations) => (
            StatusCode::OK,
            Json(
                json!({
                    "status": "success",
                    "data": locations
                })
            )
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({
                    "status": "error",
                    "message": format!("Failed to fetch branch locations: {}", err)
                })
            )
        ),
    }
}


//create an endpoint for provider to create a location
#[derive(Serialize, Deserialize, Debug, Clone, Validate, sqlx::FromRow)]
pub struct ProviderLocation {
    id: i32,
    provider_id: i32,
    #[validate(length(min = 1, max = 100))]
    latitude: f64,
    longitude: f64,
    ward_id: i32,
    #[validate(length(min = 1, max = 10))]
    phone: String,
    #[validate(length(min = 1, max = 255))]
    address: String,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
}

pub async fn create_provider_location(
    Path(provider_id): Path<i32>,
    State(pool): State<PgPool>,
    CurrentUser{user_id}: CurrentUser,
    Json(payload): Json<ProviderLocation>
) -> impl IntoResponse {
    // Validate the payload
    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
    Json(
                json!({
                    "status": "error",
                    "message": format!("Validation error: {}", e)
                })
            )
        );
    }

    // Check the provider id
    if payload.provider_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({
                    "status": "error",
                    "message": "Invalid provider ID"
                })
            )
        );
    }

    // Insert the new provider location into the database
    let query = r#"
        INSERT INTO provider_locations (provider_id, latitude, longitude, ward_id, phone, address, created_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, created_at, updated_at"#;

    let result = sqlx::query_as::<_, ProviderLocation>(query)
        .bind(payload.provider_id)
        .bind(payload.latitude)
        .bind(payload.longitude)
        .bind(payload.ward_id)
        .bind(payload.phone)
        .bind(payload.address)
        .bind(user_id)
        .fetch_one(&pool)
        .await;

    match result {
        Ok(location) => (
            StatusCode::CREATED,
            Json(
                json!({
                    "status": "success",
                    "data": location
                })
            )
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({
                    "status": "error",
                    "message": format!("Failed to create provider location: {}", err)
                })
            )
        ),
    }
}

//endpoint for searching for businesses or service providers by location
#[derive(Deserialize, Serialize, Debug, Clone, Validate)]
pub struct SearchLocation{
 county_id: Option<i32>,
    constituency_id: Option<i32>,
    ward_id: Option<i32>,
    target_type: String, // "business" or "provider"
}



pub async fn search_business_or_provider_by_location(
    Query(params): Query<SearchLocation>,
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    let mut query = String::from("SELECT * FROM ");
    let mut conditions = Vec::new();

    // Determine the target type and set the query accordingly
    if params.target_type == "business" {
        query.push_str("businesses");
    } else if params.target_type == "provider" {
        query.push_str("providers");
    } else {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({
                    "status": "error",
                    "message": "Invalid target type. Must be 'business' or 'provider'."
                })
            )
        );
    }

    // Add conditions based on provided parameters
    if let Some(county_id) = params.county_id {
        conditions.push(format!("county_id = {}", county_id));
    }
    if let Some(constituency_id) = params.constituency_id {
        conditions.push(format!("constituency_id = {}", constituency_id));
    }
    if let Some(ward_id) = params.ward_id {
        conditions.push(format!("ward_id = {}", ward_id));
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    // Execute the query
    let results = sqlx::query(&query)
        .fetch_all(&pool)
        .await;

    match results {
        Ok(data) => (
            StatusCode::OK,
            Json(
                json!({
                    "status": "success",
                    "data": data
                }))
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(
                json!({
                    "status": "error",
                    "message": format!("Failed to search: {}", err)
                })
            )
        ),
    }
}
