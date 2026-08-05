//! Ce que l'on va apprendre, avant de lancer la manche.
//!
//! L'écran montre tous les glyphes du niveau avec leur romanisation : c'est le
//! temps d'observation, la seule occasion de voir les réponses avant que les
//! tuiles ne se mettent à tomber.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::data::{Language, Level};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};
use crate::progress::{MAX_STARS, Progress};
use crate::session::{Mode, Session};

const GRID_X: f32 = 16.0;
const GRID_Y: f32 = 32.0;
const CELL_WIDTH: f32 = 32.0;
const CELL_HEIGHT: f32 = 26.0;
const COLUMNS: usize = 11;
const ROWS: usize = 3;
const GLYPH_SIZE: u16 = 16;

pub fn briefing_screen(
    app: &App,
    language_id: &str,
    level_id: &str,
    mode: &mut Mode,
    mouse: Vec2,
) -> Transition {
    clear_background(role::BACKGROUND);

    let Some(language) = app.catalog.language(language_id) else { return Transition::Pop };
    let Some(level) = language.level(level_id) else { return Transition::Pop };

    draw_header(&app.fonts, level);
    let (hovered, clicked) = draw_glyphs(app, language, level, mouse);
    draw_rules(&app.fonts, level);


    // Cliquer un signe ouvre sa fiche : le briefing ne peut montrer qu'une
    // ligne d'aide, la fiche a la place de tout dire.
    if let Some(index) = clicked {
        app.sfx.confirm();
        return Transition::Push(Screen::Sign {
            language: language_id.to_string(),
            level: level_id.to_string(),
            index,
        });
    }

    draw_modes(app, level_id, mode, mouse);

    let unlocked = app.progress.mode_unlocked(level_id, *mode);

    // Trois messages se disputent le pied de l'écran. L'ordre dit lequel
    // compte : ce qui bloque le mode choisi passe avant tout, puis l'aide du
    // signe survolé, puis le rappel par défaut.
    if !unlocked {
        ui::text_centered(
            &app.fonts,
            Progress::unlock_requirement(*mode),
            canvas::WIDTH / 2.0,
            200.0,
            fonts::TEXT,
            role::SHAKY,
        );
    } else {
        match hovered {
            Some(hint) if !hint.is_empty() => draw_hint(&app.fonts, language, hint),
            _ => ui::text_centered(
                &app.fonts,
                "CLIQUE UN SIGNE POUR SA FICHE",
                canvas::WIDTH / 2.0,
                200.0,
                fonts::TEXT,
                role::TEXT_DISABLED,
            ),
        }
    }

    let start = Rect::new(((canvas::WIDTH - 120.0) / 2.0).floor(), 172.0, 120.0, 20.0);

    // Le bouton reste dessiné même quand le mode est fermé : le retirer ferait
    // sauter la mise en page à chaque changement de mode.
    let pressed = ui::button(
        &app.fonts,
        mouse,
        Button::new(start, "START").accent(role::SUCCESS).focused(unlocked),
    );

    if unlocked && (pressed || is_key_pressed(KeyCode::Enter)) {
        if let Some(session) =
            Session::new(&app.catalog, &app.progress, language_id, level_id, *mode)
        {
            app.sfx.confirm();
            return Transition::Push(Screen::Playing(Box::new(session)));
        }
    }

    Transition::Stay
}

