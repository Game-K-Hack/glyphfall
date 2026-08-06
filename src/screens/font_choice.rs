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

/// Largeur laissée aux paragraphes, marges comprises.
const TEXTE: f32 = canvas::WIDTH - 16.0;

pub fn font_choice_screen(app: &mut App, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);

    ui::text_centered(
        &app.fonts,
        "TRACES VARIES",
        canvas::WIDTH / 2.0,
        canvas::pick(56.0, 22.0),
        fonts::TITLE,
        role::TITLE,
    );

    ui::paragraph(
        &app.fonts,
        "Un signe change de dessin selon la police, comme nos lettres imprimees ou manuscrites.",
        canvas::WIDTH / 2.0,
        canvas::pick(88.0, 48.0),
        fonts::TEXT,
        role::TEXT_MUTED,
        TEXTE,
    );

    draw_samples(app);

    ui::paragraph(
        &app.fonts,
        "Les varier rend l'apprentissage plus solide, mais les parties plus difficiles.",
        canvas::WIDTH / 2.0,
        canvas::pick(256.0, 144.0),
        fonts::TEXT,
        role::TEXT,
        TEXTE,
    );

    // En paysage les deux réponses tiennent côte à côte ; en portrait elles
    // s'empilent, deux boutons de cent trente pixels ne rentrant pas dans deux
    // cent seize.
    const WIDTH: f32 = canvas::pick(160.0, 130.0);
    const GAP: f32 = 12.0;
    let total = if canvas::PORTRAIT { WIDTH } else { WIDTH * 2.0 + GAP };
    let x = ((canvas::WIDTH - total) / 2.0).floor();

    let vary = Rect::new(x, canvas::pick(310.0, 180.0), WIDTH, 20.0);
    if ui::button(&app.fonts, mouse, Button::new(vary, "VARIER").focused(true))
        || is_key_pressed(KeyCode::Enter)
    {
        return chosen(app, true);
    }

    let single = if canvas::PORTRAIT {
        Rect::new(x, 338.0, WIDTH, 20.0)
    } else {
        Rect::new(x + WIDTH + GAP, 180.0, WIDTH, 20.0)
    };
    if ui::button(&app.fonts, mouse, Button::new(single, "UN SEUL").accent(role::TEXT_MUTED)) {
        return chosen(app, false);
    }

    Transition::Stay
}

/// Le même signe dans chacun des tracés disponibles, côte à côte.
fn draw_samples(app: &App) {
    const Y: f32 = canvas::pick(146.0, 74.0);
    const GAP: f32 = 6.0;
    /// Largeur laissée aux rangées, marges comprises.
    const BAND: f32 = canvas::pick(196.0, 344.0);
    /// Au-delà de quatre par rangée, les cases deviennent trop petites pour que
    /// l'on distingue ce que la question demande de comparer.
    const PER_ROW: usize = if canvas::PORTRAIT { 4 } else { 8 };

    let count = app.fonts.script_count(SAMPLE_LANGUAGE).max(1);
    let columns = count.min(PER_ROW);
    let size = ((BAND - GAP * (columns - 1) as f32) / columns as f32).floor();

    // Plusieurs rangées plutôt qu'une seule rétrécie : la démonstration ne vaut
    // que si l'on voit les tracés, et sept cases en largeur les réduiraient à
    // des taches.
    for index in 0..count {
        let row = index / PER_ROW;
        let in_row = index % PER_ROW;
        let of_row = (count - row * PER_ROW).min(PER_ROW);

        let total = size * of_row as f32 + GAP * (of_row - 1) as f32;
        let start_x = ((canvas::WIDTH - total) / 2.0).floor();

        let cell = Rect::new(
            start_x + in_row as f32 * (size + GAP),
            Y + row as f32 * (size + GAP),
            size,
            size,
        );
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
