use crate::errors::{AppError, AppResult};
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

pub fn search_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/", get(search))
        .with_state(pool)
}

// ── Query params ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct SearchQuery {
    /// Free-text query — matched against name, description, category fields.
    pub q: Option<String>,
    /// Filter by entity type: "provider", "business". Omit for both.
    #[serde(rename = "type")]
    pub search_type: Option<String>,
    /// Filter by category (partial, case-insensitive).
    pub category: Option<String>,
    /// Filter by location (partial, case-insensitive), e.g. "Kasarani, Nairobi".
    pub location: Option<String>,
    /// User latitude for geo-proximity search.
    pub lat: Option<f64>,
    /// User longitude for geo-proximity search.
    pub lng: Option<f64>,
    /// Search radius in km (default 10.0). Only used when lat+lng are provided.
    pub radius_km: Option<f64>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

// ── Result types ──────────────────────────────────────────────────────────────

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct ProviderResult {
    pub id: i32,
    pub user_id: i32,
    pub service_name: Option<String>,
    pub service_description: Option<String>,
    pub category: Option<String>,
    pub location: Option<String>,
    pub profile_photo: Option<String>,
    pub phone_number: Option<String>,
    pub average_rating: f64,
    pub review_count: i64,
    pub distance_km: Option<f64>,
}

#[derive(Serialize, sqlx::FromRow, Debug)]
pub struct BusinessResult {
    pub id: i32,
    pub user_id: i32,
    pub business_name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub location: Option<String>,
    pub profile_photo: Option<String>,
    pub logo: Option<String>,
    pub phone_number: Option<String>,
    pub average_rating: f64,
    pub review_count: i64,
    pub distance_km: Option<f64>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn search(
    State(pool): State<PgPool>,
    Query(params): Query<SearchQuery>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    match params.search_type.as_deref() {
        None | Some("provider") | Some("business") => {}
        _ => {
            return Err(AppError::BadRequest(
                "type must be 'provider' or 'business'".to_string(),
            ))
        }
    }

    let q = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase);

    // Wrap category in SQL wildcards for ILIKE
    let category = params
        .category
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("%{}%", s.trim()));

    // Wrap location in SQL wildcards for ILIKE
    let location = params
        .location
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(|s| format!("%{}%", s.trim()));

    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let radius_km = params.radius_km.unwrap_or(10.0);
    let lat = params.lat;
    let lng = params.lng;

    let include_providers = !matches!(params.search_type.as_deref(), Some("business"));
    let include_businesses = !matches!(params.search_type.as_deref(), Some("provider"));

    let (providers, businesses) = tokio::try_join!(
        async {
            if include_providers {
                search_providers(&pool, q.as_deref(), category.as_deref(), location.as_deref(), lat, lng, radius_km, per_page, offset).await
            } else {
                Ok(vec![])
            }
        },
        async {
            if include_businesses {
                search_businesses(&pool, q.as_deref(), category.as_deref(), location.as_deref(), lat, lng, radius_km, per_page, offset).await
            } else {
                Ok(vec![])
            }
        }
    )?;

    Ok((
        StatusCode::OK,
        Json(json!({
            "providers": providers,
            "businesses": businesses,
            "total": providers.len() + businesses.len(),
            "page": page,
            "per_page": per_page,
        })),
    ))
}

// ── Provider search ───────────────────────────────────────────────────────────

