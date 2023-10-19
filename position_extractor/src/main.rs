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

    for year in 2013..=2014 {
        for month in 1..12 {
            download_data(year, month).await?;
        }
    }
    Ok(())
}

const PERFORM_ISOLATED_TRIMMING: bool = true;

async fn download_data(year: i32, month: i32) -> io::Result<()> {
    let output_file = format!("single-{year}-{month}-board-trie.postcard");

    // Check for tries that contain this month already
    for file in std::fs::read_dir("../hugedata").unwrap() {
        let file_name = file.unwrap().file_name();
        let name = file_name.to_string_lossy();
        println!("{name}");
        if name == output_file {
            println!("Not downloading for {year}-{month} because this file is already present");
            return Ok(());
        }
        if let Some(prefix) = name.strip_suffix("-board-tries.postcard") {
            if let Some(main) = prefix.strip_prefix("combined-") {
                let parts: Vec<_> = main.split("+").collect();
                let left_parts: Vec<_> = parts[0].split("-").collect();
                let right_parts: Vec<_> = parts[1].split("-").collect();
                let left_year: i32 = (left_parts[0]).parse().unwrap();
                let left_month: i32 = (left_parts[1]).parse().unwrap();
                let right_year = (right_parts[0]).parse().unwrap();
                let right_month = (right_parts[1]).parse().unwrap();
                let mut current_month = left_month;
                let mut current_year = left_year;
                while !(current_month == right_month && current_year == right_year) {
                    if current_month == month && current_year == year {
                        println!("Not downloading for {year}-{month} because there is a file that covers range {left_year}-{left_month} to {right_year}-{right_month}");
                        return Ok(());
                    }
                    current_month += 1;
                    if current_month > 12 {
                        current_month = 1;
                        current_year += 1;
                    }
                }

                if current_month == month && current_year == year {
                    println!("Not downloading for {year}-{month} because there is a file that covers range {left_year}-{left_month} to {right_year}-{right_month}");
                    return Ok(());
                }
            }
        }
    }

    use futures::stream::TryStreamExt;
    use tokio_util::compat::FuturesAsyncReadCompatExt;
    let response = reqwest::get(format!(
        "https://database.lichess.org/standard/lichess_db_standard_rated_{year}-{month:0>2}.pgn.zst"
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
        let len = board_trie.len();
        println!("Board count: {}", len);
        println!("Counting boards with only 1 state...");
        let mut uniques = 0;
        for (_k, v) in board_trie.iter() {
            if *v == 0 {
                uniques += 1;
            }
        }
        println!("Unique boards: {uniques}");

        if PERFORM_ISOLATED_TRIMMING {
            println!("PERFORMING ISOLATED TRIMMING");
            let mut more_keys: bool = true;
            while more_keys {
                more_keys = false;
                let mut keys_to_delete = vec![];
                for (k, v) in board_trie.iter() {
                    if *v == 0 {
                        keys_to_delete.push(k.clone());
                    }
                    if keys_to_delete.len() > 128 * 1024 {
                        more_keys = true;
                        break;
                    }
                }
                for key in keys_to_delete.drain(0..) {
                    board_trie.remove(&key);
                }
                println!(
                    "Still remaining uniques: {}",
                    uniques - (len - board_trie.len())
                );
            }
        }

        println!("Board trie ready, saving...");
        let out_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(format!("../hugedata/{output_file}"))
            .unwrap();

        let out_file_buf = std::io::BufWriter::new(out_file);

        postcard::to_io(&board_trie, out_file_buf).unwrap();
    })
    .await
    .unwrap();

    Ok(())
}
