mod engines;

use std::{collections::HashMap, convert::Infallible, sync::Arc};

use axum::{routing::get, Json, Router};
use fish_teacher::EngineEvaluation;
use shakmaty::{Chess, Move};
use tokio::sync::{mpsc, oneshot, Mutex};
use web_types::{EngineDirectory, EngineRef};

#[derive(Clone, Default)]
struct ServerState {
    pub engine_links: Arc<
        Mutex<
            HashMap<
                String,
                lru::LruCache<
                    String,
                    (
                        tokio::task::JoinHandle<Result<Infallible, anyhow::Error>>,
                        mpsc::Sender<(
                            Chess,
                            oneshot::Sender<(EngineEvaluation, Move, EngineEvaluation)>,
                        )>,
                    ),
                >,
            >,
        >,
    >,
}

#[tokio::main]
async fn main() {
    println!("Starting!");
    let app = Router::new()
        .route("/", get(index))
        .nest("/engines/narrow", engines::narrow_service())
        .nest("/engines/wide", engines::wide_service())
        .nest("/engines/superwide", engines::superwide_service())
        .nest(
            "/engines/superstonkfish",
            engines::superwide_stockfish_service(),
        )
        .nest("/engines/move_counter", engines::move_counter_service())
        .with_state(ServerState::default())
        .layer(tower_http::cors::CorsLayer::permissive());

    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index() -> Json<EngineDirectory> {
    Json(EngineDirectory {
        engines: vec![
            EngineRef {
                engine_id: "narrow".to_string(),
                name: "Narrow Board Evaluator".to_string(),
                entrypoint_url: "https://api.unchessful.games/engines/narrow".to_string(),
            },
            EngineRef {
                engine_id: "wide".to_string(),
                name: "Wide Board Evaluator".to_string(),
                entrypoint_url: "https://api.unchessful.games/engines/wide".to_string(),
            },
            EngineRef {
                engine_id: "superwide".to_string(),
                name: "Superwide Board Evaluator".to_string(),
                entrypoint_url: "https://api.unchessful.games/engines/superwide".to_string(),
            },
            EngineRef {
                engine_id: "smolfish".to_string(),
                name: "Narrow-Stockfish Dilution".to_string(),
                entrypoint_url: "https://api.unchessful.games/engines/narrow/smolfish".to_string(),
            },
            EngineRef {
                engine_id: "widefish".to_string(),
                name: "Wide-Stockfish Dilution".to_string(),
                entrypoint_url: "https://api.unchessful.games/engines/wide/widefish".to_string(),
            },
            EngineRef {
                engine_id: "superstonkfish".to_string(),
                name: "Superwide-Stockfish Dilution".to_string(),
                entrypoint_url: "https://api.unchessful.games/engines/superstonkfish".to_string(),
            },
            EngineRef {
                engine_id: "move_counter_strat".to_string(),
                name: "Move-Counting Strategies".to_string(),
                entrypoint_url: "https://api.unchessful.games/engines/move_counter".to_string(),
            },
        ],
    })
}
