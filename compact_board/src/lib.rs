use bitvec::prelude::Msb0;
use bitvec::vec::BitVec;
use shakmaty::Color::*;
use shakmaty::File;
use shakmaty::Rank;
use shakmaty::Role::*;
use shakmaty::Square;
use shakmaty::{Board, Piece};

pub fn board_to_compact(board: &Board) -> Vec<u8> {
    let mut output: BitVec<u8, Msb0> = BitVec::with_capacity(248);
    let mut push = |value, count| {
        push_tail_bits(&mut output, value, count);
    };
    // The first 4 bits show how many white pieces there are on the board,
    // the next 4 bits are how many black pieces there are.
    // let white_piece_count = board.by_color(White).count() as u8;
    // push(white_piece_count, 4);
    // let black_piece_count = board.by_color(Black).count() as u8;
    // push(black_piece_count, 4);

    // the next 3 bits are the file of the white king (0 to 7).
    // The next 3 bits are the rank of the white king,
    let white_king = board.king_of(White).expect("Should be only one black king");
    push(white_king.file() as u8, 3);
    push(white_king.rank() as u8, 3);

    // then the file and rank of the black king
    let black_king = board
        .king_of(Black)
        .expect("Should be exactly one black king");
    push(black_king.file() as u8, 3);
    push(black_king.rank() as u8, 3);

    // After this, the other pieces.
    // The pieces are listed in the following order:
    // pawns, then knights, then bishops, then rooks, then queens.
    // Each piece starts with a 1 bit,
    // then 3 bits for file, then 3 bits for rank.
    // Once all the pieces in the category are over, a 0 bit is written instead of a 1 bit.
    for color in [White, Black] {
        for role in [Pawn, Knight, Bishop, Rook, Queen] {
            for pos in board.by_piece(Piece { color, role }) {
                push(1, 1);
                push(pos.file() as u8, 3);
                push(pos.rank() as u8, 3);
            }
            push(0, 1);
        }
    }

    output.set_uninitialized(true);
    output.into_vec()
}

pub fn compact_to_board(r: &mut bitreader::BitReader) -> Result<Board, bitreader::BitReaderError> {
    let mut b = Board::empty();

    // The first 4 bits is the number of white pieces, then number of black pieces. All zeros means 16.
    // let mut white_pieces_left = r.read_u8(4)?;
    // if white_pieces_left == 0 {
    //     white_pieces_left = 16;
    // }
    // let mut black_pieces_left = r.read_u8(4)?;
    // if black_pieces_left == 0 {
    //     black_pieces_left = 16;
    // }
    // Then, the white king's rank and file, 3 bits
    let wkf = r.read_u8(3)?;
    let wkr = r.read_u8(3)?;
    let wk = Square::from_coords(File::new(wkf as u32), Rank::new(wkr as u32));
    b.set_piece_at(
        wk,
        Piece {
            color: White,
            role: King,
        },
    );
    // Then, the black king's rank and file, 3 bits
    let bkf = r.read_u8(3)?;
    let bkr = r.read_u8(3)?;
    let bk = Square::from_coords(File::new(bkf as u32), Rank::new(bkr as u32));
    b.set_piece_at(
        bk,
        Piece {
            color: Black,
            role: King,
        },
    );

    for role in [Pawn, Knight, Bishop, Rook, Queen] {
        // Check if there are any more pieces of this kind.
        while r.read_bool()? {
            // There is one more piece of this kind: read its rank and file.
            let pf = r.read_u8(3)?;
            let pr = r.read_u8(3)?;
            let p = Square::from_coords(File::new(pf as u32), Rank::new(pr as u32));
            b.set_piece_at(p, Piece { color: White, role });
        }
    }

    // Now there are no white pieces left, so read the black pieces
    for role in [Pawn, Knight, Bishop, Rook, Queen] {
        // Check if there are any more pieces of this kind.
        while r.read_bool()? {
            // There is one more piece of this kind: read its rank and file.
            let pf = r.read_u8(3)?;
            let pr = r.read_u8(3)?;
            let p = Square::from_coords(File::new(pf as u32), Rank::new(pr as u32));
            b.set_piece_at(p, Piece { color: Black, role });
        }
    }

    Ok(b)
}

pub fn compact_slice_to_board(r: &[u8]) -> Result<Board, bitreader::BitReaderError> {
    compact_to_board(&mut bitreader::BitReader::new(r))
}

fn push_tail_bits(to_where: &mut BitVec<u8, Msb0>, value: u8, tail_bit_count: u8) {
    for idx in (0..tail_bit_count).rev() {
        let is_bit_set: bool = unsafe { std::mem::transmute((value >> idx) & 1) };
        to_where.push(is_bit_set);
    }
}

#[cfg(test)]
mod test {
    use bitvec::bitvec;

    use super::*;
    #[test]
    fn test_push_tail_bits() {
        let mut v = BitVec::new();
        push_tail_bits(&mut v, 0, 1);
        assert_eq!(v, bitvec![u8, Msb0; 0]);
        let mut v = BitVec::new();
        push_tail_bits(&mut v, 1, 1);
        assert_eq!(v, bitvec![u8, Msb0; 1]);

        let mut v: BitVec<u8, Msb0> = BitVec::new();
        push_tail_bits(&mut v, 0b00001111, 7);
        assert_eq!(v, bitvec![u8, Msb0; 0,0,0,1,1,1,1]);

        let mut v: BitVec<u8, Msb0> = BitVec::new();
        push_tail_bits(&mut v, 0b10101010, 4);
        assert_eq!(v, bitvec![u8, Msb0; 1,0,1,0]);
        let mut v: BitVec<u8, Msb0> = BitVec::new();
        push_tail_bits(&mut v, 0b01010101, 4);
        assert_eq!(v, bitvec![u8, Msb0; 0,1,0,1]);
    }

    #[test]
    fn test_board_round_trip() {
        let b = Board::new();
        let compact_repr = board_to_compact(&b);
        println!("{compact_repr:?}");
        let mut compact_repr = bitreader::BitReader::new(&compact_repr);
        let expanded_board = compact_to_board(&mut compact_repr).unwrap();
        assert_eq!(b, expanded_board);
    }
}
