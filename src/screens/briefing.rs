//! Ce que l'on va apprendre, avant de lancer la manche.
//!
//! L'écran montre tous les glyphes du niveau avec leur romanisation : c'est le
//! temps d'observation, la seule occasion de voir les réponses avant que les
//! tuiles ne se mettent à tomber.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::core::GameState;
use crate::data::{Language, Level};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};
use crate::progress::MAX_STARS;

const GRID_X: f32 = 16.0;
const GRID_Y: f32 = 32.0;
const CELL_WIDTH: f32 = 32.0;
const CELL_HEIGHT: f32 = 26.0;
const COLUMNS: usize = 11;
const ROWS: usize = 3;
const GLYPH_SIZE: u16 = 16;

pub fn briefing_screen(app: &App, language_id: &str, level_id: &str, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);

    let Some(language) = app.catalog.language(language_id) else { return Transition::Pop };
    let Some(level) = language.level(level_id) else { return Transition::Pop };

    draw_header(&app.fonts, level);
    let hovered = draw_glyphs(&app.fonts, language, level, mouse);
    draw_rules(&app.fonts, level);

    // Survoler un glyphe remplace le rappel des touches par son aide
    // mnémotechnique : les deux ne tiendraient pas ensemble à cette taille.
    match hovered {
        Some(hint) if !hint.is_empty() => draw_hint(&app.fonts, language, hint),
        _ => ui::text_centered(
            &app.fonts,
            "ECHAP RETOUR",
            canvas::WIDTH / 2.0,
            200.0,
            fonts::TEXT,
            role::TEXT_DISABLED,
        ),
    }

    let start = Rect::new(((canvas::WIDTH - 120.0) / 2.0).floor(), 172.0, 120.0, 20.0);
    // Mis en avant d'office : c'est la seule action attendue de cet écran.
    let pressed = ui::button(
        &app.fonts,
        mouse,
        Button::new(start, "START").accent(role::SUCCESS).focused(true),
    );

    if pressed || is_key_pressed(KeyCode::Enter) {
        // Provisoire : la partie ne connaît pas encore les règles du niveau.
        return Transition::Push(Screen::Playing(GameState::new()));
    }

    Transition::Stay
}

fn draw_header(fonts_set: &Fonts, level: &Level) {
    ui::text_truncated(
        fonts_set,
        &level.title,
        8.0,
        6.0,
        fonts::TEXT,
        role::TITLE,
        canvas::WIDTH - 16.0,
    );
    ui::text_truncated(
        fonts_set,
        &level.subtitle,
        8.0,
        17.0,
        fonts::TEXT,
        role::TEXT_MUTED,
        canvas::WIDTH - 16.0,
    );

    let count = format!("{} SIGNES", level.glyphs.len());
    let width = ui::text_width(fonts_set, &count, fonts::TEXT);
    ui::text(fonts_set, &count, canvas::WIDTH - 8.0 - width, 6.0, fonts::TEXT, role::TEXT_MUTED);
}

/// Dessine la grille des glyphes et renvoie l'aide de celui survolé.
fn draw_glyphs<'a>(
    fonts_set: &Fonts,
    language: &'a Language,
    level: &'a Level,
    mouse: Vec2,
) -> Option<&'a str> {
    let script = fonts_set.script(&language.id);
    let capacity = COLUMNS * ROWS;
    let mut hovered = None;

    for (index, glyph) in level.glyphs.iter().take(capacity).enumerate() {
        let cell = Rect::new(
            GRID_X + (index % COLUMNS) as f32 * CELL_WIDTH,
            GRID_Y + (index / COLUMNS) as f32 * CELL_HEIGHT,
            CELL_WIDTH,
            CELL_HEIGHT,
        );

        let is_hovered = ui::hit(cell, mouse);
        if is_hovered {
            hovered = Some(glyph.hint.as_str());
            ui::fill(cell, role::PANEL);
        }

        let glyph_box = Rect::new(cell.x, cell.y, cell.w, GLYPH_SIZE as f32 + 2.0);
        ui::glyph_fitted(script, &glyph.char, glyph_box, GLYPH_SIZE, role::TEXT);
        ui::text_centered(
            fonts_set,
            glyph.primary_answer(),
            cell.x + cell.w / 2.0,
            cell.y + CELL_HEIGHT - 10.0,
            fonts::TEXT,
            if is_hovered { role::ACCENT } else { role::TEXT_MUTED },
        );
    }

    // Un niveau plus fourni que la grille : on annonce ce qui n'est pas montré
    // plutôt que de le passer sous silence.
    if level.glyphs.len() > capacity {
        let remaining = level.glyphs.len() - capacity;
        ui::text(
            fonts_set,
            &format!("+{remaining} AUTRES"),
            GRID_X,
            GRID_Y + ROWS as f32 * CELL_HEIGHT + 2.0,
            fonts::TEXT,
            role::TEXT_DISABLED,
        );
    }

    hovered
}

fn draw_rules(fonts_set: &Fonts, level: &Level) {
    const Y: f32 = 122.0;

    let rules = &level.rules;
    let duration = if rules.is_timed() {
        format!("{}S", rules.duration as u32)
    } else {
        "SANS LIMITE".to_string()
    };
    let summary = format!("{} VIES   {duration}   {} COLONNES", rules.lives, rules.columns);
    ui::text_centered(fonts_set, &summary, canvas::WIDTH / 2.0, Y, fonts::TEXT, role::TEXT);

    // Les seuils, pour savoir ce qu'il faut viser avant de commencer.
    const THRESHOLD_Y: f32 = 140.0;
    let thresholds = [level.stars.one, level.stars.two, level.stars.three];
    let block_width = 100.0;
    let start_x = (canvas::WIDTH - block_width * MAX_STARS as f32) / 2.0;

    for (index, threshold) in thresholds.iter().enumerate() {
        let center = start_x + block_width * index as f32 + block_width / 2.0;
        let count = index as u8 + 1;

        ui::stars_row(
            center - ui::stars_row_width(count) / 2.0,
            THRESHOLD_Y,
            count,
            count,
        );
        ui::text_centered(
            fonts_set,
            &format!("{}%", (threshold * 100.0).round() as u32),
            center,
            THRESHOLD_Y + 10.0,
            fonts::TEXT,
            role::TEXT_MUTED,
        );
    }
}

/// L'aide est écrite avec la police de la langue : elle cite souvent les
/// glyphes eux-mêmes, que la police d'interface ne sait pas dessiner.
fn draw_hint(fonts_set: &Fonts, language: &Language, hint: &str) {
    let script = fonts_set.script(&language.id);
    let box_ = Rect::new(0.0, 196.0, canvas::WIDTH, 12.0);

    ui::glyph_fitted(script, hint, box_, 10, role::HINT);
}
