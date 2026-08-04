use macroquad::prelude::*;

use crate::app::{Screen, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};

pub fn title_screen(fonts_set: &Fonts, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);

    ui::text_centered(
        fonts_set,
        "ALPHA TILES",
        canvas::WIDTH / 2.0,
        48.0,
        fonts::TITLE,
        role::TITLE,
    );
    ui::text_centered(
        fonts_set,
        "apprends un alphabet",
        canvas::WIDTH / 2.0,
        72.0,
        fonts::TEXT,
        role::TEXT_MUTED,
    );

    const BUTTON_WIDTH: f32 = 168.0;
    const BUTTON_HEIGHT: f32 = 20.0;
    let x = ((canvas::WIDTH - BUTTON_WIDTH) / 2.0).floor();

    let play = Rect::new(x, 116.0, BUTTON_WIDTH, BUTTON_HEIGHT);
    if ui::button(fonts_set, mouse, Button::new(play, "JOUER")) || is_key_pressed(KeyCode::Enter) {
        return Transition::Push(Screen::LanguageSelect { selected: 0 });
    }

    let quit = Rect::new(x, 144.0, BUTTON_WIDTH, BUTTON_HEIGHT);
    if ui::button(fonts_set, mouse, Button::new(quit, "QUITTER").accent(role::DANGER)) {
        return Transition::Quit;
    }

    Transition::Stay
}
