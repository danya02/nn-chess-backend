use std::io::Read;

use compact_board::compact_slice_to_board;
use tch::{data::Iter2, Tensor};

use crate::chess_board_tensor::board_to_vector;

pub fn load_batch_only_evaluation(n: u64, separate_pos_neg: bool) -> Iter2 {
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
        if separate_pos_neg {
            outputs.push(datum.1.max(0.0));
            outputs.push(-datum.1.min(0.0));
        } else {
            outputs.push(datum.1);
        }
    }

    let input_tensor = Tensor::from_slice(&inputs).view((data.len() as i64, 2 * 6 * 64));
    let output_tensor = Tensor::from_slice(&outputs).view((
        data.len() as i64,
        separate_pos_neg.then_some(2).unwrap_or(1),
    ));

    println!("Input shape: {:?}", input_tensor.size());
    println!("Output shape: {:?}", output_tensor.size());

    Iter2::new(&input_tensor, &output_tensor, 100)
}
