use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use shakmaty::{fen::Fen, san::San, Position};
use web_types::{EngineDescription, EngineVariant, GameMoveRequest, GameMoveResponse};

use crate::ServerState;

use super::engine_endpoint_info;

pub(crate) fn service() -> Router<ServerState> {
    Router::new()
        .route("/", get(index))
        .route(
            "/oppt_moves/is_min/:ismin",
            post(oppt_moves).get(engine_endpoint_info),
        )
        .route(
            "/self_moves/is_min/:ismin",
            post(self_moves).get(engine_endpoint_info),
        )
}

async fn index() -> Json<EngineDescription> {
    let min_oppt_moves = EngineVariant {
        engine_id: "move_counter_strat".to_string(),
        variant_id: "min_oppt_moves".to_string(),
        name: "The Cop".to_string(),
        description: "Tries to put its opponent in a position with the fewest legal moves possible"
            .to_string(),
        game_url: "https://api.unchessful.games/engines/move_counter/oppt_moves/is_min/1"
            .to_string(),
    };

    let max_oppt_moves = EngineVariant {
        engine_id: "move_counter_strat".to_string(),
        variant_id: "max_oppt_moves".to_string(),
        name: "The Paralegal".to_string(),
        description: "Tries to put its opponent in a position with the most legal moves possible"
            .to_string(),
        game_url: "https://api.unchessful.games/engines/move_counter/oppt_moves/is_min/0"
            .to_string(),
    };

    let min_self_moves = EngineVariant {
        engine_id: "move_counter_strat".to_string(),
        variant_id: "min_self_moves".to_string(),
        name: "The Criminal".to_string(),
        description: "Tries to put itself in a position with the fewest legal moves possible"
            .to_string(),
        game_url: "https://api.unchessful.games/engines/move_counter/self_moves/is_min/1"
            .to_string(),
    };

    let max_self_moves = EngineVariant {
        engine_id: "move_counter_strat".to_string(),
        variant_id: "max_self_moves".to_string(),
        name: "The Lawyer".to_string(),
        description: "Tries to put itself in a position with the most legal moves possible"
            .to_string(),
        game_url: "https://api.unchessful.games/engines/move_counter/self_moves/is_min/0"
            .to_string(),
    };

    let variants = vec![
        max_self_moves,
        min_self_moves,
        max_oppt_moves,
        min_oppt_moves.clone(),
    ];

    Json(EngineDescription {
        engine_id: "move_counter_strat".to_string(),
        name: "Move-Counting Strategies".to_string(),
        text_description:
            "Deterministic strategies based on counting the number of legal moves available"
                .to_string(),
        variants,
        best_available_variant: min_oppt_moves,
    })
}

async fn self_moves(
    Path(is_min): Path<u8>,
    Json(req): Json<GameMoveRequest>,
) -> Result<Json<GameMoveResponse>, (StatusCode, &'static str)> {
    let start = std::time::Instant::now();

    let game = req
        .to_game()
        .ok_or((StatusCode::BAD_REQUEST, "Cannot parse the FEN into a game"))?;

    let is_min = is_min > 0;

    let mut best_move = game
        .legal_moves()
        .first()
        .ok_or((
            StatusCode::BAD_REQUEST,
            "This game has no legal moves available for me to take",
        ))?
        .clone();
    let mut best_move_score = if is_min { f64::MAX } else { f64::MIN };
    let mut oppt_responses = 0;

    {
        let mut fold_move = |act, score, responses| {
            if (score < best_move_score) == (is_min) {
                best_move = act;
                best_move_score = score;
                oppt_responses = responses;
            }
        };
        // Try every move
        for my_move in game.legal_moves() {
            let mut responses = vec![];
            // Play the move, then look at every enemy's response.
            let board_after_my = game.clone().play(&my_move).unwrap();
            for their_move in board_after_my.legal_moves() {
                // For the enemy's response, see how many legal moves I would have.
                let board_after_my_their = board_after_my.clone().play(&their_move).unwrap();
                responses.push(board_after_my_their.legal_moves().len());
            }

            // Average over the moves I would have available
            let sum = responses.iter().sum::<usize>() as f64;
            let average_moves_per_move = sum / (responses.len() as f64).max(1.0);
            fold_move(my_move.clone(), average_moves_per_move, responses.len());
        }
    }

    Ok(Json(GameMoveResponse {
        move_san: San::from_move(&game, &best_move).to_string(),
        game_after_fen: Fen::from_position(
            game.clone().play(&best_move).unwrap(),
            shakmaty::EnPassantMode::Legal,
        )
        .to_string(),
        status_text: format!("After move, I will have {best_move_score} moves on average (across {oppt_responses} of opponent's responses)"),
        move_timing: std::time::Instant::now() - start,
    }))
}

async fn oppt_moves(
    Path(is_min): Path<u8>,
    Json(req): Json<GameMoveRequest>,
) -> Result<Json<GameMoveResponse>, (StatusCode, &'static str)> {
    let start = std::time::Instant::now();

    let game = req
        .to_game()
        .ok_or((StatusCode::BAD_REQUEST, "Cannot parse the FEN into a game"))?;

    let is_min = is_min > 0;

    let mut best_move = game
        .legal_moves()
        .first()
        .ok_or((
            StatusCode::BAD_REQUEST,
            "This game has no legal moves available for me to take",
        ))?
        .clone();
    let mut best_move_score = if is_min { usize::MAX } else { usize::MIN };

    {
        let mut fold_move = |act, score| {
            if (score < best_move_score) == (is_min) {
                best_move = act;
                best_move_score = score;
            }
        };
        // Try every move
        for my_move in game.legal_moves() {
            // Play the move, then look at every enemy's response.
            let board_after_my = game.clone().play(&my_move).unwrap();
            let their_move_count = board_after_my.legal_moves().len();

            fold_move(my_move.clone(), their_move_count);
        }
    }

    Ok(Json(GameMoveResponse {
        move_san: San::from_move(&game, &best_move).to_string(),
        game_after_fen: Fen::from_position(
            game.clone().play(&best_move).unwrap(),
            shakmaty::EnPassantMode::Legal,
        )
        .to_string(),
        status_text: format!("After move, opponent will have {best_move_score} moves available"),
        move_timing: std::time::Instant::now() - start,
    }))
}
