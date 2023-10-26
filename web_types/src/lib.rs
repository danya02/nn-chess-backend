use serde::{Deserialize, Serialize};

/// The entrypoint to the engine API.
/// Lists what engines are available.
#[derive(Serialize, Deserialize, Clone, PartialEq, Hash, Debug)]
pub struct EngineDirectory {
    pub engines: Vec<EngineRef>,
}

/// A reference to an engine.
#[derive(Serialize, Deserialize, Clone, PartialEq, Hash, Debug)]
pub struct EngineRef {
    pub engine_id: String,
    pub name: String,
    pub entrypoint_url: String,
}

/// Detailed information about an engine.
#[derive(Serialize, Deserialize, Clone, PartialEq, Hash, Debug)]
pub struct EngineDescription {
    pub engine_id: String,
    pub name: String,
    pub text_description: String,
    pub variants: Vec<EngineVariant>,
    pub best_available_variant: EngineVariant,
}

/// Information about a variant of a particular engine.
/// This could be a difficulty setting,
/// or a different machine learning checkpoint.
#[derive(Serialize, Deserialize, Clone, PartialEq, Hash, Debug)]
pub struct EngineVariant {
    pub engine_id: String,
    pub variant_id: String,
    pub name: String,
    pub description: String,
    /// Post a [`GameMoveRequest`] to this URL to receive an engine move.
    pub game_url: String,
}

/// Encodes the game state in order for the engine to make a move.
#[derive(Serialize, Deserialize, Clone, PartialEq, Hash, Debug)]
pub struct GameMoveRequest {
    pub fen: String,
}

impl GameMoveRequest {
    pub fn to_game(&self) -> Option<shakmaty::Chess> {
        Some(
            shakmaty::fen::Fen::from_ascii(self.fen.as_bytes())
                .ok()?
                .into_position(shakmaty::CastlingMode::Standard)
                .ok()?,
        )
    }
}

/// Encodes the engine's move, as well as additional information about it the move.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct GameMoveResponse {
    /// What move the engine is performing, in standard algebraic notation.
    pub move_san: String,
    /// What the game looks like after this move, in FEN.
    pub game_after_fen: String,
    /// Human-readable description of the engine's thoughts regarding the move.
    pub status_text: String,
    /// How long did it take for the engine to respond.
    pub move_timing: std::time::Duration,
}
