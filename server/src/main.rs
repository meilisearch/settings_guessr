use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use settings_guessr::{Document, FieldAccumulator, FinalSettings};
use tokio::task;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/health", get(health))
        .route("/guess", post(guess_settings))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)); // 100 MB

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> StatusCode {
    StatusCode::OK
}

#[derive(serde::Deserialize)]
#[serde(transparent)]
struct ArrayOfDocuments(Vec<Document>);

async fn guess_settings(
    Json(ArrayOfDocuments(documents)): Json<ArrayOfDocuments>,
) -> Json<FinalSettings> {
    task::block_in_place(move || {
        let mut accumulator = FieldAccumulator::default();
        for document in &documents {
            accumulator.push(document);
        }
        Json(accumulator.finish())
    })
}
