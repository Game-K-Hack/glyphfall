//! Le chemin d'apprentissage d'une langue : la suite des niveaux, ce qui est
//! ouvert, ce qui reste verrouillé, et les étoiles déjà gagnées.
//!
//! L'ordre affiché vient du champ `order` des fichiers TOML, mais c'est bien
//! `requires` qui décide de ce qui est jouable : deux niveaux peuvent se
//! débloquer en parallèle.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::core::GameState;
use crate::data::{Language, Level};
use crate::gfx::palette::role;
use crate::gfx::ui;
use crate::gfx::{Fonts, canvas, fonts};
use crate::progress::{MAX_STARS, Progress};

const VIEWPORT_TOP: f32 = 24.0;
const VIEWPORT_BOTTOM: f32 = 192.0;

const NODE_X: f32 = 20.0;
const NODE_SIZE: f32 = 22.0;
const ROW_HEIGHT: f32 = 24.0;
const ROW_STEP: f32 = 34.0;
const TEXT_X: f32 = NODE_X + NODE_SIZE + 10.0;

pub fn learning_path_screen(
    app: &App,
    language_id: &str,
    selected: &mut usize,
    mouse: Vec2,
) -> Transition {
    clear_background(role::BACKGROUND);

    let Some(language) = app.catalog.language(language_id) else {
        // La langue a disparu du catalogue entre-temps : impossible en pratique,
        // mais mieux vaut revenir en arrière que d'indexer dans le vide.
        return Transition::Pop;
    };

    *selected = (*selected).min(language.levels.len() - 1);
    draw_header(&app.fonts, language, &app.progress);

    let scroll = scroll_offset(*selected, language.levels.len());
    let mut chosen = None;

    for (index, level) in language.levels.iter().enumerate() {
        let row = row_rect(index, scroll);

        // On ne dessine que les lignes entièrement visibles : une ligne coupée
        // en deux par le bord du panneau se lirait mal.
        if row.y < VIEWPORT_TOP || row.y + row.h > VIEWPORT_BOTTOM {
            continue;
        }

        let unlocked = app.progress.is_unlocked(level);

        if ui::hit(row, mouse) {
            *selected = index;
            if unlocked && is_mouse_button_pressed(MouseButton::Left) {
                chosen = Some(level);
            }
        }

        // Le trait qui relie cette étape à la suivante.
        if index + 1 < language.levels.len() {
            let next = row_rect(index + 1, scroll);
            if next.y < VIEWPORT_BOTTOM {
                ui::dotted_line(
                    NODE_X + NODE_SIZE / 2.0,
                    row.y + NODE_SIZE,
                    next.y.min(VIEWPORT_BOTTOM),
                    role::TEXT_DISABLED,
                );
            }
        }

        draw_row(&app.fonts, level, row, index == *selected, unlocked, app.progress.stars(&level.id));
    }

    draw_footer(&app.fonts, language, *selected, &app.progress);

    if is_key_pressed(KeyCode::Down) {
        *selected = (*selected + 1) % language.levels.len();
    }
    if is_key_pressed(KeyCode::Up) {
        *selected = (*selected + language.levels.len() - 1) % language.levels.len();
    }
    if is_key_pressed(KeyCode::Enter) {
        let level = &language.levels[*selected];
        if app.progress.is_unlocked(level) {
            chosen = Some(level);
        }
    }

    match chosen {
        Some(level) => Transition::Push(Screen::Briefing {
            language: language.id.clone(),
            level: level.id.clone(),
        }),
        None => Transition::Stay,
    }
}

/// Décale le contenu pour garder la ligne sélectionnée à peu près centrée,
/// sans jamais dépasser les extrémités de la liste.
fn scroll_offset(selected: usize, count: usize) -> f32 {
    let viewport_height = VIEWPORT_BOTTOM - VIEWPORT_TOP;
    let content_height = count as f32 * ROW_STEP;
    let max_scroll = (content_height - viewport_height).max(0.0);

    let centered = selected as f32 * ROW_STEP - (viewport_height - ROW_HEIGHT) / 2.0;
    centered.clamp(0.0, max_scroll).floor()
}

fn row_rect(index: usize, scroll: f32) -> Rect {
    let y = VIEWPORT_TOP + 4.0 + index as f32 * ROW_STEP - scroll;
    Rect::new(NODE_X - 4.0, y, canvas::WIDTH - (NODE_X - 4.0) * 2.0, ROW_HEIGHT)
}

fn draw_header(fonts_set: &Fonts, language: &Language, progress: &Progress) {
    ui::text(fonts_set, &language.name, 8.0, 8.0, fonts::TEXT, role::TITLE);

    let (earned, total) = progress.language_stars(language);
    let label = format!("{earned}/{total}");
    let width = ui::text_width(fonts_set, &label, fonts::TEXT);

    let x = canvas::WIDTH - 8.0 - width;
    ui::text(fonts_set, &label, x, 8.0, fonts::TEXT, role::TEXT_MUTED);
    ui::star(x - ui::STAR_WIDTH - 3.0, 8.0, earned > 0);

    draw_rectangle(8.0, 20.0, canvas::WIDTH - 16.0, 1.0, role::HIGHLIGHT);
}

fn draw_row(
    fonts_set: &Fonts,
    level: &Level,
    row: Rect,
    selected: bool,
    unlocked: bool,
    stars: u8,
) {
    if selected {
        ui::stroke(row, role::ACCENT);
    }

    let node = Rect::new(NODE_X, row.y, NODE_SIZE, NODE_SIZE);
    let node_color = match (unlocked, stars) {
        (false, _) => role::PANEL,
        (true, 0) => role::ACCENT,
        (true, _) => role::SUCCESS,
    };
    ui::panel(node, node_color);

    if unlocked {
        ui::text_centered(
            fonts_set,
            &level.order.to_string(),
            node.x + NODE_SIZE / 2.0,
            node.y + (NODE_SIZE - fonts::TEXT as f32) / 2.0,
            fonts::TEXT,
            role::BORDER,
        );
    } else {
        ui::lock(node.x + 8.0, node.y + 8.0, role::TEXT_DISABLED);
    }

    let (title_color, subtitle_color) = if unlocked {
        (role::TEXT, role::TEXT_MUTED)
    } else {
        (role::TEXT_DISABLED, role::TEXT_DISABLED)
    };

    let stars_x = canvas::WIDTH - NODE_X - ui::stars_row_width(MAX_STARS);
    // Le titre s'arrête avant les étoiles, le sous-titre peut courir dessous.
    let title_width = stars_x - TEXT_X - 6.0;
    let subtitle_width = canvas::WIDTH - NODE_X - TEXT_X;

    ui::text_truncated(fonts_set, &level.title, TEXT_X, row.y + 2.0, fonts::TEXT, title_color, title_width);
    ui::text_truncated(
        fonts_set,
        &level.subtitle,
        TEXT_X,
        row.y + 13.0,
        fonts::TEXT,
        subtitle_color,
        subtitle_width,
    );

    if unlocked {
        ui::stars_row(stars_x, row.y + 2.0, stars, MAX_STARS);
    }
}

fn draw_footer(fonts_set: &Fonts, language: &Language, selected: usize, progress: &Progress) {
    let level = &language.levels[selected];

    let hint = if progress.is_unlocked(level) {
        "ENTREE JOUER   ECHAP RETOUR"
    } else {
        "TERMINE LES ETAPES PRECEDENTES"
    };

    ui::text_centered(fonts_set, hint, canvas::WIDTH / 2.0, 200.0, fonts::TEXT, role::TEXT_DISABLED);
}
