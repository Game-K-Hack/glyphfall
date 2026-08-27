// Glyphfall — apprendre le hangeul, les kana et les kanji en jouant.
// Copyright (C) 2026 Game K
//
// Ce programme est un logiciel libre : vous pouvez le redistribuer et le
// modifier selon les termes de la GNU General Public License, version 3,
// publiée par la Free Software Foundation.
//
// Il est distribué dans l'espoir d'être utile, mais SANS AUCUNE GARANTIE,
// pas même la garantie implicite de VALEUR MARCHANDE ou d'ADÉQUATION À UN
// USAGE PARTICULIER. Voir la GNU General Public License pour les détails.
//
// Vous devriez avoir reçu une copie de la licence avec ce programme, dans
// le fichier LICENSE. Sinon, voir <https://www.gnu.org/licenses/>.

//! Glyphfall : le programme lui-même, indépendant de la façon dont il démarre.
//!
//! Le bureau et le navigateur passent par `main.rs`, Android par `quad_main` :
//! là-bas aucun processus ne démarre sur `main`, c'est une activité Java qui
//! charge cette bibliothèque et appelle un symbole. Les deux chemins se
//! rejoignent aussitôt sur `start`.

use macroquad::prelude::*;

mod app;
mod audio;
/// Outil de développement : il génère la musique et n'a rien à faire dans la
/// version navigateur, où il ne serait que du code mort.
#[cfg(not(target_arch = "wasm32"))]
mod compose;
mod daily;
mod data;
mod gfx;
mod music;
mod voices;
mod progress;
mod screens;
mod session;
mod settings;
mod storage;
mod window;

use crate::app::{App, Navigator, Screen, Transition};
use crate::daily::Daily;
use crate::audio::Sfx;
use crate::gfx::{Canvas, Fonts, ui};
use crate::music::{Ambience, Music};
use crate::progress::Progress;
use crate::settings::Settings;
use crate::session::{Mode, Session};
use crate::screens::briefing::briefing_screen;
use crate::screens::daily_goal::daily_goal_screen;
use crate::screens::font_choice::font_choice_screen;
use crate::screens::goal_reached::goal_reached_screen;
use crate::screens::game::game_screen;
use crate::screens::results::results_screen;
use crate::screens::sign::sign_screen;
use crate::screens::language_select::language_select_screen;
use crate::screens::learning_path::{PathView, learning_path_screen};
use crate::screens::options::options_screen;
use crate::screens::pronunciation::pronunciation_screen;
use crate::screens::title::title_screen;
use crate::voices::Voices;
use crate::window::window_conf;

/// Ouvre la fenêtre et lance le jeu.
///
/// C'est ce que faisait `#[macroquad::main]`, écrit à la main : la macro
/// engendre un `main`, dont une bibliothèque n'a que faire.
pub fn start() {
    macroquad::Window::from_config(window_conf(), run());
}

/// Point d'entrée Android, appelé par l'activité Java une fois la bibliothèque
/// native chargée.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn quad_main() {
    start();
}

