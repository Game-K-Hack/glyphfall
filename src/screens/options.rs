//! Les réglages : volume de la musique, volume des bruitages.
//!
//! Chaque changement est appliqué et sauvegardé immédiatement, et fait entendre
//! le son concerné. Régler un volume sans l'entendre reviendrait à viser dans
//! le noir, et un bouton « valider » n'apporterait rien à deux réglages.

use macroquad::prelude::*;

use crate::app::{App, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};
use crate::music::Ambience;
use crate::settings::MAX_LEVEL;

const ROW_X: f32 = 40.0;
const ROW_WIDTH: f32 = canvas::WIDTH - ROW_X * 2.0;
const ROW_HEIGHT: f32 = 26.0;
const FIRST_ROW_Y: f32 = 62.0;
const ROW_STEP: f32 = 32.0;

/// Largeur du libellé, avant la jauge. « MUSIQUE MENUS » est le plus long.
const LABEL_WIDTH: f32 = 122.0;
/// Un cran de la jauge.
const SEGMENT_WIDTH: f32 = 11.0;
const SEGMENT_GAP: f32 = 2.0;

/// Les réglages, dans l'ordre d'affichage.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Row {
    /// Musique des menus.
    Music,
    /// Musique pendant une manche, réglée à part.
    MusicGame,
    Sfx,
}

const ROWS: [Row; 3] = [Row::Music, Row::MusicGame, Row::Sfx];

pub fn options_screen(app: &mut App, selected: &mut usize, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);

    *selected = (*selected).min(ROWS.len() - 1);

    ui::text_centered(
        &app.fonts,
        "OPTIONS",
        canvas::WIDTH / 2.0,
        24.0,
        fonts::TITLE,
        role::TITLE,
    );

    let mut change: Option<(Row, i8)> = None;

    for (index, row) in ROWS.iter().enumerate() {
        let bounds = row_rect(index);

        if ui::hit(bounds, mouse) {
            *selected = index;

            // Cliquer directement sur un cran y règle le volume : plus rapide
            // que de marteler une flèche pour traverser la jauge.
            if is_mouse_button_pressed(MouseButton::Left) {
                if let Some(level) = level_at(bounds, mouse) {
                    change = Some((*row, level as i8 - level_of(app, *row) as i8));
                }
            }
        }

        draw_row(&app.fonts, bounds, *row, level_of(app, *row), index == *selected);
    }

    let step = if is_key_pressed(KeyCode::Right) {
        1
    } else if is_key_pressed(KeyCode::Left) {
        -1
    } else {
        0
    };
    if step != 0 {
        change = Some((ROWS[*selected], step));
    }

    if is_key_pressed(KeyCode::Down) {
        *selected = (*selected + 1) % ROWS.len();
        app.sfx.navigate();
    }
    if is_key_pressed(KeyCode::Up) {
        *selected = (*selected + ROWS.len() - 1) % ROWS.len();
        app.sfx.navigate();
    }

    if let Some((row, delta)) = change {
        apply(app, row, delta);
    }

    // Tant que la ligne de la manche est choisie, la musique des menus se joue
    // au volume de la manche : sans cet aperçu, on réglerait à l'aveugle un son
    // qui ne s'entend qu'en partie.
    if ROWS[*selected] == Row::MusicGame {
        app.music.preview(Ambience::Game);
    }

    ui::text_centered(
        &app.fonts,
        "GAUCHE ET DROITE POUR REGLER",
        canvas::WIDTH / 2.0,
        162.0,
        fonts::TEXT,
        role::TEXT_DISABLED,
    );

    let back = Rect::new(((canvas::WIDTH - 120.0) / 2.0).floor(), 176.0, 120.0, 20.0);
    if ui::button(&app.fonts, mouse, Button::new(back, "RETOUR").accent(role::TEXT_MUTED)) {
        return Transition::Pop;
    }

    Transition::Stay
}

fn row_rect(index: usize) -> Rect {
    Rect::new(ROW_X, FIRST_ROW_Y + index as f32 * ROW_STEP, ROW_WIDTH, ROW_HEIGHT)
}