async fn search_providers(
    pool: &PgPool,
    q: Option<&str>,
    category: Option<&str>,
    location: Option<&str>,
    lat: Option<f64>,
    lng: Option<f64>,
    radius_km: f64,
    limit: i64,
    offset: i64,
) -> Result<Vec<ProviderResult>, AppError> {
    // Params: $1=q, $2=category, $3=lat, $4=lng, $5=radius_km, $6=limit, $7=offset, $8=location
    //
    // NULL-safe filter pattern: ($N IS NULL OR <condition using $N>)
    // When the param is NULL the guard short-circuits, skipping that filter.
    //
    // Haversine distance formula used for geo-proximity; LEAST(1,sqrt(...))
    // guards against floating-point values just above 1.0 that would make asin error.
    let sql = r#"
        SELECT
            p.id,
            p.user_id,
            p.service_name,
            p.service_description,
            p.category,
            p.location,
            p.profile_photo,
            p.phone_number,
            COALESCE(AVG(r.rating)::float8, 0.0::float8) AS average_rating,
            COUNT(DISTINCT r.id)                          AS review_count,
            CASE
                WHEN $3::float8 IS NOT NULL AND $4::float8 IS NOT NULL THEN
                    MIN(6371.0 * 2.0 * asin(LEAST(1.0, sqrt(
                        power(sin(radians(pl.latitude  - $3::float8) / 2.0), 2) +
                        cos(radians($3::float8)) * cos(radians(pl.latitude)) *
                        power(sin(radians(pl.longitude - $4::float8) / 2.0), 2)
                    ))))
                ELSE NULL
            END AS distance_km
        FROM providers p
        LEFT JOIN reviews r
            ON r.target_type = 'provider' AND r.target_id = p.id
        LEFT JOIN provider_locations pl
            ON pl.provider_id = p.id
        WHERE p.approved = true
          AND p.onboarding_completed = true
          AND (
              $1::text IS NULL
              OR to_tsvector('english',
                     coalesce(p.service_name, '') || ' ' ||
                     coalesce(p.service_description, '') || ' ' ||
                     coalesce(p.category, '') || ' ' ||
                     coalesce(p.location, '')
                 ) @@ plainto_tsquery('english', $1::text)
          )
          AND ($2::text IS NULL OR p.category ILIKE $2::text)
          AND ($8::text IS NULL OR p.location ILIKE $8::text)
        GROUP BY p.id
        HAVING (
            $3::float8 IS NULL
            OR $4::float8 IS NULL
            OR MIN(6371.0 * 2.0 * asin(LEAST(1.0, sqrt(
                power(sin(radians(pl.latitude  - $3::float8) / 2.0), 2) +
                cos(radians($3::float8)) * cos(radians(pl.latitude)) *
                power(sin(radians(pl.longitude - $4::float8) / 2.0), 2)
            )))) <= $5::float8
        )
        ORDER BY
            CASE
                WHEN $1::text IS NOT NULL THEN
                    ts_rank(
                        to_tsvector('english',
                            coalesce(p.service_name, '') || ' ' ||
                            coalesce(p.service_description, '') || ' ' ||
                            coalesce(p.category, '') || ' ' ||
                            coalesce(p.location, '')
                        ),
                        plainto_tsquery('english', $1::text)
                    )
                ELSE 0.0
            END DESC,
            average_rating DESC
        LIMIT $6 OFFSET $7
    "#;

    sqlx::query_as::<_, ProviderResult>(sql)
        .bind(q)
        .bind(category)
        .bind(lat)
        .bind(lng)
        .bind(radius_km)
        .bind(limit)
        .bind(offset)
        .bind(location)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}

// ── Business search ───────────────────────────────────────────────────────────

async fn search_businesses(
    pool: &PgPool,
    q: Option<&str>,
    category: Option<&str>,
    location: Option<&str>,
    lat: Option<f64>,
    lng: Option<f64>,
    radius_km: f64,
    limit: i64,
    offset: i64,
) -> Result<Vec<BusinessResult>, AppError> {
    let sql = r#"
        SELECT
            b.id,
            b.user_id,
            b.business_name,
            b.description,
            b.category,
            b.location,
            b.profile_photo,
            b.logo,
            b.phone_number,
            COALESCE(AVG(r.rating)::float8, 0.0::float8) AS average_rating,
            COUNT(DISTINCT r.id)                          AS review_count,
            CASE
                WHEN $3::float8 IS NOT NULL AND $4::float8 IS NOT NULL THEN
                    MIN(6371.0 * 2.0 * asin(LEAST(1.0, sqrt(
                        power(sin(radians(bb.latitude  - $3::float8) / 2.0), 2) +
                        cos(radians($3::float8)) * cos(radians(bb.latitude)) *
                        power(sin(radians(bb.longitude - $4::float8) / 2.0), 2)
                    ))))
                ELSE NULL
            END AS distance_km
        FROM businesses b
        LEFT JOIN reviews r
            ON r.target_type = 'business' AND r.target_id = b.id
        LEFT JOIN business_branches bb
            ON bb.business_id = b.id
        WHERE b.verified = true
          AND b.onboarding_completed = true
          AND (
              $1::text IS NULL
              OR to_tsvector('english',
                     coalesce(b.business_name, '') || ' ' ||
                     coalesce(b.description, '') || ' ' ||
                     coalesce(b.category, '') || ' ' ||
                     coalesce(b.location, '')
                 ) @@ plainto_tsquery('english', $1::text)
          )
          AND ($2::text IS NULL OR b.category ILIKE $2::text)
          AND ($8::text IS NULL OR b.location ILIKE $8::text)
        GROUP BY b.id
        HAVING (
            $3::float8 IS NULL
            OR $4::float8 IS NULL
            OR MIN(6371.0 * 2.0 * asin(LEAST(1.0, sqrt(
                power(sin(radians(bb.latitude  - $3::float8) / 2.0), 2) +
                cos(radians($3::float8)) * cos(radians(bb.latitude)) *
                power(sin(radians(bb.longitude - $4::float8) / 2.0), 2)
            )))) <= $5::float8
        )
        ORDER BY
            CASE
                WHEN $1::text IS NOT NULL THEN
                    ts_rank(
                        to_tsvector('english',
                            coalesce(b.business_name, '') || ' ' ||
                            coalesce(b.description, '') || ' ' ||
                            coalesce(b.category, '') || ' ' ||
                            coalesce(b.location, '')
                        ),
                        plainto_tsquery('english', $1::text)
                    )
                ELSE 0.0
            END DESC,
            average_rating DESC
        LIMIT $6 OFFSET $7
    "#;

    sqlx::query_as::<_, BusinessResult>(sql)
        .bind(q)
        .bind(category)
        .bind(lat)
        .bind(lng)
        .bind(radius_km)
        .bind(limit)
        .bind(offset)
        .bind(location)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)
}