async fn run() {
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
    let music = Music::load(settings.music_gain(), settings.music_game_gain());
    // Les voix suivent le volume des bruitages : ce sont des sons courts qu'on
    // déclenche, pas un fond sonore.
    let voices =
        Voices::load(catalog.languages.iter().map(|langue| langue.id.clone()), settings.sfx_gain());

    // L'ancienne sauvegarde rangeait tous les signes dans une même table, sans
    // distinguer les écritures. Les répartir demande le catalogue, d'où cette
    // reprise après chargement plutôt qu'à l'intérieur de `Progress::load`.
    let mut progress = Progress::load();
    if progress.migrate(&catalog) {
        progress.save();
    }

    let mut app =
        App { catalog, fonts, sfx, music, voices, progress, settings, daily: Daily::load() };

    // Génère la musique d'ambiance puis quitte, sans ouvrir de fenêtre de jeu.
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(path) = std::env::var("GLYPHFALL_COMPOSE") {
        compose::write(&path);
        return;
    }

    let mut canvas = Canvas::new();
    let mut navigator = Navigator::new(Screen::Title);
    let mut capture = Capture::from_environment();

    // Raccourci de développement : démarrer directement sur un écran donné,
    // par exemple `GLYPHFALL_START=path:ja-hiragana`.
    if let Ok(start) = std::env::var("GLYPHFALL_START") {
        let screen = match start.split_once(':') {
            Some(("path", language)) => {
                Some(Screen::LearningPath {
                    language: language.to_string(),
                    view: PathView::new(),
                })
            }
            // `briefing:ko/ko-04/ultra` ouvre le briefing sur un mode donné.
            Some(("briefing", target)) => {
                let mut parts = target.split('/');
                match (parts.next(), parts.next()) {
                    (Some(language), Some(level)) => Some(Screen::Briefing {
                        language: language.to_string(),
                        level: level.to_string(),
                        mode: match parts.next() {
                            Some("fast") => Mode::Fast,
                            Some("ultra") => Mode::Ultra,
                            Some("endless") => Mode::Endless,
                            _ => Mode::Normal,
                        },
                    }),
                    _ => None,
                }
            }
            Some(("dit", target)) => {
                target.split_once('/').map(|(language, level)| Screen::Pronunciation {
                    language: language.to_string(),
                    level: level.to_string(),
                    index: 0,
                    swipe: None,
                })
            }
            Some(("sign", target)) => target.split_once('/').map(|(language, level)| Screen::Sign {
                language: language.to_string(),
                level: level.to_string(),
                index: 0,
                swipe: None,
            }),
            Some(("play", target)) => target.split_once('/').and_then(|(language, level)| {
                let tracings = app.tracings(language);
                Session::new(&app.catalog, &app.progress, language, level, Mode::Normal, tracings)
                    .map(|session| Screen::Playing(Box::new(session)))
            }),
            _ => match start.as_str() {
                "languages" => Some(Screen::LanguageSelect { selected: 0 }),
                "options" => Some(Screen::Options { selected: 0, dragging: None }),
                "goal" => Some(Screen::DailyGoal { step: 5, dragging: false }),
                "alert" => Some(Screen::GoalReached),
                "fonts" => Some(Screen::FontChoice),
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

        // La manche a sa propre ambiance, plus energique et jouee plus bas pour
        // laisser passer les bruitages.
        let playing = matches!(navigator.top_mut(), Screen::Playing(_));
        let ambience = if playing { Ambience::Game } else { Ambience::Menus };
        app.music.update(get_frame_time(), ambience).await;
        // Une voix demandée pendant le rendu se joue ici : les écrans ne
        // peuvent pas attendre le chargement d'un son, la boucle si.
        app.voices.update().await;

        // Seul le temps passé à apprendre compte : une manche en cours, ou la
        // fiche d'un signe que l'on étudie. Le briefing en est exclu — on peut
        // le laisser ouvert sans rien apprendre, comme n'importe quel menu.
        let learning =
            matches!(navigator.top_mut(), Screen::Playing(_) | Screen::Sign { .. });
        if learning {
            app.daily.add(get_frame_time());
        }

        let mut transition = match navigator.top_mut() {
            Screen::Title => title_screen(&app, mouse),
            Screen::LanguageSelect { selected } => language_select_screen(&app, selected, mouse),
            Screen::Options { selected, dragging } => {
                options_screen(&mut app, selected, dragging, mouse)
            }
            Screen::DailyGoal { step, dragging } => {
                daily_goal_screen(&mut app, step, dragging, mouse)
            }
            Screen::GoalReached => goal_reached_screen(&app, mouse),
            Screen::FontChoice => font_choice_screen(&mut app, mouse),
            Screen::LearningPath { language, view } => {
                learning_path_screen(&app, language, view, mouse)
            }
            Screen::Briefing { language, level, mode } => {
                briefing_screen(&app, language, level, mode, mouse)
            }
            Screen::Sign { language, level, index, swipe } => {
                sign_screen(&app, language, level, index, swipe, mouse)
            }
            Screen::Pronunciation { language, level, index, swipe } => {
                pronunciation_screen(&app, language, level, index, swipe, mouse)
            }
            Screen::Playing(session) => game_screen(&app, session, mouse),
            Screen::Results { outcome, elapsed } => {
                results_screen(&app, outcome, elapsed, mouse)
            }
        };

        // La progression s'enregistre au moment de la transition, et non a
        // chaque frame de l'ecran de resultats qui reste affiche longtemps.
        if let Transition::Replace(Screen::Results { outcome, .. }) = &mut transition {
            // Une révision libre ne correspond à aucune étape : elle ne peut donc
            // ni décrocher d'étoiles ni en faire perdre.
            if !outcome.is_revision {
                outcome.is_record = match outcome.mode {
                    Mode::Normal => app.progress.record(&outcome.level_id, outcome.stars),
                    mode => app.progress.record_mode(
                        &outcome.level_id,
                        mode,
                        outcome.is_perfect,
                        outcome.score,
                    ),
                };
            }

            // La maîtrise de chaque signe est mise à jour même sans record :
            // c'est elle qui pilotera le tirage des prochaines révisions.
            for sign in &outcome.signs {
                app.progress.note(&outcome.language_id, &sign.character, sign.hits, sign.misses);
            }
            app.progress.save();
        }

        // L'alerte attend d'être hors partie, et de ne rien avoir d'autre à
        // faire : l'empiler pendant une manche ferait perdre des vies, et
        // par-dessus une transition en cours la ferait disparaître aussitôt.
        if !playing
            && matches!(transition, Transition::Stay)
            && app.daily.goal_reached(app.settings.daily_goal_minutes())
        {
            app.daily.mark_alerted();
            app.sfx.confirm();
            transition = Transition::Push(Screen::GoalReached);
        }

        // Échap revient en arrière partout, sauf sur l'écran-titre où il n'y a
        // rien en dessous. Les écrans n'ont donc pas à s'en préoccuper.
        //
        // `Back` est la touche retour d'Android, que miniquad remonte comme
        // n'importe quelle autre : un téléphone n'a pas d'Échap, et sans elle
        // il faudrait un bouton dessiné sur chacun des douze écrans.
        let going_back = is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Back);
        let transition = match transition {
            Transition::Stay if going_back && navigator.can_go_back() => Transition::Pop,
            other => other,
        };

        // Le curseur vient de passer sur un autre élément : même bruit qu'une
        // flèche, parce que c'est le même geste. Le test se fait après le rendu,
        // seul moment où l'on sait ce que l'écran a mis sous le curseur.
        if ui::focus_moved() {
            app.sfx.navigate();
        }
        // Changer d'écran remet le compteur à zéro : sinon le bouton qui se
        // trouve sous le curseur à l'arrivée claquerait pour rien.
        if !matches!(transition, Transition::Stay) {
            ui::forget_focus();
        }

        canvas.end();
        capture.tick();

        if !navigator.apply(transition) {
            // Sortir de la boucle suffit sur un bureau : la fenêtre se ferme et
            // le processus s'arrête. Sur Android l'activité, elle, reste en vie
            // et n'affiche plus qu'un écran noir — il faut la terminer.
            #[cfg(target_os = "android")]
            std::process::exit(0);

            #[cfg(not(target_os = "android"))]
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
/// `GLYPHFALL_SCREENSHOT=chemin.png` enregistre une image après quelques
/// frames — le temps que les atlas de police soient remplis — puis quitte.
struct Capture {
    path: Option<String>,
    frames: u32,
    delay: u32,
}

impl Capture {
    /// Frames à laisser passer par défaut : la première frame dessine parfois
    /// avant que les glyphes ne soient rastérisés. `GLYPHFALL_SCREENSHOT_AFTER`
    /// permet d'attendre plus longtemps, le temps qu'une scène s'anime.
    const DELAY: u32 = 20;

    fn from_environment() -> Self {
        let delay = std::env::var("GLYPHFALL_SCREENSHOT_AFTER")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(Self::DELAY);

        Self { path: std::env::var("GLYPHFALL_SCREENSHOT").ok(), frames: 0, delay }
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
