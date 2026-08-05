//! Le bilan d'une manche : combien d'étoiles, ce qu'il reste à revoir, et
//! quoi faire ensuite.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};
use crate::progress::MAX_STARS;
use crate::session::{EndReason, Outcome, Session};

/// Les étoiles du bilan sont dessinées plus grandes que celles des listes.
const STAR_SCALE: f32 = 3.0;
/// Temps mort avant la première étoile.
///
/// Sans lui, la première est déjà posée quand l'écran apparaît : on ne voit
/// jamais l'animation, seulement son résultat.
const STAR_OPENING: f32 = 0.6;
/// Délai entre l'apparition de deux étoiles, en secondes.
const STAR_DELAY: f32 = 0.35;
/// Durée du petit rebond à l'apparition d'une étoile.
const STAR_POP: f32 = 0.15;

/// Au-delà, la ligne de correction déborderait : le reste est annoncé en
/// nombre plutôt que passé sous silence.
const MAX_MISSED_SHOWN: usize = 8;

/// Délai avant que l'écran n'accepte quoi que ce soit.
///
/// La manche se termine souvent au milieu d'une frappe : la même touche qui
/// valide un signe relance une partie ici. Sans ce délai, finir en tapant
/// enchaîne aussitôt sur une nouvelle manche, sans même laisser voir le bilan.
///
/// Il couvre aussi l'apparition des étoiles : agir avant qu'elles ne soient
/// posées reviendrait à sauter sa propre récompense.
const GRACE: f32 = 1.5;

pub fn results_screen(app: &App, outcome: &Outcome, elapsed: &mut f32, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);
    *elapsed += get_frame_time();

    let ready = accepts_input(*elapsed);

    draw_verdict(&app.fonts, outcome);
    draw_stars(outcome.stars, *elapsed);
    draw_figures(&app.fonts, outcome);
    draw_missed(app, outcome);

    const BUTTON_WIDTH: f32 = 110.0;
    const GAP: f32 = 12.0;
    let x = ((canvas::WIDTH - (BUTTON_WIDTH * 2.0 + GAP)) / 2.0).floor();

    // Pendant le délai, le bouton reste dessiné mais éteint : le montrer déjà
    // en avant alors qu'il ne répond pas serait pire que de le griser.
    let retry = Rect::new(x, 184.0, BUTTON_WIDTH, 20.0);
    let restart = ui::button(&app.fonts, mouse, Button::new(retry, "REJOUER").focused(ready))
        || is_key_pressed(KeyCode::Enter);

    if ready && restart {
        let again = if outcome.is_revision {
            Session::revision(&app.catalog, &app.progress, &outcome.language_id)
        } else {
            Session::new(
                &app.catalog,
                &app.progress,
                &outcome.language_id,
                &outcome.level_id,
                outcome.mode,
            )
        };

        return match again {
            Some(session) => Transition::Replace(Screen::Playing(Box::new(session))),
            // Le niveau a disparu du catalogue : on ne peut que remonter.
            None => Transition::Pop,
        };
    }

    let path = Rect::new(x + BUTTON_WIDTH + GAP, 184.0, BUTTON_WIDTH, 20.0);
    if ui::button(&app.fonts, mouse, Button::new(path, "CHEMIN").accent(role::TEXT_MUTED)) && ready {
        // Le briefing est juste en dessous : il faut deux crans pour revenir
        // au chemin, sinon le bouton ne tient pas ce que son nom promet.
        app.sfx.navigate();
        return Transition::PopMany(2);
    }

    Transition::Stay
}

fn draw_verdict(fonts_set: &Fonts, outcome: &Outcome) {
    if outcome.is_revision {
        ui::text_centered(
            fonts_set,
            "REVISION",
            canvas::WIDTH / 2.0,
            32.0,
            fonts::TEXT,
            role::TEXT_MUTED,
        );
    }

    let (verdict, color) = match (outcome.stars, outcome.reason) {
        (0, EndReason::OutOfLives) => ("PLUS DE VIES", role::DANGER),
        (0, EndReason::TimeUp) => ("TEMPS ECOULE", role::DANGER),
        (MAX_STARS.., _) => ("PARFAIT", role::SUCCESS),
        _ => ("TERMINE", role::TITLE),
    };

    ui::text_centered(fonts_set, verdict, canvas::WIDTH / 2.0, 10.0, fonts::TITLE, color);
}

/// Les trois étoiles, dessinées pixel par pixel puis agrandies : les redessiner
/// à l'échelle garde les bords nets, contrairement à une image étirée.
///
/// Elles apparaissent une à une — l'attente entre deux étoiles est ce qui rend
/// la troisième satisfaisante.
fn draw_stars(earned: u8, elapsed: f32) {
    const Y: f32 = 46.0;
    let size = ui::STAR_WIDTH * STAR_SCALE;
    let gap = 12.0;
    let total = size * MAX_STARS as f32 + gap * (MAX_STARS - 1) as f32;
    let start_x = ((canvas::WIDTH - total) / 2.0).floor();

    for index in 0..MAX_STARS {
        let x = start_x + index as f32 * (size + gap);
        let due = STAR_OPENING + STAR_DELAY * index as f32;

        // Les étoiles non gagnées restent en place dès le début : ce sont des
        // emplacements vides, pas des récompenses à annoncer.
        if index >= earned {
            ui::star_scaled(x, Y, STAR_SCALE, false);
            continue;
        }
        if elapsed < due {
            continue;
        }

        // Petit rebond : l'étoile arrive un cran trop grande puis se pose.
        let popping = elapsed - due < STAR_POP;
        let scale = if popping { STAR_SCALE + 1.0 } else { STAR_SCALE };
        let offset = (ui::STAR_WIDTH * (scale - STAR_SCALE)) / 2.0;

        ui::star_scaled(x - offset, Y - offset, scale, true);
    }
}

