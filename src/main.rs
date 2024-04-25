use axum::{
    extract::{Path, State},
    http::{Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_postgres::NoTls;
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
    dotenv().ok();
    tracing_subscriber::fmt::init();

    let user = std::env::var("POSTGRES_USER").expect("Missing user env var");
    let pass = std::env::var("POSTGRES_PASS").expect("Missing postgres pass");
    let (client, connection) = tokio_postgres::connect(
        format!("postgres://{}:{}@localhost/postgres", user, pass).as_str(),
        NoTls,
    )
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
        .route("/pokemon-abilities/:id", get(get_ability))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers(Any)
                .expose_headers(Any),
        )
        .with_state(Arc::new(app_state));

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
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
    region: String,
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
                let trainer_id: i32 = r.get(0);

                let pokemon_res = db
                    .query(
                        "SELECT pokemon_id FROM trainerspokemon WHERE trainer_id = $1",
                        &[&trainer_id],
                    )
                    .await
                    .unwrap();

                let mut pokemon_list = Vec::new();
                for p_row in pokemon_res {
                    let pokemon_id: i32 = p_row.get(0);
                    let p = db
                        .query(
                            "SELECT * FROM pokemon WHERE pokemon_id = $1",
                            &[&pokemon_id],
                        )
                        .await
                        .unwrap();

                    for pokemon in p {
                        let region_id: i32 = pokemon.get(2);
                        let region_res = db
                            .query(
                                "SELECT region_name FROM region WHERE region_id = $1",
                                &[&region_id],
                            )
                            .await
                            .unwrap();

                        let region = region_res.first().unwrap();
                        let pokemon = Pokemon {
                            pokemon_id: pokemon.get(0),
                            name: pokemon.get(1),
                            region: region.get(0),
                        };

                        pokemon_list.push(pokemon);
                    }
                }

                let trainer = Trainer {
                    trainer_id,
                    name: r.get(1),
                    gym_leader: r.get(2),
                    pokemon: Some(pokemon_list),
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

#[derive(Serialize)]
struct GetAbilityResponse {
    ability: Vec<Ability>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Ability {
    ability_id: i32,
    name: String,
    damage: i32,
    status_effect: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct PokemonAbilities {
    pokemon_id: i32,
    ability_id: i32,
}



async fn get_ability(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i32>,
) -> ApiResponse<GetAbilityResponse> {
    let db = state.db.clone();


    match db
        .query("SELECT * FROM pokemonabilities WHERE pokemon_id = $1", &[&id])
        .await
    {
        Ok(rows) => {
            let mut abilities: Vec<Ability> = Vec::new();
            for r in rows {
                let ability_id: i32 = r.get(1);

                let ability_res = db
                    .query(
                        "SELECT * FROM ability WHERE ability_id = $1",
                        &[&ability_id],
                    )
                    .await
                    .unwrap();

                for ability in ability_res {
                    let ability = Ability {
                        ability_id: ability.get(0),
                        name: ability.get(1),
                        damage: ability.get(2),
                        status_effect: ability.get(3),
                    };
                    abilities.push(ability);
                }
            }

            tracing::info!("{:?}", abilities);

            return ApiResponse::JsonData(GetAbilityResponse { ability: abilities });
        }
        Err(e) => {
            tracing::error!("Failed to fetch abilities: {:?}", e);

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

#[derive(Serialize, Deserialize, Debug)]
struct PokemonFull {
    pokemon_id: i32,
    name: String,
    region: String,
    abilities: Vec<Ability>,
}
#[derive(Serialize)]
struct GetPokemonResponse {
    pokemons: Vec<PokemonFull>,
}

async fn get_pokemon(State(state): State<Arc<AppState>>) -> ApiResponse<GetPokemonResponse> {
    let db = state.db.clone();

    match db.query("SELECT * FROM pokemon", &[]).await {
        Ok(rows) => {
            let mut pokemon_rows = Vec::new();
            for r in rows {
                let region_id: i32 = r.get(2);
                let region_res = db
                    .query(
                        "SELECT region_name FROM region WHERE region_id = $1",
                        &[&region_id],
                    )
                    .await
                    .unwrap();
                let pokemon = Pokemon {
                    pokemon_id: r.get(0),
                    name: r.get(1),
                    region: region_res.first().unwrap().get(0),
                };

                let ability_res = db
                    .query(
                        "SELECT * FROM pokemonabilities WHERE pokemon_id = $1",
                        &[&pokemon.pokemon_id],
                    )
                    .await
                    .unwrap();

                let mut abilities = Vec::new();
                for ability_row in ability_res {
                    let ability_id: i32 = ability_row.get(1);
                    let ability_res = db
                        .query(
                            "SELECT * FROM ability WHERE ability_id = $1",
                            &[&ability_id],
                        )
                        .await
                        .unwrap();

                    for ability in ability_res {
                        let ability = Ability {
                            ability_id: ability.get(0),
                            name: ability.get(1),
                            damage: ability.get(2),
                            status_effect: ability.get(3),
                        };
                        abilities.push(ability);
                    }
                }

                let pokemon = PokemonFull {
                    pokemon_id: pokemon.pokemon_id,
                    name: pokemon.name,
                    region: pokemon.region,
                    abilities,
                };

                pokemon_rows.push(pokemon);
            }

            tracing::info!("{:?}", pokemon_rows);

            ApiResponse::JsonData(GetPokemonResponse {
                pokemons: pokemon_rows,
            })
        }
        Err(e) => {
            tracing::error!("Failed to fetch pokemon: {:?}", e);

            ApiResponse::Error
        }
    }
}

// #[derive(Deserialize)]
// struct CreatePokemonRequest {
//     name: String,
//     region: String,
// }

// async fn create_pokemon(
//     State(state): State<Arc<AppState>>,
//     Json(payload): Json<CreateUserRequest>,
// ) -> ApiResponse<()> {
//     ApiResponse::OK
// }
