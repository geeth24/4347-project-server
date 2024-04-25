use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio_postgres::{Error, NoTls};

#[derive(Serialize)]
struct Message {
    message: String,
}

enum ApiResponse<T> {
    OK,
    Error,
    JsonData(T),
}

impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self {
            Self::OK => (StatusCode::OK).into_response(),
            Self::Error => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            Self::JsonData(data) => (StatusCode::OK, Json(data)).into_response(),
        }
    }
}

#[derive(Clone)]
struct AppState {
    db: Arc<tokio_postgres::Client>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let (client, connection) =
        tokio_postgres::connect("postgres://beam:postgres@localhost/postgres", NoTls)
            .await
            .unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let app_state = AppState {
        db: Arc::new(client),
    };

    let app = Router::new()
        .route("/trainer", get(get_trainers))
        .with_state(Arc::new(app_state));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
struct Trainer {
    trainer_id: i32,
    name: String,
    gym_leader: bool,
}

#[derive(Serialize)]
struct GetTrainersResponse {
    trainers: Vec<Trainer>,
}

async fn get_trainers(State(state): State<Arc<AppState>>) -> ApiResponse<GetTrainersResponse> {
    let db = state.db.clone();

    match db.query("SELECT * FROM trainer", &[]).await {
        Ok(rows) => {
            let mut trainers = Vec::new();
            for r in rows {
                let trainer = Trainer {
                    trainer_id: r.get(0),
                    name: r.get(1),
                    gym_leader: r.get(2),
                };
                trainers.push(trainer);
            }

            tracing::info!("{:?}", trainers);

            ApiResponse::JsonData(GetTrainersResponse { trainers })
        }
        Err(e) => {
            tracing::error!("Failed to fetch trainers: {:?}", e);

            ApiResponse::Error
        }
    }
}
