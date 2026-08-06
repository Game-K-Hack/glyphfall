use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{canvas, fonts};

/// Trente minutes : le cran proposé d'emblée, ni décourageant ni symbolique.
const DEFAULT_GOAL_STEP: usize = 5;

/// La première question restée sans réponse, ou le choix de l'alphabet s'il
/// n'en reste aucune.
///
/// Les écrans de question s'enchaînent par `Replace`, si bien que chacun sait
/// seulement passer au suivant sans connaître toute la file.
fn first_question(app: &App) -> Screen {
    if app.settings.daily_goal.is_none() {
        return Screen::DailyGoal { step: DEFAULT_GOAL_STEP, dragging: false };
    }
    if app.settings.random_fonts.is_none() {
        return Screen::FontChoice;
    }

    Screen::LanguageSelect { selected: 0 }
}

pub fn title_screen(app: &App, mouse: Vec2) -> Transition {
    let fonts_set = &app.fonts;
    clear_background(role::BACKGROUND);

    ui::text_centered(
        fonts_set,
        "GLYPHFALL",
        canvas::WIDTH / 2.0,
        96.0,
        fonts::TITLE,
        role::TITLE,
    );
    ui::text_centered(
        fonts_set,
        "hangeul  kana  kanji",
        canvas::WIDTH / 2.0,
        122.0,
        fonts::TEXT,
        role::TEXT_MUTED,
    );

    const BUTTON_WIDTH: f32 = 168.0;
    const BUTTON_HEIGHT: f32 = 20.0;
    let x = ((canvas::WIDTH - BUTTON_WIDTH) / 2.0).floor();

    let play = Rect::new(x, 190.0, BUTTON_WIDTH, BUTTON_HEIGHT);
    if ui::button(fonts_set, mouse, Button::new(play, "JOUER")) || is_key_pressed(KeyCode::Enter) {
        app.sfx.confirm();

        // Les questions du premier lancement ne sont posées qu'une fois
        // chacune. Y avoir répondu « non » est une réponse : on ne repose rien.
        return Transition::Push(first_question(app));
    }

    let options = Rect::new(x, 226.0, BUTTON_WIDTH, BUTTON_HEIGHT);
    if ui::button(fonts_set, mouse, Button::new(options, "OPTIONS")) {
        app.sfx.confirm();
        return Transition::Push(Screen::Options { selected: 0, dragging: None });
    }

    let quit = Rect::new(x, 262.0, BUTTON_WIDTH, BUTTON_HEIGHT);
    if ui::button(fonts_set, mouse, Button::new(quit, "QUITTER").accent(role::DANGER)) {
        return Transition::Quit;
    }

    Transition::Stay
}
