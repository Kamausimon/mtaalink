use axum::{
    extract::{Path, Query,State,Json},
    http::StatusCode,
    response::IntoResponse,
    Router,
    routing::{post, get},
};
use sqlx::PgPool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::extractors::current_user::CurrentUser;
use crate::utils::attachments::upload_attachments;
use validator::Validate;
use chrono::NaiveDateTime;

pub fn posts_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/createPosts", post(create_posts))
        .route("/getAllPosts", get(get_All_posts)) //public feed
        // .route("/getPost/:id", get(get_post_by_id))
        // .route("/provider/:id/posts", get(get_posts_by_provider_id))
        // .route("/business/:id/posts", get(get_posts_by_business_id))
        // .route("/deletePost/:id", post(delete_post))
        // .route("/updatePost/:id", post(update_post))
        .with_state(pool)
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct CreatPost{
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    
    #[validate(length(min = 1, max = 1000))]
    pub content: String,
    
    pub business_id: i32,
    
    pub provider_id: i32,
    
    pub created_by: i32,
    
    pub created_at: NaiveDateTime,
}

pub async fn create_posts(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<CreatPost>,
)-> impl IntoResponse {
    if let Err(e) = payload.validate(){
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response();
    }
        
     let post = sqlx::query!(
        r#"
        INSERT INTO posts (title, content, business_id, provider_id, created_by, created_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
        payload.title,
        payload.content,
        payload.business_id,
        payload.provider_id,
        user_id,
        payload.created_at
    )
    .fetch_one(&pool).await;

    match result {
        Ok(post) => {
            (StatusCode::CREATED, Json(json!({"post_id": post.id}))).into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to create post: {}", e)}))).into_response()
        }
    }


}


#[derive(Deserialize, Serialize)]
pub struct PostQuery {
    pub business_id: Option<i32>,
    pub provider_id: Option<i32>,
}

pub async fn get_All_posts(
    State(pool): State<PgPool>,
    Query(params): Query<PostQuery>,
) -> impl IntoResponse {
    let mut query = "SELECT * FROM posts".to_string();
    let mut conditions = vec![];

    if let Some(business_id) = params.business_id {
        conditions.push(format!("business_id = {}", business_id));
    }
    if let Some(provider_id) = params.provider_id {
        conditions.push(format!("provider_id = {}", provider_id));
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    let posts = sqlx::query_as::<_, Post>(&query)
        .fetch_all(&pool)
        .await;

    match posts {
        Ok(posts) => (StatusCode::OK, Json(posts)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to fetch posts: {}", e)}))).into_response(),
    }
}

//todo implement get_post_by_id, get_posts_by_provider_id, get_posts_by_business_id, delete_post, update_post
