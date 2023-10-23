use shakmaty::{Color, Piece, Role, Square};
use tch::{IndexOp, Tensor};

pub fn board_to_vector(board: &shakmaty::Board, also_neg: bool) -> [f32; 2 * 6 * 64] {
    let mut data = [0.0; 2 * 6 * 64];
    let mut cursor = 0;
    for file in shakmaty::File::ALL {
        for rank in shakmaty::Rank::ALL {
            let piece = board.piece_at(shakmaty::Square::from_coords(file, rank));
            let to_add = match piece {
                Some(piece) => {
                    let mut data = [if also_neg { -1.0 } else { 0.0 }; 12];
                    let idx =
                        (piece.role as usize) - 1 + (if piece.color.is_black() { 6 } else { 0 });
                    data[idx] = 1.0;
                    data
                }
                None => [if also_neg { -1.0 } else { 0.0 }; 12],
            };
            (&mut data[cursor..cursor + 12]).copy_from_slice(&to_add);
            cursor += 12;
        }
    }
    data
}

pub fn board_to_tensor(board: &shakmaty::Board, also_neg: bool) -> Tensor {
    Tensor::from_slice(&board_to_vector(board, also_neg))
}

/// Converts a tensor to a board.
/// The tensor must have the correct shape.
pub fn tensor_to_board(t: &Tensor) -> shakmaty::Board {
    let mut b = shakmaty::Board::empty();
    let t = Vec::<f32>::try_from(t.contiguous().i((..)).view(-1)).unwrap();
    let mut cursor = 0;
    for file in shakmaty::File::ALL {
        for rank in shakmaty::Rank::ALL {
            let chunk: &[f32] = &t[cursor..cursor + 12];
            cursor += 12;

            let max = chunk.iter().enumerate().fold(
                (0, chunk[0]),
                |(idx, value), (new_idx, new_value)| {
                    if *new_value > value {
                        (new_idx, *new_value)
                    } else {
                        (idx, value)
                    }
                },
            );
            if max.1 < 0.1 {
                continue; // No piece here
            }
            let max_idx = max.0;
            let color = if max_idx < 6 {
                Color::White
            } else {
                Color::Black
            };
            let role = Role::ALL[max_idx % 6];
            b.set_piece_at(Square::from_coords(file, rank), Piece { color, role });
        }
    }

    b
}

#[cfg(test)]
mod test {
    use shakmaty::Board;

    use super::*;

    #[test]
    fn test_tensor_round_trip() {
        let b = Board::new();
        let t = board_to_tensor(&b, false);
        println!("{}", t.to_string(1).unwrap());
        let bb = tensor_to_board(&t);
        assert_eq!(b, bb);

        let b = Board::from_ascii_board_fen(b"rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R")
            .unwrap();
        let t = board_to_tensor(&b, false);
        println!("{}", t.to_string(1).unwrap());
        let bb = tensor_to_board(&t);
        assert_eq!(b, bb);

        let b = Board::new();
        let t = board_to_tensor(&b, true);
        println!("{}", t.to_string(1).unwrap());
        let bb = tensor_to_board(&t);
        assert_eq!(b, bb);

        let b = Board::from_ascii_board_fen(b"rnbqkbnr/pp1ppppp/8/2p5/4P3/5N2/PPPP1PPP/RNBQKB1R")
            .unwrap();
        let t = board_to_tensor(&b, true);
        println!("{}", t.to_string(1).unwrap());
        let bb = tensor_to_board(&t);
        assert_eq!(b, bb);
    }
}
