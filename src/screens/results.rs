//! Le bilan d'une manche : combien d'étoiles, et quoi faire ensuite.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};
use crate::progress::MAX_STARS;
use crate::session::{EndReason, Outcome, Session};

/// Les étoiles du bilan sont dessinées plus grandes que celles des listes.
const STAR_SCALE: f32 = 3.0;

pub fn results_screen(app: &App, outcome: &Outcome, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);

    draw_verdict(&app.fonts, outcome);
    draw_stars(outcome.stars);
    draw_figures(&app.fonts, outcome);

    const BUTTON_WIDTH: f32 = 110.0;
    let gap = 12.0;
    let total = BUTTON_WIDTH * 2.0 + gap;
    let x = ((canvas::WIDTH - total) / 2.0).floor();

    let retry = Rect::new(x, 176.0, BUTTON_WIDTH, 20.0);
    if ui::button(&app.fonts, mouse, Button::new(retry, "REJOUER").focused(true)) {
        return match Session::new(&app.catalog, &outcome.language_id, &outcome.level_id) {
            Some(session) => Transition::Replace(Screen::Playing(Box::new(session))),
            // Le niveau a disparu du catalogue : on ne peut que remonter.
            None => Transition::Pop,
        };
    }

    let path = Rect::new(x + BUTTON_WIDTH + gap, 176.0, BUTTON_WIDTH, 20.0);
    if ui::button(&app.fonts, mouse, Button::new(path, "CHEMIN").accent(role::TEXT_MUTED)) {
        // On repasse par le briefing, puis le chemin : deux crans en arrière.
        return Transition::Pop;
    }

    Transition::Stay
}

fn draw_verdict(fonts_set: &Fonts, outcome: &Outcome) {
    let (verdict, color) = match (outcome.stars, outcome.reason) {
        (0, EndReason::OutOfLives) => ("PLUS DE VIES", role::DANGER),
        (0, EndReason::TimeUp) => ("TEMPS ECOULE", role::DANGER),
        (MAX_STARS.., _) => ("PARFAIT", role::SUCCESS),
        _ => ("TERMINE", role::TITLE),
    };

    ui::text_centered(fonts_set, verdict, canvas::WIDTH / 2.0, 16.0, fonts::TITLE, color);
    ui::text_truncated(
        fonts_set,
        &outcome.level_title,
        8.0,
        40.0,
        fonts::TEXT,
        role::TEXT_MUTED,
        canvas::WIDTH - 16.0,
    );
}

/// Les trois étoiles, dessinées pixel par pixel puis agrandies : les redessiner
/// à l'échelle garde les bords nets, contrairement à une image étirée.
fn draw_stars(earned: u8) {
    const Y: f32 = 62.0;
    let star_size = ui::STAR_WIDTH * STAR_SCALE;
    let gap = 10.0;
    let total = star_size * MAX_STARS as f32 + gap * (MAX_STARS - 1) as f32;
    let start_x = ((canvas::WIDTH - total) / 2.0).floor();

    for index in 0..MAX_STARS {
        let x = start_x + index as f32 * (star_size + gap);
        ui::star_scaled(x, Y, STAR_SCALE, index < earned);
    }
}

fn draw_figures(fonts_set: &Fonts, outcome: &Outcome) {
    const Y: f32 = 116.0;

    let accuracy = format!("{}% DE REUSSITE", (outcome.accuracy * 100.0).round() as u32);
    ui::text_centered(fonts_set, &accuracy, canvas::WIDTH / 2.0, Y, fonts::TEXT, role::TEXT);

    let detail = format!("{} SIGNES   {:05} POINTS", outcome.hits, outcome.score);
    ui::text_centered(fonts_set, &detail, canvas::WIDTH / 2.0, Y + 12.0, fonts::TEXT, role::TEXT_MUTED);
}
