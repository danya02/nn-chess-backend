mod engines;

use std::{collections::HashMap, convert::Infallible, num::NonZeroUsize, sync::Arc};

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
        .with_state(ServerState::default());

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
        ],
    })
}