/// La jauge, à droite du libellé.
fn gauge_rect(bounds: Rect) -> Rect {
    let width = MAX_LEVEL as f32 * (SEGMENT_WIDTH + SEGMENT_GAP) - SEGMENT_GAP;
    Rect::new(bounds.x + LABEL_WIDTH, bounds.y + 7.0, width, 12.0)
}

fn level_of(app: &App, row: Row) -> u8 {
    match row {
        Row::Music => app.settings.music,
        Row::MusicGame => app.settings.music_game,
        Row::Sfx => app.settings.sfx,
    }
}

/// Sur quel cran de la jauge se trouve le curseur, s'il est dessus.
fn level_at(bounds: Rect, mouse: Vec2) -> Option<u8> {
    let gauge = gauge_rect(bounds);
    if !ui::hit(gauge, mouse) {
        return None;
    }

    let offset = mouse.x - gauge.x;
    let index = (offset / (SEGMENT_WIDTH + SEGMENT_GAP)).floor() as i32;

    Some((index + 1).clamp(0, MAX_LEVEL as i32) as u8)
}

/// Applique un changement de volume : réglage, moteur audio, sauvegarde, et
/// retour sonore.
fn apply(app: &mut App, row: Row, delta: i8) {
    let level = level_of(app, row);
    let next = (level as i8 + delta).clamp(0, MAX_LEVEL as i8) as u8;

    if next == level {
        return;
    }

    match row {
        Row::Music | Row::MusicGame => {
            if row == Row::Music {
                app.settings.music = next;
            } else {
                app.settings.music_game = next;
            }
            app.music.set_volumes(app.settings.music_gain(), app.settings.music_game_gain());
        }
        Row::Sfx => {
            app.settings.sfx = next;
            app.sfx.set_volume(app.settings.sfx_gain());
            // Le réglage des bruitages n'a rien qui joue en continu : sans ce
            // blip, on ne saurait pas ce que vaut le cran choisi.
            app.sfx.navigate();
        }
    }

    app.settings.save();
}

fn draw_row(fonts_set: &Fonts, bounds: Rect, row: Row, level: u8, selected: bool) {
    if selected {
        ui::panel(bounds, role::PANEL);
    }

    let label = match row {
        Row::Music => "MUSIQUE MENUS",
        Row::MusicGame => "MUSIQUE JEU",
        Row::Sfx => "BRUITAGES",
    };
    ui::text(
        fonts_set,
        label,
        bounds.x + 6.0,
        bounds.y + (ROW_HEIGHT - fonts::TEXT as f32) / 2.0,
        fonts::TEXT,
        if selected { role::TEXT } else { role::TEXT_MUTED },
    );

    let gauge = gauge_rect(bounds);
    for index in 0..MAX_LEVEL {
        let segment = Rect::new(
            gauge.x + index as f32 * (SEGMENT_WIDTH + SEGMENT_GAP),
            gauge.y,
            SEGMENT_WIDTH,
            gauge.h,
        );

        if index < level {
            ui::fill(segment, if selected { role::ACCENT } else { role::TEXT_MUTED });
        } else {
            // Les crans éteints restent visibles : on doit voir la course
            // complète du réglage, pas seulement la part atteinte. Le gris
            // ardoise ressort aussi bien sur le fond que sur le panneau de la
            // ligne sélectionnée.
            ui::stroke(segment, role::TEXT_DISABLED);
        }
    }

    // Un volume nul mérite un mot plutôt qu'une jauge vide, qui pourrait passer
    // pour un affichage cassé.
    let value =
        if level == 0 { "COUPE".to_string() } else { format!("{}%", level as u32 * 10) };
    let width = ui::text_width(fonts_set, &value, fonts::TEXT);
    ui::text(
        fonts_set,
        &value,
        bounds.x + bounds.w - 6.0 - width,
        bounds.y + (ROW_HEIGHT - fonts::TEXT as f32) / 2.0,
        fonts::TEXT,
        if level == 0 { role::DANGER } else { role::TEXT },
    );
}
