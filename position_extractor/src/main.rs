use std::io::{self, Read};

use radix_trie::TrieCommon;
use shakmaty::{Board, Chess, Position};

use pgn_reader::{SanPlus, Skip, Visitor};
use rayon::prelude::*;
use tokio::io::AsyncReadExt;

struct AllPositions {
    positions: Vec<Board>,
    current_pos: Chess,
}

impl AllPositions {
    fn new() -> AllPositions {
        AllPositions {
            positions: vec![],
            current_pos: Chess::new(),
        }
    }
}

impl Visitor for AllPositions {
    type Result = Vec<Board>;

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn san(&mut self, san_plus: SanPlus) {
        if let Ok(m) = san_plus.san.to_move(&self.current_pos) {
            self.current_pos.play_unchecked(&m);
            self.positions.push(self.current_pos.board().clone());
        }
    }

    fn end_game(&mut self) -> Self::Result {
        self.current_pos = Chess::new();
        ::std::mem::replace(&mut self.positions, vec![])
    }
}

struct BytesStreamReader {
    pub data_recv: tokio::sync::mpsc::Receiver<Vec<u8>>,
}

impl Read for BytesStreamReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let data = self.data_recv.blocking_recv();
            match data {
                Some(d) => {
                    for (src_byte, dst_byte) in d.iter().zip(buf.iter_mut()) {
                        *dst_byte = *src_byte;
                    }
                    return Ok(d.len());
                }
                None => {
                    println!("Decompression thread finished!");
                    return Ok(0);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let filename = "lichess_db_standard_rated_2013-03.pgn.zst";

    use futures::stream::TryStreamExt;
    use tokio_util::compat::FuturesAsyncReadCompatExt;
    let response = reqwest::get(format!(
        "https://database.lichess.org/standard/{}",
        filename
    ))
    .await
    .unwrap();
    let total_len = response.content_length().unwrap_or(1);
    let mut data = response
        // .bytes()
        // .await
        // .unwrap();
        // let data = Box::new(data);
        // let data = &Box::leak(data)[..];
        .bytes_stream()
        .map_ok(|v| v.to_vec())
        .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
        .into_async_read()
        .compat();

    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<u8>>(256);

    tokio::spawn(async move {
        let mut buf = [0u8; 16 * 1024];
        let mut downloaded_so_far = 0;
        let total_len_float = total_len as f64;
        loop {
            let len = data.read(&mut buf).await;
            match len {
                Ok(v) => {
                    downloaded_so_far += v;

                    println!(
                        "Read compressed data so far: {downloaded_so_far} \t/\t{total_len}\t{}",
                        ((downloaded_so_far as f64) / total_len_float) * 100.0
                    );
                    let data = Vec::from_iter(buf[..v].iter().map(|v| *v));
                    if let Err(_) = tx.send(data).await {
                        break;
                    }
                }
                Err(e) => {
                    println!("Error while reading compressed data: {e}");
                    drop(tx);
                    break;
                }
            }
        }
    });

    let compressed_data_blocking = BytesStreamReader { data_recv: rx };

    let mut decompressed_stream = zstd::Decoder::new(compressed_data_blocking).unwrap();

    tokio::task::spawn_blocking(move || {
        let mut board_trie: radix_trie::Trie<Vec<u8>, usize> = radix_trie::Trie::new();
        let mut reader = pgn_reader::BufferedReader::new(&mut decompressed_stream);

        let mut visitor = AllPositions::new();
        loop {
            let pos = reader.read_game(&mut visitor).unwrap();
            let pos = match pos {
                None => break,
                Some(pos) => pos,
            };

            // println!("{} positions", pos.len());

            let compact_boards: Vec<Vec<u8>> = pos
                .par_iter()
                .map(|board| compact_board::board_to_compact(&board))
                .collect();
            for board in compact_boards {
                // let reverse = compact_board::compact_slice_to_board(&compact).unwrap();
                // assert_eq!(board, reverse);

                // Increment the counter associated with this board state.
                board_trie.map_with_default(board, |v| *v += 1, 0);
            }
        }
        println!("Unique board count: {}", board_trie.len());
        println!("Counting boards with only 1 state...");
        let mut uniques = 0;
        for (_k, v) in board_trie.iter() {
            if *v == 0 {
                uniques += 1;
            }
        }
        println!("Unique boards: {uniques}");

        println!("Board trie ready, saving...");
        let out_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("../hugedata/{filename}-board-trie.postcard"))
            .unwrap();

        let out_file_buf = std::io::BufWriter::new(out_file);

        postcard::to_io(&board_trie, out_file_buf).unwrap();

        println!("Writing datas and lengths");
        let out_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("../hugedata/{filename}-board-states.postcard"))
            .unwrap();

        let out_file_buf = std::io::BufWriter::new(out_file);
        let data: Vec<_> = board_trie.iter().collect();
        postcard::to_io(&data, out_file_buf).unwrap();
    })
    .await
    .unwrap();

    Ok(())
}
