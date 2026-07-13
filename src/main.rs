mod reporter;
mod viri;

use kf_internals::{
    chess::{All, Color, Move, MoveList, Position},
    engine::Engine,
    evaluate::is_material_draw,
};
use rand::{random_range, seq::IndexedRandom};
use rand::seq::SliceRandom;
use reporter::DatagenReporter;
use std::{fs::OpenOptions, io::stdout};
use std::{
    io::Write,
    sync::mpsc::{Receiver, Sender, channel},
};
use viri::{Result, ViriGameData};

const GEN_DEPTH: u8 = 8;

fn is_terminal(pos: &mut Position) -> bool {
    if pos.rule_50_count() >= 100 || pos.is_threefold() || is_material_draw(pos) {
        return true;
    }
    let mut moves = MoveList::new();
    pos.get_moves::<All>(&mut moves);
    moves.is_empty()
}

fn determine_result(pos: &mut Position) -> Result {
    let mut moves = MoveList::new();
    pos.get_moves::<All>(&mut moves);
    if !moves.is_empty() || !pos.in_check() {
        Result::Draw
    } else {
        match pos.color() {
            Color::White => Result::Black,
            Color::Black => Result::White,
        }
    }
}

fn play_game<F: Fn() -> Position>(
    white: &mut Engine<DatagenReporter>,
    black: &mut Engine<DatagenReporter>,
    rx: &Receiver<(i32, Move)>,
    get_pos: F,
) -> ViriGameData {
    let mut pos = get_pos();
    let mut game_data = ViriGameData::from_pos(&pos);

    let players = [white, black];
    let color_muls = [1, -1];

    while !is_terminal(&mut pos) {
        let stm = pos.color() as usize;
        players[stm].set_position(pos.clone());
        players[stm].start_search(Some(GEN_DEPTH), None);
        let (e, mv) = rx.recv().unwrap();
        game_data.add_move(mv, e * color_muls[stm]);
        pos.do_move(mv);
    }

    game_data.set_result(determine_result(&mut pos));
    game_data
}

//Randomly generate a starting position
fn random_opening() -> Position {
    let mut pos = Position::new();
    let moves_count = random_range(7..12);

    let mut moves = MoveList::new();

    for _ in 0..moves_count {
        pos.get_moves::<All>(&mut moves);
        if moves.is_empty() {
            pos.undo_move();
            pos.undo_move();
            continue;
        }
        let idx = random_range(0..moves.len());
        pos.do_move(moves[idx]);
        moves.clear();
    }

    if is_terminal(&mut pos) {
        random_opening()
    } else {
        pos
    }
}

fn run_games(n_rand: usize, n_pmdfrc: usize, out: Sender<Vec<u8>>) {
    let (tx, rx) = channel();
    let rep = DatagenReporter::new(tx);
    let mut e1 = Engine::new(rep.clone());
    let mut e2 = Engine::new(rep);

    let mut total_rand = 0;
    while total_rand < n_rand {
        let data = play_game(&mut e1, &mut e2, &rx, random_opening);
        total_rand += data.len();
        let _ = out.send(data.serialize());
        e1.reset_all();
        e2.reset_all();
    }

    let mut total_pmdfrc = 0;
    while total_pmdfrc < n_pmdfrc {
        let data = play_game(&mut e1, &mut e2, &rx, poor_mans_dfrc);
        total_pmdfrc += data.len();
        let _ = out.send(data.serialize());
        e1.reset_all();
        e2.reset_all();
    }
}

fn generate_training_data(threads: usize, total: usize) {
    // Do a 90 - 10 split normal games vs "dfrc"
    let normals = total * 99 / 100;
    let pmdfrcs = total / 100;

    let t_normal = normals / threads;
    let t_pmdfrc = pmdfrcs / threads;

    let (tx, rx) = channel();

    for _ in 0..threads {
        let n = t_normal;
        let d = t_pmdfrc;
        let out = tx.clone();
        std::thread::spawn(move || run_games(n, d, out));
    }

    drop(tx);

    let mut out_file = OpenOptions::new()
        .read(true)
        .create(true)
        .append(true)
        .open("training.data")
        .unwrap();

    let total_len = (total as f64).log10().floor() as usize + 1;

    let mut current = 0;
    for v in rx {
        current += (v.len() - 36) / 4;
        let _ = out_file.write_all(&v);
        print!("\r {current:>total_len$}/{total}");
        stdout().flush().unwrap();
    }
    println!();
}

// Just randomize the backranks, no castling. Should be fine? Mostly to have some data at weird
// positions.
fn poor_mans_dfrc() -> Position {
    let mut start_fen = String::from("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1");
    unsafe {
        start_fen.as_bytes_mut()[..8].shuffle(&mut rand::rng());
        start_fen.as_bytes_mut()[35..43].shuffle(&mut rand::rng());
    }
    let mut pos = Position::from_fen(start_fen).unwrap();
    let mut moves = MoveList::new();
    pos.get_moves::<All>(&mut moves);
    let m = *moves.choose(&mut rand::rng()).unwrap();
    pos.do_move(m);

    pos
}

fn main() {
    generate_training_data(4, 2_000_000);
}
