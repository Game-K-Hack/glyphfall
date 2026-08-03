use macroquad::prelude::*;

pub const TILE_WIDTH: f32 = 100.0;
pub const TILE_HEIGHT: f32 = 140.0;
pub const COLS: i32 = 4; // 4 colonnes comme dans Piano Tiles
pub const TARGET_Y: f32 = 500.0; // La ligne de validation en bas de l'écran

const KOREAN_FONT: &str = "NotoSansKR-Regular.ttf";

/// Ressources chargées une seule fois au démarrage puis partagées par tous les écrans.
/// Les polices sont embarquées dans le binaire pour rester multiplateforme (WASM inclus).
pub struct Assets {
    pub korean: Font,
}

impl Assets {
    pub fn load() -> Self {
        // Les polices viennent du même répertoire embarqué que les langues :
        // un manifeste peut ainsi désigner la sienne par son nom de fichier.
        let bytes = crate::data::font_bytes(KOREAN_FONT)
            .unwrap_or_else(|| panic!("{KOREAN_FONT} est absent de assets/fonts/"));
        let korean =
            load_ttf_font_from_bytes(bytes).expect("Fichier de police invalide ou corrompu");

        Self { korean }
    }
}

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
    pub game_over: bool,
    pub current_screen: Screen,
    pub input_buffer: String,
}

impl GameState {
    pub fn new() -> Self {
        Self {
            tiles: Vec::new(),
            score: 0,
            lives: 3,
            speed: 200.0,
            spawn_timer: 0.0,
            game_over: false,
            current_screen: Screen::MainMenu,
            input_buffer: String::new(),
        }
    }

    pub fn spawn_tile(&mut self) {
        let col = rand::gen_range(0, COLS);

        // Notre dictionnaire d'apprentissage (Consonnes et Voyelles de base)
        let alphabet_coreen = [
            ("ㄱ", "g"),  ("ㄴ", "n"),  ("ㄷ", "d"),  ("ㄹ", "r"),
            ("ㅁ", "m"),  ("ㅂ", "b"),  ("ㅅ", "s"),  ("ㅇ", "ng"),
            ("ㅈ", "j"),  ("ㅊ", "ch"), ("ㅋ", "k"),  ("ㅌ", "t"),
            ("ㅍ", "p"),  ("ㅎ", "h"),
            ("ㅏ", "a"),  ("ㅑ", "ya"), ("ㅓ", "eo"), ("ㅕ", "yeo"),
            ("ㅗ", "o"),  ("ㅛ", "yo"), ("ㅜ", "u"),  ("ㅠ", "yu"),
            ("ㅡ", "eu"), ("ㅣ", "i"),
        ];

        // Choisit un index au hasard dans le dictionnaire
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

#[derive(PartialEq)]
pub enum Screen {
    MainMenu,
    Playing,
}
