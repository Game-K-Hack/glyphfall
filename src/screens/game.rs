//! La manche : les tuiles tombent, le joueur tape la lecture avant la ligne.
//!
//! En portrait, l'écran se lit de haut en bas : le bandeau, la zone de chute,
//! la saisie, puis le clavier. Les gouttières latérales du paysage n'ont plus
//! lieu d'être — la largeur revient entièrement aux tuiles.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};
use crate::screens::keyboard::{self, Key};
use crate::session::{Event, Session, TARGET_Y, TILE_HEIGHT};

/// Taille des glyphes sur les tuiles.
const GLYPH_SIZE: u16 = 24;

/// Hauteur du bandeau du haut : score, vies, chronomètre.
///
/// Nulle en paysage, où le tableau de bord tient dans les gouttières de part et
/// d'autre de la zone de chute — un écran couché a de la largeur à revendre et
/// de la hauteur à économiser.
const HUD_HEIGHT: f32 = canvas::pick(38.0, 0.0);

/// La barre qui montre ce que le joueur est en train de taper.
const INPUT_BAR: Rect = Rect {
    x: canvas::pick(8.0, 112.0),
    y: canvas::pick(256.0, 192.0),
    w: canvas::pick(canvas::WIDTH - 16.0, 160.0),
    h: canvas::pick(18.0, 16.0),
};

pub fn game_screen(app: &App, session: &mut Session, mouse: Vec2) -> Transition {
    let outcome = session.update(get_frame_time());

    // La manche remonte ce qui vient de se produire ; c'est l'ecran qui decide
    // de le sonoriser.
    for event in session.take_events() {
        match event {
            Event::Hit => app.sfx.hit(),
            Event::Wrong => app.sfx.wrong(),
            Event::Missed => app.sfx.missed(),
        }
    }

    draw(app, session);

    // Abandonner une manche : loin des doigts, qui vivent sur le clavier en
    // bas. Une partie qu'on ne peut pas quitter enferme le joueur, mais un
    // bouton à portée de pouce la ferait perdre par accident. Couché, Échap
    // suffit et l'écran n'a pas de place à donner.
    if canvas::PORTRAIT {
        let quit = Rect::new(canvas::WIDTH - 26.0, 26.0, 20.0, 12.0);
        if ui::button(&app.fonts, mouse, Button::new(quit, "X").accent(role::DANGER)) {
            return Transition::Pop;
        }

        // Le clavier est traité après le rendu de la manche : l'appui porte
        // ainsi sur ce qui est à l'écran, et non sur la frame d'avant.
        if let Some(key) = keyboard::draw(&app.fonts, mouse) {
            app.sfx.navigate();
            match key {
                Key::Letter(letter) => session.type_letter(letter),
                Key::Erase => session.erase(),
                Key::Submit => session.submit(),
            }
        }
    }

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
    // Seule la zone de jeu tremble : un bandeau qui bouge se lit mal.
    let playfield_x = session.playfield_x() + session.shake_offset().x;
    let playfield_width = tile_width * session.rules.columns as f32;
    let field = Rect::new(playfield_x, HUD_HEIGHT, playfield_width, TARGET_Y - HUD_HEIGHT);

    ui::fill(field, role::PANEL);
    for column in 0..=session.rules.columns {
        let x = playfield_x + column as f32 * tile_width;
        draw_rectangle(x, field.y, 1.0, field.h, role::BORDER);
    }

    // Sous la ligne, tout est perdu : elle est dessinée avant les tuiles pour
    // qu'une tuile sur le point de mourir passe visiblement par-dessus.
    draw_rectangle(playfield_x, TARGET_Y, playfield_width, 1.0, role::DANGER);

    for tile in &session.tiles {
        let rect = Rect::new(
            playfield_x + tile.column as f32 * tile_width + 1.0,
            tile.y.floor(),
            tile_width - 2.0,
            TILE_HEIGHT,
        );

        let background = if tile.cleared.is_some() { role::SUCCESS } else { role::BORDER };
        ui::panel(rect, background);
        ui::glyph_fitted(
            app.fonts.script_variant(&session.language_id, tile.font),
            &tile.glyph.char,
            rect,
            GLYPH_SIZE,
            role::TEXT,
        );
    }

    // Le bandeau est dessiné après les tuiles : une tuile qui apparaît glisse
    // ainsi derrière lui, au lieu de surgir d'un bord net.
    ui::fill(Rect::new(0.0, 0.0, canvas::WIDTH, HUD_HEIGHT), role::BACKGROUND);
    draw_hud(&app.fonts, session);
    draw_input_bar(&app.fonts, session);
}

