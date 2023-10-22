use std::io::Read;

use compact_board::compact_slice_to_board;
use shakmaty::{uci::Uci, Role, Square};
use tch::{data::Iter2, Tensor};

use crate::chess_board_tensor::board_to_vector;

pub fn load_batch(n: usize) -> Iter2 {
    println!("Loading file batch_{n}...");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .open(format!("../hugedata/batches/batch_{n}.postcard"))
        .unwrap();
    let mut reader = std::io::BufReader::new(file);
    let mut data = vec![];
    reader.read_to_end(&mut data).unwrap();

    //    let data: Vec<(Vec<u8>, f32, String)> = postcard::from_io((reader, &mut buf)).unwrap().0;
    let data: Vec<(Vec<u8>, f32, String)> = postcard::from_bytes(&data).unwrap();

    // Now convert it into an input and output tensor.
    let mut inputs = vec![];
    let mut outputs = vec![];

    for datum in data.iter() {
        let board = compact_slice_to_board(&datum.0).unwrap();
        let board_vector = board_to_vector(&board);
        inputs.extend_from_slice(&board_vector);
        let uci = Uci::from_ascii(datum.2.as_bytes()).unwrap();
        let mut move_dest = [0i64; 3];
        //*move_dest.last_mut().unwrap() = datum.1;
        if let Uci::Normal {
            from,
            to,
            promotion,
        } = uci
        {
            move_dest[0] = from as i64;
            move_dest[1] = from as i64;
            if let Some(promo) = promotion {
                move_dest[2] = promo as i64 + 1;
            }
            // move_dest[from as usize] = 1.0;
            // move_dest[to as usize + 64] = 1.0;
            // if let Some(promo) = promotion {
            //     move_dest[promo as usize + 128] = 1.0;
            // }
        } else {
            panic!("Unexpected kind of UCI move: {uci:?}");
        }
        outputs.extend_from_slice(&move_dest);
    }

    let input_tensor = Tensor::from_slice(&inputs).view((data.len() as i64, 2 * 6 * 64));
    let output_tensor = Tensor::from_slice(&outputs).view((data.len() as i64, 128 + 6 + 1));

    println!("Input shape: {:?}", input_tensor.size());
    println!("Output shape: {:?}", output_tensor.size());

    Iter2::new(&input_tensor, &output_tensor, 100)
}



pub fn tensor_to_move(t: &Tensor) -> Uci {
    let t = Vec::<f32>::try_from(t.view(-1)).unwrap();
    let first_val: f32 = t[0];
    let first_idx =
        t.iter()
            .enumerate()
            .take(64)
            .fold((0, first_val), |(idx, value), (new_idx, new_value)| {
                if new_value > &value {
                    (new_idx, *new_value)
                } else {
                    (idx, value)
                }
            });
    let from = Square::ALL[first_idx.0];

    let first_val: f32 = t[64];
    let first_idx = t.iter().skip(64).enumerate().take(64).fold(
        (0, first_val),
        |(idx, value), (new_idx, new_value)| {
            if new_value > &value {
                (new_idx, *new_value)
            } else {
                (idx, value)
            }
        },
    );
    let to = Square::ALL[first_idx.0];

    let first_val: f32 = t[64];

    let first_idx = t.iter().skip(128).enumerate().take(6).fold(
        (0, first_val),
        |(idx, value), (new_idx, new_value)| {
            if new_value > &value {
                (new_idx, *new_value)
            } else {
                (idx, value)
            }
        },
    );
    let promotion = if first_idx.1 < 0.1 {
        None
    } else {
        Some(Role::ALL[first_idx.0])
    };

    Uci::Normal {
        from,
        to,
        promotion,
    }
}
