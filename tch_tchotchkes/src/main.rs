use std::time::Duration;

use shakmaty::{san::San, Chess, Position};
use tokio::sync::{mpsc, oneshot};

use chess_evaluation_nn::{move_predictor, run_training};
use mnist_demo::run;

mod chess_board_tensor;
mod chess_dataset;
mod chess_evaluation_nn;
mod mnist_demo;

pub fn main() {
    //run();
    run_training().unwrap();
    //play_epochs("10-wide", "70-wide");
}

#[tokio::main]
async fn play_epochs(white: &str, black: &str) {
    let white = format!("{white}");
    let black = format!("{black}");
    println!("White playing as checkpoint {white}");
    println!("Black playing as checkpoint {black}");

    let (white_tx, white_rx) = mpsc::channel(10);
    let (black_tx, black_rx) = mpsc::channel(10);
    tokio::task::spawn_blocking(move || move_predictor(white, white_rx).unwrap());
    tokio::task::spawn_blocking(move || move_predictor(black, black_rx).unwrap());
    tokio::time::sleep(Duration::from_secs(1)).await;

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
                print!("{}. {} ", turnidx, San::from_move(&position, &act));
                if turnidx % 10 == 0 {
                    println!()
                }
                position = position.play(&act).unwrap();
            }
            shakmaty::Color::White => {
                //println!("White to move:\n{}", position.board());
                let (tx, rx) = oneshot::channel();
                white_tx.send((position.clone(), tx)).await.unwrap();
                let act = rx.await.unwrap();
                //println!("White moved: {act}");
                print!("{}. {} ", turnidx, San::from_move(&position, &act));
                if turnidx % 10 == 0 {
                    println!()
                }
                position = position.play(&act).unwrap();
            }
        }
        turnidx += 1;
    }

    println!("Game over, to move: {}", position.turn());
}
