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
use crate::settings::{DAILY_GOALS, MAX_LEVEL, goal_label};

const ROW_X: f32 = 40.0;
const ROW_WIDTH: f32 = canvas::WIDTH - ROW_X * 2.0;
const ROW_HEIGHT: f32 = 26.0;
const FIRST_ROW_Y: f32 = 52.0;
const ROW_STEP: f32 = 30.0;

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
    /// Le temps d'apprentissage visé chaque jour.
    DailyGoal,
}

const ROWS: [Row; 4] = [Row::Music, Row::MusicGame, Row::Sfx, Row::DailyGoal];

pub fn options_screen(
    app: &mut App,
    selected: &mut usize,
    dragging: &mut Option<usize>,
    mouse: Vec2,
) -> Transition {
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

    // Le curseur relâché, plus rien n'est attrapé.
    if !is_mouse_button_down(MouseButton::Left) {
        *dragging = None;
    }

    for (index, row) in ROWS.iter().enumerate() {
        let bounds = row_rect(index);

        if ui::hit(bounds, mouse) {
            *selected = index;

            // Un appui sur la barre l'attrape : on peut ensuite la balayer sans
            // rester dessus, ce qui est le seul geste tenable au doigt.
            if is_mouse_button_pressed(MouseButton::Left)
                && value_at(*row, bounds, mouse).is_some()
            {
                *dragging = Some(index);
            }
        }

        if *dragging == Some(index) {
            let value = value_from_x(*row, bounds, mouse.x);
            change = Some((*row, value as i8 - level_of(app, *row) as i8));
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
        176.0,
        fonts::TEXT,
        role::TEXT_DISABLED,
    );

    let back = Rect::new(((canvas::WIDTH - 120.0) / 2.0).floor(), 190.0, 120.0, 20.0);
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

/// La barre de l'objectif, plus courte que les jauges de volume.
///
/// Une durée s'écrit plus long qu'un pourcentage : sans ce retrait, le texte
/// viendrait mordre sur le curseur. Le dessin et le clic passent tous deux par
/// cette fonction, faute de quoi on cliquerait à côté de ce que l'on voit.
fn goal_bar(bounds: Rect) -> Rect {
    const VALUE_ROOM: f32 = 26.0;

    let gauge = gauge_rect(bounds);
    Rect::new(gauge.x, gauge.y, gauge.w - VALUE_ROOM, gauge.h)
}

fn level_of(app: &App, row: Row) -> u8 {
    match row {
        Row::Music => app.settings.music,
        Row::MusicGame => app.settings.music_game,
        Row::Sfx => app.settings.sfx,
        // L'objectif se compte en crans de durée, pas en dixièmes de volume.
        Row::DailyGoal => app.settings.daily_goal_step() as u8,
    }
}

/// Le nombre de crans d'une ligne. Les volumes en ont dix, l'objectif neuf.
fn steps_of(row: Row) -> u8 {
    match row {
        Row::DailyGoal => DAILY_GOALS.len() as u8 - 1,
        _ => MAX_LEVEL,
    }
}

/// La valeur désignée par un appui, si celui-ci tombe sur la barre.
fn value_at(row: Row, bounds: Rect, mouse: Vec2) -> Option<u8> {
    match row {
        Row::DailyGoal => ui::slider_step_at(goal_bar(bounds), DAILY_GOALS.len(), mouse)
            .map(|step| step as u8),
        _ => {
            let gauge = gauge_rect(bounds);
            ui::hit(ui::grab_area(gauge), mouse).then(|| level_from_x(gauge, mouse.x))
        }
    }
}

/// La valeur désignée par une abscisse, pendant un glissement.
fn value_from_x(row: Row, bounds: Rect, x: f32) -> u8 {
    match row {
        Row::DailyGoal => ui::slider_step_from_x(goal_bar(bounds), DAILY_GOALS.len(), x) as u8,
        _ => level_from_x(gauge_rect(bounds), x),
    }
}

/// Le cran de jauge sous une abscisse. Le premier cran commence au bord, d'où
/// le décalage de un : à gauche de la jauge, le volume est nul.
fn level_from_x(gauge: Rect, x: f32) -> u8 {
    let offset = x - gauge.x;
    let index = (offset / (SEGMENT_WIDTH + SEGMENT_GAP)).floor() as i32;

    (index + 1).clamp(0, MAX_LEVEL as i32) as u8
}

/// Applique un changement de volume : réglage, moteur audio, sauvegarde, et
/// retour sonore.
fn apply(app: &mut App, row: Row, delta: i8) {
    let level = level_of(app, row);
    let next = (level as i8 + delta).clamp(0, steps_of(row) as i8) as u8;

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
        Row::DailyGoal => {
            app.settings.daily_goal = Some(DAILY_GOALS[next as usize]);
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
        Row::DailyGoal => "TEMPS PAR JOUR",
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

    // L'objectif n'est pas une quantité que l'on remplit mais une valeur que
    // l'on désigne : une jauge pleine à gauche du curseur mentirait.
    if row == Row::DailyGoal {
        let minutes = DAILY_GOALS[level as usize];
        let bar = goal_bar(bounds);

        ui::slider(bar, DAILY_GOALS.len(), level as usize, if selected { role::ACCENT } else { role::TEXT_MUTED });
        draw_value(fonts_set, bounds, &goal_label(minutes), minutes == 0);
        return;
    }

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
    draw_value(fonts_set, bounds, &value, level == 0);
}

/// La valeur, alignée à droite de la ligne.
fn draw_value(fonts_set: &Fonts, bounds: Rect, value: &str, muted: bool) {
    let width = ui::text_width(fonts_set, value, fonts::TEXT);
    ui::text(
        fonts_set,
        value,
        bounds.x + bounds.w - 6.0 - width,
        bounds.y + (ROW_HEIGHT - fonts::TEXT as f32) / 2.0,
        fonts::TEXT,
        if muted { role::DANGER } else { role::TEXT },
    );
}
