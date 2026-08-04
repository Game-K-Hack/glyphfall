//! L'état d'une partie en cours.
//!
//! Les valeurs codées en dur ici sont provisoires : elles seront remplacées par
//! les règles du niveau chargé depuis `assets/languages/`.

use macroquad::prelude::*;

/// Toutes les dimensions sont en pixels virtuels (voir `gfx::canvas`).
pub const TILE_WIDTH: f32 = 48.0;
pub const TILE_HEIGHT: f32 = 40.0;
pub const COLS: i32 = 4;
/// La ligne de validation, au-dessus de la barre de saisie.
pub const TARGET_Y: f32 = 160.0;

pub struct Tile {
    pub col: i32,
    pub y: f32,
    pub glyph: String,
    pub romanization: String,
    pub is_pressed: bool,
}

pub struct GameState {
    pub tiles: Vec<Tile>,
    pub score: u32,
    pub lives: u32,
    pub speed: f32,
    pub spawn_timer: f32,
    pub input_buffer: String,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            tiles: Vec::new(),
            score: 0,
            lives: 3,
            speed: 55.0,
            spawn_timer: 0.0,
            input_buffer: String::new(),
        }
    }

    pub fn spawn_tile(&mut self) {
        let col = rand::gen_range(0, COLS);

        // Provisoire : le tirage viendra des glyphes du niveau choisi.
        let alphabet_coreen = [
            ("ㄱ", "g"),  ("ㄴ", "n"),  ("ㄷ", "d"),  ("ㄹ", "r"),
            ("ㅁ", "m"),  ("ㅂ", "b"),  ("ㅅ", "s"),  ("ㅇ", "ng"),
            ("ㅈ", "j"),  ("ㅊ", "ch"), ("ㅋ", "k"),  ("ㅌ", "t"),
            ("ㅍ", "p"),  ("ㅎ", "h"),
            ("ㅏ", "a"),  ("ㅑ", "ya"), ("ㅓ", "eo"), ("ㅕ", "yeo"),
            ("ㅗ", "o"),  ("ㅛ", "yo"), ("ㅜ", "u"),  ("ㅠ", "yu"),
            ("ㅡ", "eu"), ("ㅣ", "i"),
        ];

        let idx = rand::gen_range(0, alphabet_coreen.len());
        let (glyph, romanization) = alphabet_coreen[idx];

        self.tiles.push(Tile {
            col,
            y: -TILE_HEIGHT,
            glyph: glyph.to_string(),
            romanization: romanization.to_string(),
            is_pressed: false,
        });
    }
}
