use macroquad::prelude::*;

mod app;
mod audio;
/// Outil de développement : il génère la musique et n'a rien à faire dans la
/// version navigateur, où il ne serait que du code mort.
#[cfg(not(target_arch = "wasm32"))]
mod compose;
mod data;
mod gfx;
mod music;
mod progress;
mod screens;
mod session;
mod settings;
mod storage;
mod window;

use crate::app::{App, Navigator, Screen, Transition};
use crate::audio::Sfx;
use crate::gfx::{Canvas, Fonts};
use crate::music::Music;
use crate::progress::Progress;
use crate::settings::Settings;
use crate::session::Session;
use crate::screens::briefing::briefing_screen;
use crate::screens::game::game_screen;
use crate::screens::results::results_screen;
use crate::screens::language_select::language_select_screen;
use crate::screens::learning_path::learning_path_screen;
use crate::screens::options::options_screen;
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
    // Les réglages viennent avant le son : les volumes sont appliqués dès la
    // création, sans passer par un état par défaut audible une fraction de
    // seconde.
    let settings = Settings::load();
    let sfx = Sfx::load(settings.sfx_gain()).await;
    let music = Music::load(settings.music_gain());

    let mut app =
        App { catalog, fonts, sfx, music, progress: Progress::load(), settings };

    // Génère la musique d'ambiance puis quitte, sans ouvrir de fenêtre de jeu.
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(path) = std::env::var("ALPHATILES_COMPOSE") {
        compose::write(&path);
        return;
    }

    let mut canvas = Canvas::new();
    let mut navigator = Navigator::new(Screen::Title);
    let mut capture = Capture::from_environment();

    // Raccourci de développement : démarrer directement sur un écran donné,
    // par exemple `ALPHATILES_START=path:ja-hiragana`.
    if let Ok(start) = std::env::var("ALPHATILES_START") {
        let screen = match start.split_once(':') {
            Some(("path", language)) => {
                Some(Screen::LearningPath { language: language.to_string(), selected: None })
            }
            Some(("briefing", target)) => target.split_once('/').map(|(language, level)| {
                Screen::Briefing { language: language.to_string(), level: level.to_string() }
            }),
            Some(("play", target)) => target.split_once('/').and_then(|(language, level)| {
                Session::new(&app.catalog, language, level)
                    .map(|session| Screen::Playing(Box::new(session)))
            }),
            _ => match start.as_str() {
                "languages" => Some(Screen::LanguageSelect { selected: 0 }),
                "options" => Some(Screen::Options { selected: 0 }),
                _ => None,
            },
        };

        if let Some(screen) = screen {
            navigator.apply(Transition::Push(screen));
        }
    }

    loop {
        // Tout est dessiné sur la toile virtuelle, jamais directement à la
        // résolution de la fenêtre.
        canvas.begin();
        let mouse = canvas.mouse();

        // La manche reste silencieuse : les bruitages y portent l'information,
        // une musique par-dessus les couvrirait.
        let in_menus = !matches!(navigator.top_mut(), Screen::Playing(_));
        app.music.update(get_frame_time(), in_menus).await;

        let mut transition = match navigator.top_mut() {
            Screen::Title => title_screen(&app, mouse),
            Screen::LanguageSelect { selected } => language_select_screen(&app, selected, mouse),
            Screen::Options { selected } => options_screen(&mut app, selected, mouse),
            Screen::LearningPath { language, selected } => {
                learning_path_screen(&app, language, selected, mouse)
            }
            Screen::Briefing { language, level } => {
                briefing_screen(&app, language, level, mouse)
            }
            Screen::Playing(session) => game_screen(&app, session),
            Screen::Results { outcome, elapsed } => {
                results_screen(&app, outcome, elapsed, mouse)
            }
        };

        // La progression s'enregistre au moment de la transition, et non a
        // chaque frame de l'ecran de resultats qui reste affiche longtemps.
        if let Transition::Replace(Screen::Results { outcome, .. }) = &mut transition {
            outcome.is_record = app.progress.record(&outcome.level_id, outcome.stars);
            if outcome.is_record {
                app.progress.save();
            }
        }

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
