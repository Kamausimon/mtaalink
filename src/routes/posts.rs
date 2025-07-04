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
use chrono::Utc;
use sqlx::{Transaction, Postgres};
 use chrono::DateTime;

pub fn posts_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/createPosts", post(create_posts))
        .route("/getAllPosts", get(get_All_posts)) //public feed
        .route("/getPost/:id", get(get_post_by_id))
        .route("/provider/:id/posts", get(get_posts_by_provider_id))
        .route("/business/:id/posts", get(get_posts_by_business_id))
        .route("/deletePost/:id", post(delete_post))
        .route("/updatePost/:id", post(update_post_and_attachments))
        .with_state(pool)
}

#[derive(Debug, Serialize, Deserialize, Validate, sqlx::FromRow)]
pub struct CreatPost{
    #[validate(length(min = 1, max = 255))]
    pub title: String,
    
    #[validate(length(min = 1, max = 1000))]
    pub content: String,
    
    pub business_id: Option<i32>,
    
    pub provider_id: Option<i32>,


}

pub async fn create_posts(
    State(pool): State<PgPool>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<CreatPost>,
)-> impl IntoResponse {
    let user_id = user_id.parse::<i32>().unwrap_or(0);

    //check whther user is business or provider
   let user_role = sqlx::query_scalar!("SELECT role FROM users WHERE id = $1", user_id)
        .fetch_one(&pool)
        .await;

         match user_role {
    Ok(Some(role)) => {
        if role == "client" {
            return (StatusCode::FORBIDDEN, 
                    Json(json!({"error": "User is not authorized to create posts"})))
                   .into_response();
        }
    },
    Ok(None) => {
        return (StatusCode::NOT_FOUND, 
                Json(json!({"error": "User role not found"})))
               .into_response();
    },
    Err(e) => {
        return (StatusCode::INTERNAL_SERVER_ERROR, 
                Json(json!({"error": format!("Failed to fetch user role: {}", e)})))
               .into_response();
    }
}
 


    if let Err(e) = payload.validate(){
        return (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response();
    }

    //ensure that the provider or business exists
    if let Some(business_id) = payload.business_id {
        let business_exists = sqlx::query!("SELECT id FROM businesses WHERE id = $1", business_id)
            .fetch_optional(&pool)
            .await;
        
      match business_exists {
            Ok(Some(_)) => {},
            Ok(None) => {
                return (StatusCode::BAD_REQUEST, Json(json!({"error": "Business does not exist"}))).into_response();
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to check business existence: {}", e)}))).into_response();
            }
        }

    }

    if let Some(provider_id) = payload.provider_id {
        let provider_exists = sqlx::query!("SELECT id FROM providers WHERE id = $1", provider_id)
            .fetch_optional(&pool)
            .await;
        match provider_exists {
            Ok(Some(_)) => {},
            Ok(None) => {
                return (StatusCode::BAD_REQUEST, Json(json!({"error": "Provider does not exist"}))).into_response();
            }
            Err(e) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to check provider existence: {}", e)}))).into_response();
            }
    }}
        
     let result = sqlx::query!(
        r#"
        INSERT INTO posts (title, content, business_id, provider_id, created_at,updated_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, title, content, business_id, provider_id, created_at,updated_at
        "#,
        payload.title,
        payload.content,
        payload.business_id,
        payload.provider_id,
     Utc::now(),
        Utc::now() // Use current time for both created_at and updated_at
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
#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct Post{
    pub id: i32,
    pub title: String,
    pub content: String,
    pub business_id: Option<i32>,
    pub provider_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn get_post_by_id(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let post = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await;

    match post {
       Ok(posts) => (StatusCode::OK, Json(json!({"post": posts}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to fetch post: {}", e)})))
    }
}

//get posts by provider id
#[derive(Deserialize, Serialize)]
pub struct ProviderPostsQuery {
    pub provider_id: i32,
}

pub async fn get_posts_by_provider_id(
    State(pool): State<PgPool>,
    Path(provider_id): Path<i32>,
) -> impl IntoResponse {
    let posts = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE provider_id = $1")
        .bind(provider_id)
        .fetch_all(&pool)
        .await;

    match posts {
        Ok(posts) => (StatusCode::OK, Json(json!({"posts": posts}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to fetch posts: {}", e)}))).into_response(),
    }
}

//get posts by business id
pub struct BusinessPostsQuery {
    pub business_id: i32,
}


pub async fn get_posts_by_business_id(
    State(pool): State<PgPool>,
    Path(business_id): Path<i32>,
) -> impl IntoResponse {
    let posts = sqlx::query_as::<_, Post>("SELECT * FROM posts WHERE business_id = $1")
        .bind(business_id)
        .fetch_all(&pool)
        .await;

    match posts {
        Ok(posts) => (StatusCode::OK, Json(json!({"posts": posts}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to fetch posts: {}", e)}))).into_response(),
    }
}

//delete post by id
pub async fn delete_post(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
) -> impl IntoResponse {
    //ensure that the post exists
    let post_exists = sqlx::query!("SELECT id FROM posts WHERE id = $1", id)
        .fetch_optional(&pool)
        .await;
    match post_exists {
        Ok(Some(_)) => {},
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Json(json!({"error": "Post not found"}))).into_response();
        }
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to check post existence: {}", e)}))).into_response();
        }
    }

    

    let result = sqlx::query!("DELETE FROM posts WHERE id = $1", id)
        .execute(&pool)
        .await;

    match result {
        Ok(_) => (StatusCode::OK, Json(json!({"message": "Post deleted successfully"}))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Failed to delete post: {}", e)}))).into_response(),
    }
}

#[derive(Deserialize, Serialize, sqlx::FromRow)]
pub struct UpdatePost {
    pub title: Option<String>,
    pub content: Option<String>,
    pub business_id: Option<i32>,
    pub provider_id: Option<i32>,
    pub updated_by: i32,
    pub attachments: Vec<String>, // Moved here
}

pub async fn update_post_and_attachments(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    CurrentUser { user_id }: CurrentUser,
    Json(payload): Json<UpdatePost>,
) -> impl IntoResponse {
    // Start transaction
    let mut tx: Transaction<'_, Postgres> = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to begin transaction: {}", e) }))
            ).into_response();
        }
    };

    // Update post
    let update_result = sqlx::query!(
        r#"
        UPDATE posts 
        SET 
            title = COALESCE($1, title),
            content = COALESCE($2, content)
        WHERE id = $3
        "#,
        payload.title,
        payload.content,
        id
    ).execute(&mut *tx).await;

    if let Err(e) = update_result {
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to update post: {}", e) }))
        ).into_response();
    }

    // Delete old attachments
    if let Err(e) = sqlx::query!(
        "DELETE FROM attachments WHERE post_id = $1",
        id
    ).execute(&mut *tx).await {
        let _ = tx.rollback().await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to delete old attachments: {}", e) }))
        ).into_response();
    }

    //place a limit on the number of attachments
    if payload.attachments.len() > 5 {
        let _ = tx.rollback().await;
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "Too many attachments. Maximum is 5." }))
        ).into_response();
    }

    // Upload new attachments
    for path in &payload.attachments {
        let result = sqlx::query!(
            "INSERT INTO attachments (post_id, file_path, file_type) VALUES ($1, $2, 'image')",
            id,
            path
        ).execute(&mut *tx).await;

        if let Err(e) = result {
            let _ = tx.rollback().await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("Failed to insert attachment: {}", e) }))
            ).into_response();
        }
    }

    // All good — commit
    if let Err(e) = tx.commit().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("Failed to commit transaction: {}", e) }))
        ).into_response();
    }

    (
        StatusCode::OK,
        Json(json!({ "message": "Post and attachments updated successfully" }))
    ).into_response()
}
