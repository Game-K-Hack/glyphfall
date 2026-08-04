use macroquad::prelude::*;

mod core;
mod data;
mod gfx;
mod screens;
mod window;

use crate::core::{GameState, Screen};
use crate::gfx::{Canvas, Fonts};
use crate::screens::game::game_screen;
use crate::screens::main_menu::main_menu_screen;
use crate::window::window_conf;

#[macroquad::main(window_conf)]
async fn main() {
    // Initialise le générateur de nombres aléatoires
    rand::srand(miniquad::date::now() as u64);

    // Le contenu est validé au démarrage : mieux vaut un écran d'erreur lisible
    // qu'un chemin d'apprentissage silencieusement cassé.
    let catalog = match data::load_catalog() {
        Ok(catalog) => catalog,
        Err(error) => return fatal_error_screen(&error.to_string()).await,
    };

    let fonts = Fonts::load(&catalog);
    let mut canvas = Canvas::new();
    let mut state = GameState::new();
    let mut capture = Capture::from_environment();

    // Raccourci de développement : démarrer directement sur un écran donné.
    if std::env::var("ALPHATILES_START").as_deref() == Ok("playing") {
        state.current_screen = Screen::Playing;
    }

    loop {
        // Tout est dessiné sur la toile virtuelle, jamais directement à la
        // résolution de la fenêtre.
        canvas.begin();
        let mouse = canvas.mouse();

        match state.current_screen {
            Screen::MainMenu => main_menu_screen(&mut state, &fonts, mouse),
            Screen::Playing => game_screen(&mut state, &fonts, mouse),
        }

        canvas.end();
        // Avant le `next_frame` : c'est là que le tampon contient encore ce qui
        // vient d'être dessiné.
        capture.tick();
        next_frame().await;
    }
}

/// Capture d'écran de contrôle, pour vérifier le rendu sans oeil humain.
///
/// `ALPHATILES_SCREENSHOT=chemin.png` enregistre une image après quelques
/// frames — le temps que les atlas de police soient remplis — puis quitte.
struct Capture {
    path: Option<String>,
    frames: u32,
    delay: u32,
}

impl Capture {
    /// Frames à laisser passer par défaut : la première frame dessine parfois
    /// avant que les glyphes ne soient rastérisés. `ALPHATILES_SCREENSHOT_AFTER`
    /// permet d'attendre plus longtemps, le temps qu'une scène s'anime.
    const DELAY: u32 = 20;

    fn from_environment() -> Self {
        let delay = std::env::var("ALPHATILES_SCREENSHOT_AFTER")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(Self::DELAY);

        Self { path: std::env::var("ALPHATILES_SCREENSHOT").ok(), frames: 0, delay }
    }

    fn tick(&mut self) {
        let Some(path) = &self.path else { return };

        self.frames += 1;
        if self.frames >= self.delay {
            get_screen_data().export_png(path);
            std::process::exit(0);
        }
    }
}

/// Affiche une erreur de contenu jusqu'à ce que la fenêtre soit fermée.
/// Un `panic!` ne serait vu par personne : le jeu se lance sans terminal.
async fn fatal_error_screen(message: &str) {
    loop {
        clear_background(BLACK);
        draw_text("CONTENU INVALIDE", 40.0, 80.0, 34.0, RED);
        draw_text(message, 40.0, 130.0, 18.0, WHITE);
        draw_text("Corrigez le fichier TOML puis relancez.", 40.0, 170.0, 18.0, GRAY);
        next_frame().await
    }
}
