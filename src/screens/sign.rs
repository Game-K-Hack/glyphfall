//! La fiche d'un signe : à quoi il ressemble, comment il se lit, et comment le
//! retenir.
//!
//! C'est le seul écran où l'on peut s'arrêter sur un caractère sans qu'il
//! tombe. Le briefing en montre trente d'un coup ; ici il n'y en a qu'un, en
//! grand, avec tout ce qui aide à l'ancrer.

use macroquad::prelude::*;

use crate::app::{App, Transition};
use crate::data::{Glyph, Language, Level};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};

/// Le pavé qui porte le signe, à gauche.
const CARD: Rect = Rect { x: 16.0, y: 34.0, w: 96.0, h: 96.0 };
/// Taille du signe dans son pavé.
const GLYPH_SIZE: u16 = 64;

/// Colonne de droite, où vont les lectures et les moyens de retenir.
const TEXT_X: f32 = CARD.x + CARD.w + 14.0;
const TEXT_WIDTH: f32 = canvas::WIDTH - TEXT_X - 16.0;

pub fn sign_screen(
    app: &App,
    language_id: &str,
    level_id: &str,
    index: &mut usize,
    mouse: Vec2,
) -> Transition {
    clear_background(role::BACKGROUND);

    let Some(language) = app.catalog.language(language_id) else { return Transition::Pop };
    let Some(level) = language.level(level_id) else { return Transition::Pop };
    if level.glyphs.is_empty() {
        return Transition::Pop;
    }

    *index = (*index).min(level.glyphs.len() - 1);

    // Les flèches passent d'un signe à l'autre sans repasser par le briefing :
    // on lit rarement une seule fiche.
    if is_key_pressed(KeyCode::Right) {
        *index = (*index + 1) % level.glyphs.len();
    }
    if is_key_pressed(KeyCode::Left) {
        *index = (*index + level.glyphs.len() - 1) % level.glyphs.len();
    }

    let glyph = &level.glyphs[*index];

    draw_header(&app.fonts, level, *index);
    draw_card(app, language, glyph);
    draw_readings(&app.fonts, glyph);
    draw_mnemonics(&app.fonts, glyph);

    draw_navigation(app, level, index, mouse)
}

fn draw_header(fonts_set: &Fonts, level: &Level, index: usize) {
    ui::text_truncated(
        fonts_set,
        &level.title,
        8.0,
        8.0,
        fonts::TEXT,
        role::TITLE,
        canvas::WIDTH - 90.0,
    );

    let position = format!("{} / {}", index + 1, level.glyphs.len());
    let width = ui::text_width(fonts_set, &position, fonts::TEXT);
    ui::text(fonts_set, &position, canvas::WIDTH - 8.0 - width, 8.0, fonts::TEXT, role::TEXT_MUTED);

    draw_rectangle(8.0, 22.0, canvas::WIDTH - 16.0, 1.0, role::HIGHLIGHT);
}

/// Le signe en grand, et l'état de sa maîtrise.
fn draw_card(app: &App, language: &Language, glyph: &Glyph) {
    ui::panel(CARD, role::PANEL);
    ui::glyph_fitted(
        app.fonts.script(&language.id),
        &glyph.char,
        CARD,
        GLYPH_SIZE,
        role::TEXT,
    );

    // Un rappel de ce que le jeu sait du joueur sur ce signe précis : c'est ce
    // qui distingue une fiche d'un simple dictionnaire.
    let (label, color) = match app.progress.mastery(&language.id, &glyph.char) {
        score if score < 0 => ("A CONSOLIDER", role::SHAKY),
        0 => ("PAS ENCORE VU", role::TEXT_DISABLED),
        score if score >= 3 => ("ACQUIS", role::SUCCESS),
        _ => ("EN COURS", role::TEXT_MUTED),
    };
    ui::text_centered(
        &app.fonts,
        label,
        CARD.x + CARD.w / 2.0,
        CARD.y + CARD.h + 6.0,
        fonts::TEXT,
        color,
    );
}

/// Les romanisations acceptées, la principale en évidence.
fn draw_readings(fonts_set: &Fonts, glyph: &Glyph) {
    ui::text(fonts_set, glyph.primary_answer(), TEXT_X, 34.0, fonts::TITLE, role::ACCENT);

    if glyph.answers.len() > 1 {
        // Les variantes tolérées : les taire ferait croire à une seule bonne
        // réponse, alors que le jeu en accepte plusieurs.
        let others = glyph.answers[1..].join("  ");
        ui::text_truncated(
            fonts_set,
            &format!("aussi : {others}"),
            TEXT_X,
            54.0,
            fonts::TEXT,
            role::TEXT_MUTED,
            TEXT_WIDTH,
        );
    }
}

fn draw_mnemonics(fonts_set: &Fonts, glyph: &Glyph) {
    const TOP: f32 = 72.0;
    const LINE: f32 = 10.0;
    /// Au-delà, la fiche déborderait sur les boutons.
    const MAX_LINES: usize = 11;

    ui::text(fonts_set, "POUR LE RETENIR", TEXT_X, TOP, fonts::TEXT, role::TEXT_MUTED);

    let mut y = TOP + 14.0;
    let mut written = 0;

    for mnemonic in &glyph.mnemonics {
        // Le tiret n'est mis qu'à la première ligne : les suivantes s'alignent
        // sous le texte, pour que chaque moyen se lise comme un bloc.
        for (line_number, line) in
            ui::wrap(fonts_set, mnemonic, fonts::TEXT, TEXT_WIDTH - 8.0).iter().enumerate()
        {
            if written >= MAX_LINES {
                return;
            }
            if line_number == 0 {
                ui::text(fonts_set, "-", TEXT_X, y, fonts::TEXT, role::HINT);
            }
            ui::text(fonts_set, line, TEXT_X + 8.0, y, fonts::TEXT, role::TEXT);

            y += LINE;
            written += 1;
        }
        y += 4.0;
    }
}

fn draw_navigation(app: &App, level: &Level, index: &mut usize, mouse: Vec2) -> Transition {
    const Y: f32 = 194.0;
    const ARROW: f32 = 22.0;

    let previous = Rect::new(16.0, Y, ARROW, 16.0);
    if ui::button(&app.fonts, mouse, Button::new(previous, "<")) {
        *index = (*index + level.glyphs.len() - 1) % level.glyphs.len();
        app.sfx.navigate();
    }

    let next = Rect::new(16.0 + ARROW + 4.0, Y, ARROW, 16.0);
    if ui::button(&app.fonts, mouse, Button::new(next, ">")) {
        *index = (*index + 1) % level.glyphs.len();
        app.sfx.navigate();
    }

    let back = Rect::new(canvas::WIDTH - 16.0 - 76.0, Y, 76.0, 16.0);
    if ui::button(&app.fonts, mouse, Button::new(back, "RETOUR").accent(role::TEXT_MUTED)) {
        return Transition::Pop;
    }

    // Centré entre les flèches et le bouton, et non sur la toile : centré sur
    // la toile, le rappel passait sous le bouton de retour.
    ui::text_centered(
        &app.fonts,
        "FLECHES POUR CHANGER",
        (next.x + next.w + back.x) / 2.0,
        Y + 4.0,
        fonts::TEXT,
        role::TEXT_DISABLED,
    );

    Transition::Stay
}
