use std::num::NonZeroUsize;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use fish_teacher::EngineEvaluation;
use shakmaty::{fen::Fen, san::San, Chess, Move, Position};
use tch_tchotchkes::models::eval_superwide;
use tokio::sync::{mpsc, oneshot};
use web_types::{EngineDescription, EngineVariant, GameMoveRequest, GameMoveResponse};

use crate::ServerState;

use super::engine_endpoint_info;

pub(crate) fn service() -> Router<ServerState> {
    Router::new()
        .route("/", get(index))
        .route("/checkpoints/:id", post(get_move).get(engine_endpoint_info))
}

async fn index() -> Json<EngineDescription> {
    let mut variants = vec![];
    let mut max_variant_id = 0;
    let mut max_variant = EngineVariant {
        engine_id: String::new(),
        variant_id: String::new(),
        name: String::new(),
        game_url: String::new(),
    };

    for idx in eval_superwide::get_checkpoint_idxs() {
        let var = EngineVariant {
            engine_id: "wide".to_string(),
            variant_id: format!("chkpoint_{idx}"),
            name: format!("Using checkpoint {idx}"),
            game_url: format!("https://api.unchessful.games/engines/wide/checkpoints/{idx}"),
        };
        if idx >= max_variant_id {
            max_variant = var.clone();
            max_variant_id = idx;
        }
        variants.push(var);
    }
    Json(EngineDescription {
         engine_id: "wide".to_string(),
         name: "Superwide Board Evaluator".to_string(),
         text_description: "Neural network with hidden layers 1536-4096-8192-2048-512-256-128 that outputs board evaluations. Iterated over moves, chooses (almost) best evaluation".to_string(),
         variants: variants,
         best_available_variant: max_variant
    })
}

async fn get_move(
    Path(id): Path<u64>,
    State(s): State<ServerState>,
    Json(req): Json<GameMoveRequest>,
) -> Result<Json<GameMoveResponse>, (StatusCode, String)> {
    // Check that there exists an engine checkpoint with this idx.
    let idxs = tokio::task::spawn_blocking(eval_superwide::get_checkpoint_idxs)
        .await
        .unwrap();

    if !idxs.contains(&id) {
        return Err((
            StatusCode::NOT_FOUND,
            format!("There is no checkpoint ID {id} for superwide model"),
        ));
    }

    // Check that the input FEN is correct.
    let fen = Fen::from_ascii(req.fen.as_bytes());
    let fen = match fen {
        Ok(f) => f,
        Err(why) => return Err((StatusCode::BAD_REQUEST, format!("Invalid FEN: {why}"))),
    };

    let game: Chess = match fen.into_position(shakmaty::CastlingMode::Standard) {
        Ok(c) => c,
        Err(why) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("FEN could not be parsed into a position: {why}"),
            ))
        }
    };

    let (rx, start_instant) = submit_for_analysis(s, game.clone(), id).await;
    let (before, act, after) = match rx.await {
        Ok(v) => v,
        Err(why) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Could not receive response from engine: {why}"),
            ))
        }
    };
    let duration = std::time::Instant::now() - start_instant;

    let game_after = game.clone().play(&act).unwrap();

    Ok(Json(GameMoveResponse {
        move_san: San::from_move(&game, &act).to_string(),
        game_after_fen: Fen::from_position(game_after, shakmaty::EnPassantMode::Legal).to_string(),
        evaluation_before: before.to_numeric_score(),
        evaluation_after: after.to_numeric_score(),
        move_timing: duration,
    }))
}

async fn submit_for_analysis(
    s: ServerState,
    pos: Chess,
    id: u64,
) -> (
    oneshot::Receiver<(EngineEvaluation, Move, EngineEvaluation)>,
    std::time::Instant,
) {
    // Retrieve the corresponding state
    let mut data = s.engine_links.lock().await;
    let lru = data
        .entry("superwide".to_string()) // If there is no entry with the name of the engine, add a new LRU
        .or_insert_with(|| lru::LruCache::new(NonZeroUsize::new(4).unwrap())); // this engine is very large, so only store a small number of them.

    let idstr = format!("{id}");

    // If there is an engine ref already in the LRU, pull it out.
    let engine_ref = lru.pop(&idstr);

    // If there was an engine ref, but it's dead now, then log it and throw it away.
    let engine_ref = if let Some(eref) = engine_ref {
        if eref.0.is_finished() {
            let why = eref.0.await;
            println!("Engine superwide/{id} finished abnormally: {why:?}");
            None
        } else {
            Some(eref)
        }
    } else {
        None
    };

    // If there is no engine ref now, then spawn a new one.
    let engine_ref = if let Some(eref) = engine_ref {
        eref
    } else {
        println!("Spawning new engine superwide/{id}");
        let (tx, rx) = mpsc::channel(8);
        let handle = tokio::task::spawn_blocking(move || eval_superwide::move_predictor(id, rx));
        (handle, tx)
    };

    // Send a board to it.
    let (tx, rx) = oneshot::channel();
    let start = std::time::Instant::now();
    engine_ref.1.send((pos, tx)).await.unwrap();

    // Finally, put it back into the LRU.
    lru.push(idstr, engine_ref);

    (rx, start)
}
