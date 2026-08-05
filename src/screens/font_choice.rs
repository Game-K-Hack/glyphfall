//! La question des tracés variés, posée une fois avant la première partie.
//!
//! Elle n'est pas anodine : jouer avec un tracé unique est nettement plus
//! facile, et beaucoup de joueurs préféreront commencer ainsi. L'écran montre
//! donc le même signe dans tous ses tracés plutôt que de le décrire — la
//! différence se voit en une seconde, elle s'explique mal en une phrase.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{canvas, fonts};

/// Le signe montré en exemple, et l'écriture dont il vient.
///
/// Le hangeul, parce que son tracé au stylo s'écarte le plus de l'imprimé :
/// c'est l'exemple qui démontre le mieux ce que la question propose.
const SAMPLE_LANGUAGE: &str = "ko";
const SAMPLE_GLYPH: &str = "ㅎ";

pub fn font_choice_screen(app: &mut App, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);

    ui::text_centered(
        &app.fonts,
        "TRACES VARIES",
        canvas::WIDTH / 2.0,
        22.0,
        fonts::TITLE,
        role::TITLE,
    );

    for (index, line) in [
        "Un signe change de dessin selon la police,",
        "comme nos lettres imprimees ou manuscrites.",
    ]
    .iter()
    .enumerate()
    {
        ui::text_centered(
            &app.fonts,
            line,
            canvas::WIDTH / 2.0,
            48.0 + index as f32 * 11.0,
            fonts::TEXT,
            role::TEXT_MUTED,
        );
    }

    draw_samples(app);

    ui::text_centered(
        &app.fonts,
        "Les varier rend l'apprentissage plus solide,",
        canvas::WIDTH / 2.0,
        144.0,
        fonts::TEXT,
        role::TEXT,
    );
    ui::text_centered(
        &app.fonts,
        "mais les parties plus difficiles.",
        canvas::WIDTH / 2.0,
        155.0,
        fonts::TEXT,
        role::TEXT,
    );

    const WIDTH: f32 = 130.0;
    const GAP: f32 = 12.0;
    let x = ((canvas::WIDTH - (WIDTH * 2.0 + GAP)) / 2.0).floor();

    let vary = Rect::new(x, 180.0, WIDTH, 20.0);
    if ui::button(&app.fonts, mouse, Button::new(vary, "VARIER").focused(true))
        || is_key_pressed(KeyCode::Enter)
    {
        return chosen(app, true);
    }

    let single = Rect::new(x + WIDTH + GAP, 180.0, WIDTH, 20.0);
    if ui::button(&app.fonts, mouse, Button::new(single, "UN SEUL").accent(role::TEXT_MUTED)) {
        return chosen(app, false);
    }

    Transition::Stay
}

/// Le même signe dans chacun des tracés disponibles, côte à côte.
fn draw_samples(app: &App) {
    const Y: f32 = 74.0;
    const GAP: f32 = 6.0;
    /// Largeur laissée à la rangée, marges comprises.
    const BAND: f32 = 344.0;
    /// Au-delà, une seule écriture remplirait l'écran de démonstration.
    const MAX_SIZE: f32 = 58.0;

    let count = app.fonts.script_count(SAMPLE_LANGUAGE).max(1);

    // Les cases se serrent pour tenir en une rangée : les faire déborder ou les
    // replier sur deux lignes casserait la lecture d'un coup d'oeil, qui est
    // tout ce que cet écran a à offrir.
    let size = ((BAND - GAP * (count - 1) as f32) / count as f32).min(MAX_SIZE).floor();
    let total = size * count as f32 + GAP * (count - 1) as f32;
    let start_x = ((canvas::WIDTH - total) / 2.0).floor();

    for index in 0..count {
        let cell = Rect::new(start_x + index as f32 * (size + GAP), Y, size, size);
        ui::panel(cell, role::PANEL);
        ui::glyph_fitted(
            app.fonts.script_variant(SAMPLE_LANGUAGE, index),
            SAMPLE_GLYPH,
            cell,
            (size * 0.7) as u16,
            role::TEXT,
        );
    }
}

fn chosen(app: &mut App, random: bool) -> Transition {
    app.settings.random_fonts = Some(random);
    app.settings.save();
    app.sfx.confirm();

    // `Replace` et non `Push` : la question ne doit pas réapparaître en revenant
    // du choix de la langue.
    Transition::Replace(Screen::LanguageSelect { selected: 0 })
}
