use std::convert::Infallible;

use anyhow::Result;
use fish_teacher::EngineEvaluation;
use rand::Rng;
use shakmaty::{Board, ByColor, Chess, Color, FromSetup, Move, Position, Setup};
use tch::{nn, nn::Module, nn::OptimizerConfig, Device};

use crate::{
    chess_board_tensor::board_to_tensor, datasets::load_batch_only_evaluation, models::net,
};

const BOARD_SIZE: i64 = 64;
const BOARD_SQUARE_NUM_OPTS: i64 = 2 * 6;
const INPUT_SHAPE: i64 = BOARD_SIZE * BOARD_SQUARE_NUM_OPTS;
const HIDDEN_SHAPE: &[i64] = &[1536, 4096, 8192, 2048, 512, 256, 128]; // superwide
const OUTPUT_SHAPE: i64 = 2;

pub fn run_training() -> Result<()> {
    let mut train = load_batch_only_evaluation(0, true);
    let mut test = load_batch_only_evaluation(1, true);
    let mut vs = nn::VarStore::new(Device::Cpu);

    let mut epoch = *get_checkpoint_idxs().iter().max().unwrap_or(&0);
    if epoch > 0 {
        vs.load(format!(
            "../hugedata/eval-checkpoints/superwide/{epoch}.checkpoint"
        ))?;
    }

    let net = net(&vs.root(), INPUT_SHAPE, HIDDEN_SHAPE, OUTPUT_SHAPE);
    let mut opt = nn::Adam::default().build(&vs, 0.001)?;
    println!("Starting optimizing...");
    while epoch < 450 {
        epoch += 1;
        train = load_batch_only_evaluation(epoch * 2, true);
        test = load_batch_only_evaluation(epoch * 2 + 1, true);
        println!("Training...");
        for (input, output) in train.shuffle().to_device(vs.device()) {
            //let input = input.view(-1);
            //let output = output.view(-1);

            //dbg!(input.size());
            //dbg!(output.size());

            //println!("Forward step:");
            let prediction = net.forward(&input);

            //dbg!(prediction.size());

            //println!("Loss:");
            let loss = prediction.mse_loss(&output, tch::Reduction::Sum);
            //dbg!(loss.size());
            //println!("Backward step:");
            opt.backward_step(&loss);

            println!(
                "epoch: {:4} train loss: {:8.5}",
                epoch,
                f64::try_from(&loss)?,
            );
        }
        for (input, output) in test.shuffle().to_device(vs.device()).take(10) {
            // Testing
            let prediction = net.forward(&input);

            //dbg!(input.size());
            //dbg!(output.size());
            //dbg!(prediction.size());
            let test_accuracy = prediction.mse_loss(&output, tch::Reduction::Sum);
            println!(
                "epoch: {:4} test loss: {:8.5}",
                epoch,
                f64::try_from(&test_accuracy)?,
            );
        }

        println!("Saving checkpoint {epoch}");
        vs.save(format!(
            "../hugedata/eval-checkpoints/superwide/{epoch}.checkpoint"
        ))?;
    }

    Ok(())
}

pub fn get_checkpoint_idxs() -> Vec<u64> {
    let mut idxs = vec![];
    for path in std::fs::read_dir("../hugedata/eval-checkpoints/superwide/").unwrap() {
        let name = path.unwrap().file_name();
        let name = name.to_string_lossy();
        if let Some(id) = name.strip_suffix(".checkpoint") {
            idxs.push(id.parse().unwrap())
        }
    }
    idxs
}

pub fn move_predictor(
    checkpoint: u64,
    mut jobs: tokio::sync::mpsc::Receiver<(
        Chess,
        tokio::sync::oneshot::Sender<(EngineEvaluation, Move, EngineEvaluation)>,
    )>,
) -> anyhow::Result<Infallible> {
    println!("Loading checkpoint superwide/{checkpoint}");
    let mut vs = nn::VarStore::new(Device::Cpu);

    vs.load(format!(
        "../hugedata/eval-checkpoints/superwide/{checkpoint}.checkpoint"
    ))?;

    let net = net(&vs.root(), INPUT_SHAPE, HIDDEN_SHAPE, OUTPUT_SHAPE);

    loop {
        let job = jobs.blocking_recv().unwrap();
        let mut position = job.0;

        // If the position has black to move, then recolor and rotate the board before presenting it to the NN.
        let black_to_move = position.turn() == Color::Black;
        if black_to_move {
            let setup = position.into_setup(shakmaty::EnPassantMode::Legal);
            let mut board = setup.board;
            board.flip_vertical();
            let (role, color) = board.into_bitboards();
            let board = Board::from_bitboards(
                role,
                ByColor {
                    black: color.white,
                    white: color.black,
                },
            );
            let new_setup = Setup {
                board,
                promoted: setup.promoted.flip_horizontal(),
                pockets: setup.pockets,
                turn: Color::White,
                castling_rights: setup.castling_rights.flip_vertical(),
                ep_square: setup.ep_square.map(|v| v.flip_vertical()),
                remaining_checks: setup.remaining_checks,
                halfmoves: setup.halfmoves,
                fullmoves: setup.fullmoves,
            };
            position = Chess::from_setup(new_setup, shakmaty::CastlingMode::Standard)?;
        }
        let current_eval_tensor = net.forward(&board_to_tensor(position.board(), false));
        let current_eval = Vec::<f32>::try_from(current_eval_tensor)?;
        let current_eval = EngineEvaluation::from_numeric_score(current_eval[0] - current_eval[1]);

        let moves = position.legal_moves();
        let mut rng = rand::thread_rng();
        let mut preferred_move = moves.first().unwrap().clone();
        let mut preferred_move_eval = f32::NEG_INFINITY;
        for potential_move in position.legal_moves() {
            let new_position = position.clone().play(&potential_move)?;
            let eval_tensor = net.forward(&board_to_tensor(new_position.board(), false));
            let eval = Vec::<f32>::try_from(eval_tensor)?;
            let eval = eval[0] - eval[1];
            if eval > preferred_move_eval {
                preferred_move = potential_move.clone();
                preferred_move_eval = eval + rng.gen_range(-0.025..0.025);
            }
        }

        // If the board was turned around, then the move needs to be turned around before submitting it.
        if black_to_move {
            preferred_move = match preferred_move {
                Move::Normal {
                    role,
                    from,
                    capture,
                    to,
                    promotion,
                } => Move::Normal {
                    role,
                    from: from.flip_vertical(),
                    capture,
                    to: to.flip_vertical(),
                    promotion,
                },
                Move::EnPassant { from, to } => Move::EnPassant {
                    from: from.flip_vertical(),
                    to: to.flip_vertical(),
                },
                Move::Castle { king, rook } => Move::Castle {
                    king: king.flip_vertical(),
                    rook: rook.flip_vertical(),
                },
                Move::Put { role, to } => Move::Put {
                    role,
                    to: to.flip_vertical(),
                },
            };
        }
        job.1
            .send((
                current_eval,
                preferred_move,
                EngineEvaluation::from_numeric_score(preferred_move_eval),
            ))
            .unwrap();
    }
}
