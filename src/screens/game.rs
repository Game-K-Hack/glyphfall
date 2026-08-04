//! La manche : les tuiles tombent, le joueur tape la lecture avant la ligne.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui;
use crate::gfx::{Fonts, canvas, fonts};
use crate::session::{PLAYFIELD_WIDTH, Session, TARGET_Y, TILE_HEIGHT};

/// Taille des glyphes sur les tuiles.
const GLYPH_SIZE: u16 = 24;

pub fn game_screen(app: &App, session: &mut Session) -> Transition {
    let outcome = session.update(get_frame_time());
    draw(app, session);

    match outcome {
        // `Replace` et non `Push` : l'écran de résultats prend la place de la
        // manche, pour que « retour » ramène au briefing et non à une partie
        // déjà finie.
        Some(outcome) => {
            Transition::Replace(Screen::Results { outcome: Box::new(outcome), elapsed: 0.0 })
        }
        None => Transition::Stay,
    }
}

fn draw(app: &App, session: &Session) {
    clear_background(role::BACKGROUND);

    let tile_width = session.tile_width();
    let playfield_x = session.playfield_x();
    let playfield_width = tile_width * session.rules.columns as f32;

    ui::fill(Rect::new(playfield_x, 0.0, playfield_width, canvas::HEIGHT), role::PANEL);
    for column in 0..=session.rules.columns {
        let x = playfield_x + column as f32 * tile_width;
        draw_rectangle(x, 0.0, 1.0, canvas::HEIGHT, role::BORDER);
    }

    // Sous la ligne, tout est perdu : elle est dessinée avant les tuiles pour
    // qu'une tuile sur le point de mourir passe visiblement par-dessus.
    draw_rectangle(playfield_x, TARGET_Y, playfield_width, 1.0, role::DANGER);

    let script = app.fonts.script(&session.language_id);
    for tile in &session.tiles {
        let rect = Rect::new(
            playfield_x + tile.column as f32 * tile_width + 1.0,
            tile.y.floor(),
            tile_width - 2.0,
            TILE_HEIGHT,
        );

        let background = if tile.cleared.is_some() { role::SUCCESS } else { role::BORDER };
        ui::panel(rect, background);
        ui::glyph_fitted(script, &tile.glyph.char, rect, GLYPH_SIZE, role::TEXT);
    }

    draw_hud(&app.fonts, session);
    draw_input_bar(&app.fonts, session);
}

fn draw_hud(fonts_set: &Fonts, session: &Session) {
    ui::text(fonts_set, &format!("{:05}", session.score), 6.0, 6.0, fonts::TEXT, role::TEXT);
    ui::hearts_row(6.0, 18.0, session.lives, session.rules.lives);

    // La gouttière est étroite : le titre y passe sur plusieurs lignes plutôt
    // que d'être coupé au troisième mot.
    let gutter = session.playfield_x() - 12.0;
    for (index, line) in ui::wrap(fonts_set, &session.level_title, fonts::TEXT, gutter)
        .iter()
        .take(3)
        .enumerate()
    {
        ui::text(fonts_set, line, 6.0, 34.0 + index as f32 * 10.0, fonts::TEXT, role::TEXT_DISABLED);
    }

    if session.rules.is_timed() {
        draw_timer(fonts_set, session);
    }
}

/// Le temps restant, en jauge et en chiffres, à droite de la zone de jeu.
fn draw_timer(fonts_set: &Fonts, session: &Session) {
    let x = session.playfield_x() + PLAYFIELD_WIDTH + 12.0;
    let width = canvas::WIDTH - x - 6.0;

    let remaining = session.time_left.ceil() as u32;
    let label = format!("{remaining}S");
    let label_width = ui::text_width(fonts_set, &label, fonts::TEXT);
    ui::text(fonts_set, &label, canvas::WIDTH - 6.0 - label_width, 6.0, fonts::TEXT, role::TEXT);

    // La jauge vire au rouge sur la fin : le chiffre seul se remarque mal quand
    // on a les yeux sur les tuiles.
    let ratio = session.time_ratio();
    let color = if ratio < 0.2 { role::DANGER } else { role::ACCENT };
    ui::progress_bar(Rect::new(x, 18.0, width, 8.0), ratio, color);
}

fn draw_input_bar(fonts_set: &Fonts, session: &Session) {
    const WIDTH: f32 = 160.0;
    let bar = Rect::new(((canvas::WIDTH - WIDTH) / 2.0).floor(), 192.0, WIDTH, 16.0);
    ui::panel(bar, role::BORDER);

    let (content, color) = if session.input.is_empty() {
        ("tapez la lecture", role::TEXT_DISABLED)
    } else {
        (session.input.as_str(), role::STAR)
    };
    ui::text_truncated(fonts_set, content, bar.x + 5.0, bar.y + 4.0, fonts::TEXT, color, WIDTH - 10.0);
}
