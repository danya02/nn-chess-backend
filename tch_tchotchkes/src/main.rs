use std::time::Duration;

use fish_teacher::EngineEvaluation;
use models::*;
use shakmaty::{san::San, Chess, Move, Position};
use tokio::sync::{mpsc, oneshot};

mod chess_board_tensor;
mod chess_dataset;
mod mnist_demo;

mod datasets;
pub mod models;

pub fn main() {
    //eval_wide::run_training().unwrap();
    // let (white_tx, white_rx) = mpsc::channel(1);
    // std::thread::spawn(move || eval_narrow::move_predictor(20, white_rx));

    // let (black_tx, black_rx) = mpsc::channel(1);
    // std::thread::spawn(move || eval_wide::move_predictor(20, black_rx));

    // play_epochs(white_tx, black_tx);

    move_rnn::run_training().unwrap();
}

#[tokio::main]
async fn play_epochs(
    white_tx: mpsc::Sender<(
        Chess,
        oneshot::Sender<(EngineEvaluation, Move, EngineEvaluation)>,
    )>,
    black_tx: mpsc::Sender<(
        Chess,
        oneshot::Sender<(EngineEvaluation, Move, EngineEvaluation)>,
    )>,
) {
    let mut position = Chess::new();
    let mut turnidx = 1;
    while !position.is_game_over() {
        match position.turn() {
            shakmaty::Color::Black => {
                //println!("Black to move:\n{}", position.board());
                let (tx, rx) = oneshot::channel();
                black_tx.send((position.clone(), tx)).await.unwrap();
                let act = rx.await.unwrap();
                //println!("Black moved: {act}");
                println!(
                    "{} {{ Black's eval: {:?} -> {:?} }}",
                    San::from_move(&position, &act.1),
                    act.0,
                    act.2,
                );
                position = position.play(&act.1).unwrap();
            }
            shakmaty::Color::White => {
                //println!("White to move:\n{}", position.board());
                let (tx, rx) = oneshot::channel();
                white_tx.send((position.clone(), tx)).await.unwrap();
                let act = rx.await.unwrap();
                //println!("White moved: {act}");
                println!(
                    "{}. {} {{ White's eval: {:?} -> {:?} }}",
                    turnidx,
                    San::from_move(&position, &act.1),
                    act.0,
                    act.2
                );
                position = position.play(&act.1).unwrap();
                turnidx += 1;
            }
        }
    }
    println!();
    println!("Game over, to move: {}", position.turn());
}