fn draw_hud(fonts_set: &Fonts, session: &Session) {
    ui::text(fonts_set, &format!("{:05}", session.score), 6.0, 6.0, fonts::TEXT, role::TEXT);
    ui::hearts_row(6.0, 18.0, session.lives, session.rules.lives);

    // Debout, le titre passe sous le score sur une ligne ; couché, la gouttière
    // est étroite et il s'y replie sur trois.
    if canvas::PORTRAIT {
        ui::text_truncated(
            fonts_set,
            &session.level_title,
            6.0,
            29.0,
            fonts::TEXT,
            role::TEXT_DISABLED,
            canvas::WIDTH - 12.0,
        );
    } else {
        let gutter = session.playfield_x() - 12.0;
        for (index, line) in ui::wrap(fonts_set, &session.level_title, fonts::TEXT, gutter)
            .iter()
            .take(3)
            .enumerate()
        {
            ui::text(
                fonts_set,
                line,
                6.0,
                34.0 + index as f32 * 10.0,
                fonts::TEXT,
                role::TEXT_DISABLED,
            );
        }
    }

    if session.rules.is_timed() {
        draw_timer(fonts_set, session);
    }
}

/// Le temps restant, en jauge et en chiffres, à droite du bandeau.
fn draw_timer(fonts_set: &Fonts, session: &Session) {
    let remaining = session.time_left.ceil() as u32;
    let label = format!("{remaining}S");
    let label_width = ui::text_width(fonts_set, &label, fonts::TEXT);
    ui::text(fonts_set, &label, canvas::WIDTH - 6.0 - label_width, 6.0, fonts::TEXT, role::TEXT);

    // La jauge vire au rouge sur la fin : le chiffre seul se remarque mal quand
    // on a les yeux sur les tuiles.
    let ratio = session.time_ratio();
    let color = if ratio < 0.2 { role::DANGER } else { role::ACCENT };
    // Couché, la jauge occupe la gouttière de droite, sous le chiffre.
    let gauge = if canvas::PORTRAIT {
        Rect::new(canvas::WIDTH - 76.0, 18.0, 70.0, 8.0)
    } else {
        let x = session.playfield_x() + session.rules.columns as f32 * session.tile_width() + 12.0;
        Rect::new(x, 18.0, canvas::WIDTH - x - 6.0, 8.0)
    };
    ui::progress_bar(gauge, ratio, color);
}

fn draw_input_bar(fonts_set: &Fonts, session: &Session) {
    ui::panel(INPUT_BAR, role::BORDER);

    let (content, color) = if session.input.is_empty() {
        ("tape la lecture", role::TEXT_DISABLED)
    } else {
        (session.input.as_str(), role::STAR)
    };
    ui::text_centered(
        fonts_set,
        content,
        INPUT_BAR.x + INPUT_BAR.w / 2.0,
        INPUT_BAR.y + (INPUT_BAR.h - fonts::TEXT as f32) / 2.0,
        fonts::TEXT,
        color,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_overlaps_between_the_line_and_the_keyboard() {
        // La barre de saisie se glisse entre les deux : si le clavier remontait,
        // elle passerait dessous et le joueur ne verrait plus ce qu'il tape.
        assert!(TARGET_Y < INPUT_BAR.y, "la ligne rouge est au-dessus de la saisie");
        if canvas::PORTRAIT {
            assert!(INPUT_BAR.y + INPUT_BAR.h <= keyboard::TOP, "la saisie finit avant le clavier");
        }
    }
}
