#![feature(buf_read_has_data_left)]
mod fish;

use std::time::Duration;

use fish::Stockfish;
use rand::{seq::SliceRandom, SeedableRng};
use shakmaty::{uci::Uci, Bitboard, Board, ByColor, Chess, Color, FromSetup, Position, Setup};

use compact_board::{board_to_compact, compact_slice_to_board};
use radix_trie::TrieCommon;
use tokio::sync::mpsc;

async fn fish_worker(
    mut board_rx: mpsc::Receiver<Vec<u8>>,
    eval_tx: mpsc::Sender<(Vec<u8>, f32, String)>,
) {
    tokio::task::spawn_blocking(move || {
        let mut fish = Stockfish::new();
        loop {
            let compact_board = board_rx.blocking_recv().unwrap();
            let mut board = compact_slice_to_board(&compact_board).unwrap();

            //println!("fen: {}", board.board_fen(Bitboard::EMPTY));

            // If this board has a game over, then do not give it to the engine.
            let mut setup = Setup::empty();
            setup.board = board.clone();
            setup.turn = Color::White;
            let checkmate = Chess::from_setup(setup, shakmaty::CastlingMode::Standard)
                .and_then(|v| Ok(v.is_game_over()))
                .unwrap_or(true);
            if !checkmate {
                // If it is not checkmate with white to move, then evaluate the board from white's perspective.
                let res = fish.evaluate_board(&board, Color::White);
                if let Err(_) = res {
                    println!(
                        "Engine crashed on board: {}",
                        board.board_fen(Bitboard::EMPTY)
                    );
                    fish = Stockfish::new();
                    continue;
                };
                let res = res.unwrap();
                if let Some(eval_a) = res {
                    // Send this.
                    eval_tx
                        .blocking_send((
                            compact_board,
                            eval_a.0.to_numeric_score(),
                            eval_a.1.to_string(),
                        ))
                        .unwrap();
                }
            }

            let mut setup = Setup::empty();
            setup.board = board.clone();
            setup.turn = Color::Black;
            let checkmate = Chess::from_setup(setup, shakmaty::CastlingMode::Standard)
                .and_then(|v| Ok(v.is_game_over()))
                .unwrap_or(true);
            if !checkmate {
                // If it is not checkmate from Black's perspective:
                // First, transform the board so that it's still White's perspective.
                let mut board = board.clone();
                board.rotate_180();
                let (by_role, by_color) = board.into_bitboards();
                let board = Board::from_bitboards(
                    by_role,
                    ByColor {
                        black: by_color.white,
                        white: by_color.black,
                    },
                );
                // Now evaluate it from the new White's, old Black's, perspective
                let res = fish.evaluate_board(&board, Color::White);
                if let Err(_) = res {
                    println!(
                        "Engine crashed on board: {}",
                        board.board_fen(Bitboard::EMPTY)
                    );
                    fish = Stockfish::new();
                    continue;
                };
                let res = res.unwrap();
                if let Some(eval_b) = res {
                    // Send this.
                    eval_tx
                        .blocking_send((
                            board_to_compact(&board),
                            eval_b.0.to_numeric_score(),
                            eval_b.1.to_string(),
                        ))
                        .unwrap();
                }
            }
        }
    })
    .await
    .unwrap();
}

async fn board_loader(senders: Vec<mpsc::Sender<Vec<u8>>>) {
    let mut names = vec![];
    for file in std::fs::read_dir("../hugedata").unwrap() {
        let file_name = file.unwrap().file_name();
        let name = file_name.to_string_lossy().to_string();
        if name.contains("board-trie") {
            names.push(name);
        }
    }

    for name in names {
        println!("Loading file {name}...");
        let board_trie = tokio::task::spawn_blocking(move || {
            let file = std::fs::OpenOptions::new()
                .read(true)
                .open(format!("../hugedata/{name}"))
                .unwrap();
            let reader = std::io::BufReader::new(file);
            let mut buf = [0; 32 * 1024];
            let trie: radix_trie::Trie<Vec<u8>, usize> =
                postcard::from_io((reader, &mut buf)).unwrap().0;
            println!("Loading file completed!");
            trie
        })
        .await
        .unwrap();

        println!("Iterating over trie and loading boards...");
        let count = board_trie.len();
        let mut sender_cycle = senders.iter().cycle();
        for (idx, (board, seen_count)) in board_trie.iter().enumerate() {
            if idx % 1000 == 0 {
                println!("{idx}\t/\t{count} boards loaded...");
            }
            //println!("seen: {seen_count}");
            sender_cycle
                .next()
                .unwrap()
                .send(board.clone())
                .await
                .unwrap();
        }
    }
}

async fn board_saver(mut recv: mpsc::Receiver<(Vec<u8>, f32, String)>) {
    let batch_size = 8192;
    let mut batch_idx: usize = 0;
    let mut rng = rand::rngs::StdRng::from_seed(rand::random());
    let mut values = Vec::with_capacity(batch_size);
    loop {
        let v = recv.recv().await.unwrap();
        values.push(v);
        if values.len() >= batch_size {
            println!("Shuffling batch {batch_idx}");
            values.shuffle(&mut rng);
            println!("Saving batch {batch_idx}");
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(format!("../hugedata/batches/batch_{batch_idx}.postcard"))
                .unwrap();
            let buf = std::io::BufWriter::new(file);
            postcard::to_io(&values, buf).unwrap();
            values.clear();
            batch_idx += 1;
        }
    }
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");
    let mut board_senders = vec![];
    let (eval_tx, eval_rx) = mpsc::channel(1024);
    for _ in 0..64 {
        let (tx, rx) = mpsc::channel(256);
        board_senders.push(tx);
        tokio::spawn(fish_worker(rx, eval_tx.clone()));
    }

    tokio::spawn(board_saver(eval_rx));
    board_loader(board_senders).await;
}
