use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio_postgres::{Error, NoTls};
use tower_http::cors::{Any, CorsLayer};

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
        .route("/trainer/:id", get(get_trainer))
        .route("/trainer/:id", delete(delete_trainer))
        .route("/trainer", post(create_trainer))
        .route("/pokemon", get(get_pokemon))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers(Any),
        )
        .with_state(Arc::new(app_state));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
struct Trainer {
    trainer_id: i32,
    name: String,
    gym_leader: bool,
    pokemon: Option<Vec<Pokemon>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Pokemon {
    pokemon_id: i32,
    name: String,
    region_id: i32,
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
                let pokemon_res = db
                    .query(
                        "SELECT pokemon_id FROM trainerspokemon WHERE trainer_id = $1",
                        &[&r.get(0)],
                    )
                    .await
                    .unwrap();

                let pokemon = Vec::new();
                for r in pokemon_res {
                    let p = db.query("SELECT * FROM pokemon WHERE pokemon_id = $1", &[&r.get(0)]);
                }

                let trainer = Trainer {
                    trainer_id: r.get(0),
                    name: r.get(1),
                    gym_leader: r.get(2),
                    pokemon: None,
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

#[derive(Serialize)]
struct GetTrainerResponse {
    trainers: Vec<Trainer>,
}

async fn get_trainer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResponse<GetTrainerResponse> {
    let db = state.db.clone();

    match db
        .query("SELECT * FROM trainer WHERE trainer_id = $1", &[&id])
        .await
    {
        Ok(rows) => {
            let mut trainers = Vec::new();
            for r in rows {
                let trainer = Trainer {
                    trainer_id: r.get(0),
                    name: r.get(1),
                    gym_leader: r.get(2),
                    pokemon: None,
                };
                trainers.push(trainer);
            }

            tracing::info!("{:?}", trainers);

            return ApiResponse::JsonData(GetTrainerResponse { trainers });
        }
        Err(e) => {
            tracing::error!("Failed to fetch trainers: {:?}", e);

            return ApiResponse::Error;
        }
    }
}

#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    gym_leader: bool,
}

async fn create_trainer(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserRequest>,
) -> ApiResponse<()> {
    let db = state.db.clone();

    match db
        .execute(
            "INSERT INTO trainer (name, gym_leader) VALUES ($1, $2)",
            &[&payload.name, &payload.gym_leader],
        )
        .await
    {
        Ok(_) => ApiResponse::OK,
        Err(e) => {
            tracing::error!("Failed to create trainer: {}", e);

            ApiResponse::Error
        }
    }
}

async fn delete_trainer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResponse<()> {
    let db = state.db.clone();

    match db
        .execute("DELETE FROM trainer WHERE trainer_id = $1", &[&id])
        .await
    {
        Ok(_) => ApiResponse::OK,
        Err(e) => {
            tracing::error!("Failed to delete trainer: {}", e);

            ApiResponse::Error
        }
    }
}

#[derive(Serialize)]
struct GetPokemonResponse {
    trainers: Vec<Pokemon>,
}

async fn get_pokemon(State(state): State<Arc<AppState>>) -> ApiResponse<GetPokemonResponse> {
    let db = state.db.clone();

    match db.query("SELECT * FROM pokemon", &[]).await {
        Ok(rows) => {
            let mut trainers = Vec::new();
            for r in rows {
                let trainer = Pokemon {
                    pokemon_id: r.get(0),
                    name: r.get(1),
                    region_id: r.get(2),
                };
                trainers.push(trainer);
            }

            tracing::info!("{:?}", trainers);

            ApiResponse::JsonData(GetPokemonResponse { trainers })
        }
        Err(e) => {
            tracing::error!("Failed to fetch pokemon: {:?}", e);

            ApiResponse::Error
        }
    }
}

#[derive(Deserialize)]
struct CreatePokemonRequest {
    name: String,
    region: String,
}

async fn create_pokemon(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateUserRequest>,
) -> ApiResponse<()> {
    ApiResponse::OK
}
