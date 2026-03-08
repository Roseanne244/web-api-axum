use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ─────────────────────────────────────────────
//  Models
// ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    fn ok(data: T, message: &str) -> Json<Self> {
        Json(Self {
            success: true,
            data: Some(data),
            message: message.to_string(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
}

impl ErrorResponse {
    fn new(message: &str) -> Json<Self> {
        Json(Self {
            success: false,
            message: message.to_string(),
        })
    }
}

// ─────────────────────────────────────────────
//  State (in-memory database)
// ─────────────────────────────────────────────

type Db = Arc<RwLock<HashMap<String, Task>>>;

// ─────────────────────────────────────────────
//  Handlers
// ─────────────────────────────────────────────

/// GET /health — Health check
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "Task API",
        "version": "1.0.0",
        "timestamp": Utc::now().to_rfc3339()
    }))
}

/// GET /tasks — List all tasks
async fn list_tasks(
    State(db): State<Db>,
) -> Json<ApiResponse<Vec<Task>>> {
    let db = db.read().unwrap();
    let tasks: Vec<Task> = db.values().cloned().collect();
    let count = tasks.len();
    ApiResponse::ok(tasks, &format!("{count} tasks found"))
}

/// GET /tasks/:id — Get a single task
async fn get_task(
    State(db): State<Db>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Task>>, (StatusCode, Json<ErrorResponse>)> {
    let db = db.read().unwrap();
    match db.get(&id) {
        Some(task) => Ok(ApiResponse::ok(task.clone(), "Task found")),
        None => Err((
            StatusCode::NOT_FOUND,
            ErrorResponse::new(&format!("Task with id '{id}' not found")),
        )),
    }
}

/// POST /tasks — Create a new task
async fn create_task(
    State(db): State<Db>,
    Json(payload): Json<CreateTaskRequest>,
) -> (StatusCode, Json<ApiResponse<Task>>) {
    if payload.title.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                success: false,
                data: None,
                message: "Title cannot be empty".to_string(),
            }),
        );
    }

    let now = Utc::now().to_rfc3339();
    let task = Task {
        id: Uuid::new_v4().to_string(),
        title: payload.title,
        description: payload.description,
        completed: false,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut db = db.write().unwrap();
    db.insert(task.id.clone(), task.clone());

    (StatusCode::CREATED, ApiResponse::ok(task, "Task created successfully"))
}

/// PUT /tasks/:id — Update a task
async fn update_task(
    State(db): State<Db>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<Json<ApiResponse<Task>>, (StatusCode, Json<ErrorResponse>)> {
    let mut db = db.write().unwrap();

    match db.get_mut(&id) {
        Some(task) => {
            if let Some(title) = payload.title {
                task.title = title;
            }
            if let Some(desc) = payload.description {
                task.description = desc;
            }
            if let Some(completed) = payload.completed {
                task.completed = completed;
            }
            task.updated_at = Utc::now().to_rfc3339();

            Ok(ApiResponse::ok(task.clone(), "Task updated successfully"))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            ErrorResponse::new(&format!("Task '{id}' not found")),
        )),
    }
}

/// DELETE /tasks/:id — Delete a task
async fn delete_task(
    State(db): State<Db>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ErrorResponse>)> {
    let mut db = db.write().unwrap();

    if db.remove(&id).is_some() {
        Ok(ApiResponse::ok(id, "Task deleted successfully"))
    } else {
        Err((
            StatusCode::NOT_FOUND,
            ErrorResponse::new(&format!("Task '{id}' not found")),
        ))
    }
}

// ─────────────────────────────────────────────
//  Main
// ─────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Shared in-memory state
    let db: Db = Arc::new(RwLock::new(HashMap::new()));

    // Seed with a sample task
    {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        db.write().unwrap().insert(
            id.clone(),
            Task {
                id,
                title: "Learn Rust".to_string(),
                description: "Study ownership, traits, and async/await".to_string(),
                completed: false,
                created_at: now.clone(),
                updated_at: now,
            },
        );
    }

    // Router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/tasks", get(list_tasks).post(create_task))
        .route("/tasks/:id", get(get_task).put(update_task).delete(delete_task))
        .with_state(db);

    let addr = "0.0.0.0:3000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    println!("🚀 Server running at http://{addr}");
    println!("📋 Endpoints:");
    println!("   GET    /health");
    println!("   GET    /tasks");
    println!("   POST   /tasks");
    println!("   GET    /tasks/:id");
    println!("   PUT    /tasks/:id");
    println!("   DELETE /tasks/:id");

    axum::serve(listener, app).await.unwrap();
}