fn draw_figures(fonts_set: &Fonts, outcome: &Outcome) {
    const Y: f32 = 76.0;

    let accuracy = format!("{}% DE REUSSITE", (outcome.accuracy * 100.0).round() as u32);
    ui::text_centered(fonts_set, &accuracy, canvas::WIDTH / 2.0, Y, fonts::TEXT, role::TEXT);

    let detail = format!("{} SIGNES   {:05} POINTS", outcome.hits, outcome.score);
    ui::text_centered(
        fonts_set,
        &detail,
        canvas::WIDTH / 2.0,
        Y + 12.0,
        fonts::TEXT,
        role::TEXT_MUTED,
    );

    if outcome.is_record {
        ui::text_centered(
            fonts_set,
            "NOUVEAU RECORD",
            canvas::WIDTH / 2.0,
            Y + 24.0,
            fonts::TEXT,
            role::STAR,
        );
    }
}

/// Les glyphes tombés sans être reconnus : la seule partie vraiment utile du
/// bilan pour apprendre, plus que le score.
fn draw_missed(app: &App, outcome: &Outcome) {
    const Y: f32 = 118.0;
    const CELL_WIDTH: f32 = 36.0;

    if outcome.missed_glyphs.is_empty() {
        if outcome.hits > 0 {
            ui::text_centered(
                &app.fonts,
                "AUCUN SIGNE MANQUE",
                canvas::WIDTH / 2.0,
                Y + 14.0,
                fonts::TEXT,
                role::SUCCESS,
            );
        }
        return;
    }

    ui::text_centered(&app.fonts, "A REVOIR", canvas::WIDTH / 2.0, Y, fonts::TEXT, role::TEXT_MUTED);

    let shown = outcome.missed_glyphs.len().min(MAX_MISSED_SHOWN);
    let start_x = ((canvas::WIDTH - shown as f32 * CELL_WIDTH) / 2.0).floor();
    let script = app.fonts.script(&outcome.language_id);

    for (index, character) in outcome.missed_glyphs.iter().take(shown).enumerate() {
        let cell = Rect::new(start_x + index as f32 * CELL_WIDTH, Y + 12.0, CELL_WIDTH, 22.0);
        ui::glyph_fitted(script, character, cell, 18, role::TEXT);

        // La lecture attendue sous le glyphe : c'est elle qu'il fallait taper.
        if let Some(answer) = answer_for(app, outcome, character) {
            ui::text_centered(
                &app.fonts,
                answer,
                cell.x + cell.w / 2.0,
                cell.y + cell.h + 1.0,
                fonts::TEXT,
                role::HINT,
            );
        }
    }

    if outcome.missed_glyphs.len() > shown {
        let remaining = outcome.missed_glyphs.len() - shown;
        ui::text_centered(
            &app.fonts,
            &format!("+{remaining}"),
            canvas::WIDTH / 2.0,
            Y + 46.0,
            fonts::TEXT,
            role::TEXT_DISABLED,
        );
    }
}

/// Retrouve la lecture de référence d'un glyphe raté.
///
/// La recherche se fait sur toute la langue et non sur le seul niveau : un
/// glyphe raté peut venir de la part de révision.
fn answer_for<'a>(app: &'a App, outcome: &Outcome, character: &str) -> Option<&'a str> {
    let language = app.catalog.language(&outcome.language_id)?;

    language
        .levels
        .iter()
        .flat_map(|level| level.glyphs.iter())
        .find(|glyph| glyph.char == character)
        .map(|glyph| glyph.primary_answer())
}

/// L'écran répond-il déjà aux touches et aux clics ?
fn accepts_input(elapsed: f32) -> bool {
    elapsed >= GRACE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_ignores_the_keystroke_that_ended_the_round() {
        // La touche qui valide un signe est celle qui relance une partie : une
        // frappe a cheval sur la fin de manche enchainerait sans laisser voir
        // le bilan.
        assert!(!accepts_input(0.0));
        assert!(!accepts_input(GRACE - 0.01));
        assert!(accepts_input(GRACE));
    }

    #[test]
    fn the_delay_outlasts_the_star_animation() {
        // Pouvoir relancer avant que les etoiles ne soient posees reviendrait a
        // sauter sa propre recompense.
        let animation_end = STAR_OPENING + STAR_DELAY * (MAX_STARS - 1) as f32 + STAR_POP;

        assert!(GRACE >= animation_end, "delai {GRACE}, animation {animation_end}");
    }
}
