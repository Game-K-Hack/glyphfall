//! L'alerte de fin d'objectif.
//!
//! Elle n'apparaît jamais pendant une manche : interrompre une partie en cours
//! ferait perdre des vies et la ferait passer pour une punition, alors que
//! c'est une récompense. La boucle principale attend donc d'être revenue dans
//! les menus.

use macroquad::prelude::*;

use crate::app::{App, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{canvas, fonts};
use crate::settings::goal_label;

/// Le panneau du message, centré sur la toile.
const PANEL: Rect = Rect {
    x: canvas::pick(16.0, 52.0),
    y: canvas::pick(120.0, 56.0),
    w: canvas::pick(184.0, 280.0),
    h: canvas::pick(128.0, 104.0),
};

pub fn goal_reached_screen(app: &App, mouse: Vec2) -> Transition {
    // La pile ne dessine que son sommet : laisser le fond intact ne montrerait
    // pas l'écran précédent mais un lavis de frames superposées.
    clear_background(role::BACKGROUND);
    ui::panel(PANEL, role::PANEL);

    let center = PANEL.x + PANEL.w / 2.0;

    ui::stars_row(
        (center - ui::stars_row_width(3) / 2.0).floor(),
        PANEL.y + 12.0,
        3,
        3,
    );
    ui::text_centered(&app.fonts, "OBJECTIF ATTEINT", center, PANEL.y + 26.0, fonts::TEXT, role::SUCCESS);

    // Le temps réellement passé, et non l'objectif : on a pu le dépasser en
    // finissant une manche, et l'annoncer au rabais serait mesquin.
    ui::text_centered(
        &app.fonts,
        &format!("{} AUJOURD'HUI", goal_label(app.daily.minutes())),
        center,
        PANEL.y + 42.0,
        fonts::TEXT,
        role::TEXT,
    );
    ui::paragraph(
        &app.fonts,
        "Rien ne t'empeche de continuer.",
        center,
        PANEL.y + 58.0,
        fonts::TEXT,
        role::TEXT_MUTED,
        PANEL.w - 12.0,
    );

    let close = Rect::new((center - 60.0).floor(), PANEL.y + PANEL.h - 28.0, 120.0, 20.0);
    if ui::button(&app.fonts, mouse, Button::new(close, "MERCI").focused(true))
        || is_key_pressed(KeyCode::Enter)
    {
        return Transition::Pop;
    }

    Transition::Stay
}