/// La rangée des quatre modes.
///
/// Les modes fermés restent affichés, avec ce qu'il faut faire pour les ouvrir.
/// Les cacher priverait le joueur de la seule chose qui donne envie d'aller
/// chercher les trois étoiles.
fn draw_modes(app: &App, level_id: &str, mode: &mut Mode, mouse: Vec2) {
    const Y: f32 = 152.0;
    const WIDTH: f32 = 76.0;
    const GAP: f32 = 4.0;

    let total = WIDTH * Mode::ALL.len() as f32 + GAP * (Mode::ALL.len() - 1) as f32;
    let start_x = ((canvas::WIDTH - total) / 2.0).floor();

    // Un mode fermé se sélectionne quand même : c'est le seul moyen de lire ce
    // qu'il demande. Sauter par-dessus rendrait la condition inatteignable,
    // et le joueur ne saurait jamais comment ouvrir la suite.
    let step: i32 = match () {
        _ if is_key_pressed(KeyCode::Right) => 1,
        _ if is_key_pressed(KeyCode::Left) => -1,
        _ => 0,
    };
    if step != 0 {
        let count = Mode::ALL.len() as i32;
        let from = Mode::ALL.iter().position(|candidate| candidate == mode).unwrap_or(0) as i32;

        *mode = Mode::ALL[(from + step).rem_euclid(count) as usize];
        app.sfx.navigate();
    }

    for (index, candidate) in Mode::ALL.iter().enumerate() {
        let cell = Rect::new(start_x + index as f32 * (WIDTH + GAP), Y, WIDTH, 14.0);
        let unlocked = app.progress.mode_unlocked(level_id, *candidate);
        let chosen = *candidate == *mode;

        if ui::hit(cell, mouse) && is_mouse_button_pressed(MouseButton::Left) {
            *mode = *candidate;
            app.sfx.navigate();
        }

        // Un mode fermé mais choisi garde un contour, sans la couleur pleine :
        // il est bien désigné, il n'est simplement pas jouable.
        let background = match (unlocked, chosen) {
            (true, true) => role::ACCENT,
            _ => role::PANEL,
        };
        ui::panel(cell, background);
        if !unlocked && chosen {
            ui::stroke(cell, role::SHAKY);
        }

        let label_color = match (unlocked, chosen) {
            (false, _) => role::TEXT_DISABLED,
            (true, true) => role::BORDER,
            (true, false) => role::TEXT,
        };
        ui::text_centered(
            &app.fonts,
            candidate.label(),
            cell.x + cell.w / 2.0,
            cell.y + 3.0,
            fonts::TEXT,
            label_color,
        );
    }

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

/// Dessine la grille des signes, et renvoie l'aide de celui survolé ainsi que
/// l'indice de celui que l'on vient de cliquer.
fn draw_glyphs<'a>(
    app: &App,
    language: &'a Language,
    level: &'a Level,
    mouse: Vec2,
) -> (Option<&'a str>, Option<usize>) {
    let fonts_set = &app.fonts;
    let script = fonts_set.script(&language.id);
    let capacity = COLUMNS * ROWS;
    let mut hovered = None;
    let mut clicked = None;

    for (index, glyph) in level.glyphs.iter().take(capacity).enumerate() {
        let cell = Rect::new(
            GRID_X + (index % COLUMNS) as f32 * CELL_WIDTH,
            GRID_Y + (index / COLUMNS) as f32 * CELL_HEIGHT,
            CELL_WIDTH,
            CELL_HEIGHT,
        );

        let is_hovered = ui::hit(cell, mouse);
        if is_hovered {
            hovered = Some(glyph.hint());
            ui::fill(cell, role::PANEL);
            if is_mouse_button_pressed(MouseButton::Left) {
                clicked = Some(index);
            }
        }

        let glyph_box = Rect::new(cell.x, cell.y, cell.w, GLYPH_SIZE as f32 + 2.0);
        ui::glyph_fitted(script, &glyph.char, glyph_box, GLYPH_SIZE, role::TEXT);
        // Les signes déjà ratés par le passé ressortent : le briefing dit ainsi
        // quoi travailler, au lieu de présenter une liste uniforme où les
        // faiblesses se noient.
        let color = match (is_hovered, app.progress.is_shaky(&language.id, &glyph.char)) {
            (true, _) => role::ACCENT,
            (false, true) => role::SHAKY,
            (false, false) => role::TEXT_MUTED,
        };
        ui::text_centered(
            fonts_set,
            glyph.primary_answer(),
            cell.x + cell.w / 2.0,
            cell.y + CELL_HEIGHT - 10.0,
            fonts::TEXT,
            color,
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

    (hovered, clicked)
}

fn draw_rules(fonts_set: &Fonts, level: &Level) {
    const Y: f32 = 112.0;

    let rules = &level.rules;
    let duration = if rules.is_timed() {
        format!("{}S", rules.duration as u32)
    } else {
        "SANS LIMITE".to_string()
    };
    let summary = format!("{} VIES   {duration}   {} COLONNES", rules.lives, rules.columns);
    ui::text_centered(fonts_set, &summary, canvas::WIDTH / 2.0, Y, fonts::TEXT, role::TEXT);

    // Les seuils, pour savoir ce qu'il faut viser avant de commencer.
    const THRESHOLD_Y: f32 = 126.0;
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
