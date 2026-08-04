use macroquad::prelude::*;

mod app;
mod core;
mod data;
mod gfx;
mod screens;
mod window;

use crate::app::{App, Navigator, Screen, Transition};
use crate::core::GameState;
use crate::gfx::{Canvas, Fonts};
use crate::screens::game::game_screen;
use crate::screens::game_over::game_over_screen;
use crate::screens::language_select::language_select_screen;
use crate::screens::title::title_screen;
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
    let app = App { catalog, fonts };

    let mut canvas = Canvas::new();
    let mut navigator = Navigator::new(Screen::Title);
    let mut capture = Capture::from_environment();

    // Raccourci de développement : démarrer directement sur un écran donné.
    match std::env::var("ALPHATILES_START").as_deref() {
        Ok("languages") => navigator.apply(Transition::Push(Screen::LanguageSelect { selected: 0 })),
        Ok("playing") => navigator.apply(Transition::Push(Screen::Playing(GameState::new()))),
        _ => true,
    };

    loop {
        // Tout est dessiné sur la toile virtuelle, jamais directement à la
        // résolution de la fenêtre.
        canvas.begin();
        let mouse = canvas.mouse();

        let transition = match navigator.top_mut() {
            Screen::Title => title_screen(&app.fonts, mouse),
            Screen::LanguageSelect { selected } => language_select_screen(&app, selected, mouse),
            Screen::Playing(state) => game_screen(state, &app.fonts),
            Screen::GameOver { score } => game_over_screen(*score, &app.fonts, mouse),
        };

        // Échap revient en arrière partout, sauf sur l'écran-titre où il n'y a
        // rien en dessous. Les écrans n'ont donc pas à s'en préoccuper.
        let transition = match transition {
            Transition::Stay if is_key_pressed(KeyCode::Escape) && navigator.can_go_back() => {
                Transition::Pop
            }
            other => other,
        };

        canvas.end();
        capture.tick();

        if !navigator.apply(transition) {
            return;
        }
        next_frame().await;
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
