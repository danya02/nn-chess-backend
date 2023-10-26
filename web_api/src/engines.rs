mod move_counter_strat;
mod narrow;
mod superwide;
mod wide;

use axum::http::StatusCode;
pub(crate) use move_counter_strat::service as move_counter_service;
pub(crate) use narrow::service as narrow_service;
pub(crate) use superwide::service as superwide_service;
pub(crate) use superwide::stockfish_service as superwide_stockfish_service;
pub(crate) use wide::service as wide_service;

pub async fn engine_endpoint_info() -> (StatusCode, &'static str) {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        "You have found an engine endpoint! POST a JSON object like: \n\n{\"fen\": \"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1\"}\n\nto get this engine's chosen move in that board position.",
    )
}
