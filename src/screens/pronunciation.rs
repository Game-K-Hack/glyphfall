//! Comment un signe se prononce, en détail.
//!
//! Un écran à lui seul, et non un pavé de plus sur la fiche : une consonne
//! coréenne ne se dit pas pareil en début de mot, entre deux voyelles et en fin
//! de syllabe, et les trois cas avec leurs équivalents français font une
//! vingtaine de lignes. La fiche en tient dix, moyens mnémotechniques compris.
//!
//! Le texte est repris tel quel des fichiers de langue, retours à la ligne et
//! retraits inclus : c'est son auteur qui le met en page, pas cet écran.

use macroquad::prelude::*;

use crate::app::{App, Transition};
use crate::data::{Glyph, Language, Level};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};

/// Le pavé du signe, en haut à gauche.
const CARD: Rect = Rect { x: 10.0, y: canvas::pick(30.0, 28.0), w: 54.0, h: 54.0 };

/// L'ordonnée de la rangée de boutons, la seule chose que le texte pourrait
/// recouvrir.
const NAVIGATION_Y: f32 = canvas::pick(356.0, 194.0);

/// Où commence le texte.
///
/// Debout, il passe sous le pavé et prend toute la largeur ; couché, il se met à
/// sa droite et commence aussi haut que lui — sinon les deux tiers de l'écran
/// resteraient vides pendant que le texte se ferait couper.
const TEXT_X: f32 = canvas::pick(10.0, CARD.x + CARD.w + 12.0);
const TEXT_TOP: f32 = canvas::pick(CARD.y + CARD.h + 12.0, CARD.y);

/// La zone du texte. `ui::block` s'y arrête : ce qui n'y tient pas n'est pas
/// écrit, d'où le test qui vérifie que tout y tient.
const TEXT: Rect = Rect {
    x: TEXT_X,
    y: TEXT_TOP,
    w: canvas::WIDTH - TEXT_X - 10.0,
    h: NAVIGATION_Y - 6.0 - TEXT_TOP,
};

pub fn pronunciation_screen(
    app: &App,
    language_id: &str,
    level_id: &str,
    index: &mut usize,
    swipe: &mut Option<f32>,
    mouse: Vec2,
) -> Transition {
    clear_background(role::BACKGROUND);

    let Some(language) = app.catalog.language(language_id) else { return Transition::Pop };
    let Some(level) = language.level(level_id) else { return Transition::Pop };
    if level.glyphs.is_empty() {
        return Transition::Pop;
    }

    *index = (*index).min(level.glyphs.len() - 1);
    let before = *index;

    // Comme sur la fiche : deux prononciations voisines se comparent, et
    // repasser par la fiche entre les deux ferait perdre le fil. Flèches,
    // glissement et boutons y mènent de la même façon.
    if is_key_pressed(KeyCode::Right) {
        *index = ui::turn(*index, 1, level.glyphs.len());
    }
    if is_key_pressed(KeyCode::Left) {
        *index = ui::turn(*index, -1, level.glyphs.len());
    }

    let travelled = ui::swipe(swipe, mouse);
    if travelled != 0 {
        *index = ui::turn(*index, travelled, level.glyphs.len());
    }

    let glyph = &level.glyphs[*index];

    draw_header(&app.fonts, level, *index);
    draw_sign(app, language, glyph);
    draw_pronunciation(&app.fonts, glyph);

    let transition = draw_navigation(app, level, index, mouse);

    // Le clic qui referme l'écran est déjà un début de glissement : sans cet
    // oubli, la fiche retrouverait le doigt au retour à l'autre bout de la
    // rangée et changerait de signe toute seule.
    if !matches!(transition, Transition::Stay) {
        *swipe = None;
    }

    if *index != before {
        app.sfx.navigate();
    }

    transition
}

fn draw_header(fonts_set: &Fonts, level: &Level, index: usize) {
    ui::text(fonts_set, "PRONONCIATION", 8.0, 8.0, fonts::TEXT, role::TITLE);

    let position = format!("{} / {}", index + 1, level.glyphs.len());
    let width = ui::text_width(fonts_set, &position, fonts::TEXT);
    ui::text(fonts_set, &position, canvas::WIDTH - 8.0 - width, 8.0, fonts::TEXT, role::TEXT_MUTED);

    draw_rectangle(8.0, 22.0, canvas::WIDTH - 16.0, 1.0, role::HIGHLIGHT);
}

/// Le signe, sa lecture et son nom : de quoi savoir de qui l'on parle sans
/// revenir en arrière.
fn draw_sign(app: &App, language: &Language, glyph: &Glyph) {
    ui::panel(CARD, role::PANEL);
    ui::glyph_fitted(app.fonts.script(&language.id), &glyph.char, CARD, 40, role::TEXT);

    // Debout la lecture se met à côté du pavé, couché dessous : c'est de ce
    // côté-là qu'il reste de la place une fois le texte installé.
    let (x, y) = if canvas::PORTRAIT {
        (CARD.x + CARD.w + 12.0, CARD.y + 4.0)
    } else {
        (CARD.x, CARD.y + CARD.h + 6.0)
    };

    ui::text(&app.fonts, glyph.primary_answer(), x, y, fonts::TITLE, role::ACCENT);

    if !glyph.name.is_empty() {
        ui::text(&app.fonts, &glyph.name, x, y + 22.0, fonts::TEXT, role::TEXT_MUTED);
    }
}

fn draw_pronunciation(fonts_set: &Fonts, glyph: &Glyph) {
    if glyph.pronunciation.is_empty() {
        ui::text(fonts_set, "Pas encore decrite.", TEXT.x, TEXT.y, fonts::TEXT, role::TEXT_DISABLED);
        return;
    }

    ui::block(fonts_set, &glyph.pronunciation, TEXT, fonts::TEXT, role::TEXT);
}

fn draw_navigation(app: &App, level: &Level, index: &mut usize, mouse: Vec2) -> Transition {
    const Y: f32 = NAVIGATION_Y;
    const ARROW: f32 = canvas::pick(24.0, 22.0);
    const HEIGHT: f32 = canvas::pick(20.0, 16.0);

    let previous = Rect::new(10.0, Y, ARROW, HEIGHT);
    if ui::button(&app.fonts, mouse, Button::new(previous, "<")) {
        *index = ui::turn(*index, -1, level.glyphs.len());
    }

    let next = Rect::new(10.0 + ARROW + 4.0, Y, ARROW, HEIGHT);
    if ui::button(&app.fonts, mouse, Button::new(next, ">")) {
        *index = ui::turn(*index, 1, level.glyphs.len());
    }

    let back = Rect::new(canvas::WIDTH - 10.0 - 76.0, Y, 76.0, HEIGHT);
    if ui::button(&app.fonts, mouse, Button::new(back, "RETOUR").accent(role::TEXT_MUTED)) {
        return Transition::Pop;
    }

    Transition::Stay
}
