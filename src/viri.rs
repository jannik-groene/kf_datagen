use kf_internals::chess::{Color, Move, MoveType, Piece, Position, Square};

#[derive(Debug)]
pub enum Result {
    Black = 0,
    Draw = 1,
    White = 2,
}

fn piece_to_viri(p: Piece, c: Color, castle: bool) -> u8 {
    let val = match p {
        Piece::Pawn => 0,
        Piece::Knight => 1,
        Piece::Bishop => 2,
        Piece::Rook => {
            if castle {
                6
            } else {
                3
            }
        }
        Piece::Queen => 4,
        Piece::King => 5,
    };
    val ^ ((c as u8) << 3)
}

#[derive(Default)]
pub struct ViriGameData {
    occupation: u64,
    pieces: [u8; 16],
    ep_stm: u8,
    hmc: u8,
    fmc: u16,
    score: i16,
    res: u8,
    filler: u8,
    moves: Vec<(u16, i16)>,
}

impl ViriGameData {
    pub fn from_pos(pos: &Position) -> Self {
        let occupation = u64::from(pos.get_board().occupation());
        let mut pieces = [0; 16];
        for (i, sq) in pos.get_board().occupation().into_iter().enumerate() {
            let idx = i / 2;
            let offset = (i % 2) * 4;
            let piece = pos.get_board().piece_at(sq).unwrap();
            let color = if pos.get_board().get_color_bb(Color::White).is_set(sq) {
                Color::White
            } else {
                Color::Black
            };
            let castling_rights = pos.castling_rights();
            let castle = piece == Piece::Rook
                && ((sq == Square::A1 && castling_rights[0][1])
                    || (sq == Square::H1 && castling_rights[0][0])
                    || (sq == Square::A8 && castling_rights[1][1])
                    || (sq == Square::H8 && castling_rights[1][0]));
            pieces[idx] ^= piece_to_viri(piece, color, castle) << offset;
        }
        let ep_stm = pos.ep_square().map_or(64, u8::from) ^ (pos.color() as u8) << 7;
        let hmc = pos.rule_50_count();
        Self {
            occupation,
            pieces,
            ep_stm,
            hmc,
            fmc: 0,
            score: 0,
            res: 0,
            filler: 0,
            moves: Vec::new(),
        }
    }
    pub fn add_move(&mut self, m: Move, e: i32) {
        self.moves.push((move_to_viri(m), e as i16));
    }
    pub fn set_result(&mut self, res: Result) {
        self.res = res as u8;
    }
    pub fn len(&self) -> usize {
        self.moves.len()
    }
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + 4 * self.moves.len() + 4);
        out.extend_from_slice(&self.occupation.to_le_bytes());
        out.extend_from_slice(&self.pieces);
        out.push(self.ep_stm);
        out.push(self.hmc);
        out.extend_from_slice(&self.fmc.to_le_bytes());
        out.extend_from_slice(&self.score.to_le_bytes());
        out.push(self.res);
        out.push(self.filler);
        for m in self
            .moves
            .iter()
            .map(|(m, e)| [m.to_le_bytes(), e.to_le_bytes()].concat())
        {
            out.extend_from_slice(&m);
        }
        out.extend_from_slice(&[0, 0, 0, 0]);
        out
    }
}

fn translate_castling_square(sq: Square) -> Square {
    match sq {
        Square::C1 => Square::A1,
        Square::G1 => Square::H1,
        Square::C8 => Square::A8,
        Square::G8 => Square::H8,
        _ => panic!(),
    }
}

fn move_to_viri(mut m: Move) -> u16 {
    //TODO: adjust castling to be king takes rook
    if matches!(m.typ(), MoveType::Castle) {
        m = Move::new(
            m.from(),
            translate_castling_square(m.to()),
            MoveType::Castle,
        )
    }
    let upper = match m.typ() {
        MoveType::Normal | MoveType::Capture => 0b0000,
        MoveType::Enpassant => 0b0100,
        MoveType::Castle => 0b1000,
        MoveType::PromotionN | MoveType::PromotionCaptureN => 0b1100,
        MoveType::PromotionB | MoveType::PromotionCaptureB => 0b1101,
        MoveType::PromotionR | MoveType::PromotionCaptureR => 0b1110,
        MoveType::PromotionQ | MoveType::PromotionCaptureQ => 0b1111,
    };
    (m.compress() & 0xfff) | upper << 12
}

#[test]
fn viri_example() {
    use kf_internals::chess::{MoveType, Square};
    let pos = Position::new();
    let mut data = ViriGameData::from_pos(&pos);
    data.add_move(Move::new(Square::E2, Square::E4, MoveType::Normal), 10);
    data.add_move(Move::new(Square::E7, Square::E5, MoveType::Normal), 20);
    data.add_move(Move::new(Square::D1, Square::H5, MoveType::Normal), -30);
    data.add_move(Move::new(Square::E8, Square::E7, MoveType::Normal), 32767);
    data.add_move(Move::new(Square::H5, Square::E5, MoveType::Capture), 32767);
    data.set_result(Result::White);
    let expected = vec![
        0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, 0x16, 0x42, 0x25, 0x61, 0x00, 0x00, 0x00,
        0x00, 0x88, 0x88, 0x88, 0x88, 0x9e, 0xca, 0xad, 0xe9, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x02, 0x00, 0x0c, 0x07, 0x0a, 0x00, 0x34, 0x09, 0x14, 0x00, 0xc3, 0x09, 0xe2, 0xff, 0x3c,
        0x0d, 0xff, 0x7f, 0x27, 0x09, 0xff, 0x7f, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(data.serialize(), expected);
}
