use std::{
    io::{BufRead, BufReader, Write},
    process::Stdio,
    str::FromStr,
};

use anyhow::Result;
use shakmaty::{fen::Fen, uci::Uci, Bitboard, Color, Position};

pub struct Stockfish {
    process: std::process::Child,
    stdin: std::process::ChildStdin,
    stdout: std::io::BufReader<std::process::ChildStdout>,
}

impl Stockfish {
    pub fn new() -> Self {
        let mut child = std::process::Command::new("stockfish")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        stdin.write_all(b"uci\n").unwrap();

        let mut ok = String::new();
        while ok != "uciok\n" {
            ok.clear();
            stdout.read_line(&mut ok).unwrap();
            //println!("{ok:?}");
        }

        Self {
            process: child,
            stdin,
            stdout,
        }
    }

    fn say(&mut self, what: &str) {
        //println!("> {what}");
        self.stdin.write_all(what.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
    }
    fn listen(&mut self) -> anyhow::Result<String> {
        let mut value = String::new();
        if !self.stdout.has_data_left().unwrap() {
            anyhow::bail!("Engine crashed!");
        }
        self.stdout.read_line(&mut value).unwrap();
        value.pop();
        //println!("< {value}");
        Ok(value)
    }

    pub fn ready_check(&mut self) -> Result<()> {
        self.say("isready");
        while self.listen()? != "readyok" {}
        Ok(())
    }

    pub fn evaluate_board(
        &mut self,
        board: &shakmaty::Board,
        to_move: shakmaty::Color,
    ) -> anyhow::Result<Option<(EngineEvaluation, Uci)>> {
        self.ready_check();
        self.say("ucinewgame");
        self.say("setoption name UCI_AnalyseMode value true");
        self.say(&format!(
            "position fen {} {}",
            board.board_fen(Bitboard::EMPTY),
            to_move.fold_wb("w", "b")
        ));
        self.say(&format!("go depth {}", 10));
        let mut last_score = String::new();
        let mut line = String::new();

        while !line.starts_with("bestmove") {
            line = self.listen()?;
            if line.starts_with("info") {
                last_score = line
                    .split("score")
                    .last()
                    .unwrap()
                    .split("nodes")
                    .nth(0)
                    .unwrap()
                    .to_string();
            }
        }

        let eval = EngineEvaluation::from_str(&last_score, to_move);

        let parts: Vec<_> = line.split_whitespace().collect();
        let my_move: Vec<_> = parts[1]
            .as_bytes()
            .iter()
            .map(|v| char::from_u32(*v as u32).unwrap())
            .collect();
        if parts[1] == "(none)" {
            return Ok(None);
        }

        Ok(Some((eval, Uci::from_str(&parts[1].trim()).unwrap())))
    }

    pub fn evaluate_pos(
        &mut self,
        board: &shakmaty::Chess,
    ) -> anyhow::Result<Option<(EngineEvaluation, Uci)>> {
        self.ready_check();
        self.say("ucinewgame");
        self.say("setoption name UCI_AnalyseMode value true");
        self.say(&format!(
            "position fen {}",
            Fen::from_position(board.clone(), shakmaty::EnPassantMode::Legal).to_string()
        ));
        self.say(&format!("go depth {}", 10));
        let mut last_score = String::new();
        let mut line = String::new();

        while !line.starts_with("bestmove") {
            line = self.listen()?;
            if line.starts_with("info") {
                last_score = line
                    .split("score")
                    .last()
                    .unwrap()
                    .split("nodes")
                    .nth(0)
                    .unwrap()
                    .to_string();
            }
        }

        let eval = EngineEvaluation::from_str(&last_score, board.turn());

        let parts: Vec<_> = line.split_whitespace().collect();
        let my_move: Vec<_> = parts[1]
            .as_bytes()
            .iter()
            .map(|v| char::from_u32(*v as u32).unwrap())
            .collect();
        if parts[1] == "(none)" {
            return Ok(None);
        }

        Ok(Some((eval, Uci::from_str(&parts[1].trim()).unwrap())))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EngineEvaluation {
    /// This amount of advantage to white
    Centipawns(i64),
    /// Mate in this number of moves
    Mate(i64),
}

fn translate(value: f32, left_min: f32, left_max: f32, right_min: f32, right_max: f32) -> f32 {
    let left_span = left_max - left_min;
    let right_span = right_max - right_min;
    let value_scaled = (value - left_min) / left_span;
    right_min + (value_scaled * right_span)
}

impl EngineEvaluation {
    pub fn to_numeric_score(&self) -> f32 {
        match self {
            EngineEvaluation::Centipawns(v) => {
                // The range between -3000 and 3000 centipawns is mapped into -0.8 to 0.8
                let v = (*v).clamp(-3000, 3000) as f32;
                translate(v, -3000.0, 3000.0, -0.8, 0.8)
            }
            EngineEvaluation::Mate(t) => {
                // The range between mate-in 25 and 1 is mapped to 0.8 to 1.
                let inv_fac = translate((*t).clamp(1, 25) as f32, 1.0, 25.0, 0.0, 0.2);
                (if t.is_negative() { -1.0 } else { 1.0 }) - inv_fac
            }
        }
    }

    pub fn from_numeric_score(v: f32) -> Self {
        // If the value is between -0.8 and 0.8, then it's a centipawns between -3000 and 3000
        if v.abs() < 0.8 {
            Self::Centipawns(translate(v, -0.8, 0.8, -3000.0, 3000.0) as i64)
        } else {
            let fac = v.abs() - 0.8;
            let inv_fac = 0.2 - fac;
            Self::Mate(translate(inv_fac, 0.0, 0.2, 1.0, 25.0).ceil() as i64)
        }
    }
}

impl EngineEvaluation {
    pub fn from_str(text: &str, to_move: Color) -> Self {
        let parts: Vec<_> = text.trim().split(" ").collect();
        let number: i64 = parts[1].parse().unwrap();
        match parts[0] {
            "cp" => Self::Centipawns(number * to_move.fold_wb(1, -1)),
            "mate" => Self::Mate(number * to_move.fold_wb(1, -1)),
            other => panic!("Unknown scoring: {other}"),
        }
    }
}
