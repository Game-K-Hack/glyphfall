use macroquad::prelude::*;

use crate::app::{Screen, Transition};
use crate::core::GameState;
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};

/// Écran de fin provisoire : il deviendra l'écran de résultats en étoiles.
pub fn game_over_screen(score: u32, fonts_set: &Fonts, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);

    ui::text_centered(fonts_set, "GAME OVER", canvas::WIDTH / 2.0, 56.0, fonts::TITLE, role::DANGER);
    ui::text_centered(
        fonts_set,
        &format!("SCORE {score:05}"),
        canvas::WIDTH / 2.0,
        84.0,
        fonts::TEXT,
        role::TEXT,
    );

    const BUTTON_WIDTH: f32 = 168.0;
    let x = ((canvas::WIDTH - BUTTON_WIDTH) / 2.0).floor();

    let retry = Rect::new(x, 120.0, BUTTON_WIDTH, 20.0);
    if ui::button(fonts_set, mouse, Button::new(retry, "REJOUER")) {
        return Transition::Replace(Screen::Playing(GameState::new()));
    }

    let menu = Rect::new(x, 148.0, BUTTON_WIDTH, 20.0);
    if ui::button(fonts_set, mouse, Button::new(menu, "MENU").accent(role::TEXT_MUTED)) {
        return Transition::Pop;
    }

    Transition::Stay
}
