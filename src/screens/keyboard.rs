//! Le clavier dessiné dans le jeu, pour jouer au doigt.
//!
//! Un téléphone n'a pas de clavier, et Glyphfall se joue en tapant la lecture
//! d'un signe : sans ces trois rangées, la moitié du jeu est inaccessible.
//!
//! Celui du système aurait été gratuit, mais il couvre la moitié de l'écran,
//! ne ressemble en rien au reste, et sa hauteur dépend de celui qu'a installé
//! le joueur — impossible de placer quoi que ce soit en dessous.
//!
//! Les touches ne passent pas par `ui::button` : celui-ci signale ce qu'il
//! survole pour le bruit de déplacement, et un doigt qui balaie vingt-huit
//! touches ferait une rafale. Ici seul l'appui compte.

use macroquad::prelude::*;

use crate::gfx::palette::role;
use crate::gfx::ui;
use crate::gfx::{Fonts, canvas, fonts};

/// Le haut du clavier. Ce qui tombe doit s'arrêter au-dessus.
pub const TOP: f32 = 276.0;

/// Hauteur de la rangée des chiffres, plus basse que celle des lettres : on y
/// tape moins souvent, et la place gagnée revient à la zone de chute.
const DIGIT_HEIGHT: f32 = 20.0;
const KEY_HEIGHT: f32 = 26.0;
const GAP: f32 = 2.0;
/// Largeur d'une touche. Dix par rangée, marges comprises.
const KEY_WIDTH: f32 = 19.0;
const MARGIN: f32 = 4.0;

/// Les chiffres, sur leur propre rangée.
///
/// Les kanji des nombres se répondent par « 1 » ou « 10 » : sans eux, quatre
/// niveaux du chemin japonais restaient injouables au doigt.
const DIGITS: &str = "1234567890";

/// Disposition française : le joueur retrouve sur le verre les lettres qu'il a
/// sous les doigts ailleurs.
const ROWS: [&str; 3] = ["azertyuiop", "qsdfghjklm", "wxcvbn"];

/// Ce que le joueur vient de demander.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Letter(char),
    /// Effacer la dernière lettre.
    Erase,
    /// Valider la saisie.
    Submit,
}

/// Dessine le clavier et renvoie la touche appuyée.
pub fn draw(fonts_set: &Fonts, mouse: Vec2) -> Option<Key> {
    let pressed = is_mouse_button_pressed(MouseButton::Left);
    let mut hit = None;

    for digit in DIGITS.chars() {
        let column = DIGITS.chars().position(|other| other == digit).unwrap_or(0);
        let key = Rect::new(
            MARGIN + column as f32 * (KEY_WIDTH + GAP),
            TOP,
            KEY_WIDTH,
            DIGIT_HEIGHT,
        );
        let touched = ui::hit(key, mouse);

        draw_key(fonts_set, key, &digit.to_string(), touched, role::PANEL);
        if touched && pressed {
            hit = Some(Key::Letter(digit));
        }
    }

    for (index, row) in ROWS.iter().enumerate() {
        let y = letters_top() + index as f32 * (KEY_HEIGHT + GAP);
        // Toutes les rangées partent de la même marge : centrer les plus
        // courtes décalerait leurs lettres de celles du dessus, alors que
        // l'oeil cherche une lettre à la place qu'il lui connaît.
        let mut x = MARGIN;

        for letter in row.chars() {
            let key = Rect::new(x, y, KEY_WIDTH, KEY_HEIGHT);
            let touched = ui::hit(key, mouse);

            draw_key(fonts_set, key, &letter.to_uppercase().to_string(), touched, role::PANEL);
            if touched && pressed {
                hit = Some(Key::Letter(letter));
            }

            x += KEY_WIDTH + GAP;
        }
    }

    // La troisième rangée n'a que six lettres : la place gagnée revient aux deux
    // touches que l'on cherche dans l'urgence, quand une tuile arrive.
    let y = letters_top() + 2.0 * (KEY_HEIGHT + GAP);
    let letters_width = 6.0 * KEY_WIDTH + 5.0 * GAP;
    let x = MARGIN + letters_width + GAP;

    let erase = Rect::new(x, y, 32.0, KEY_HEIGHT);
    let touched = ui::hit(erase, mouse);
    draw_key(fonts_set, erase, "<-", touched, role::BORDER);
    if touched && pressed {
        hit = Some(Key::Erase);
    }

    let submit = Rect::new(x + erase.w + GAP, y, canvas::WIDTH - MARGIN - x - erase.w - GAP, KEY_HEIGHT);
    let touched = ui::hit(submit, mouse);
    draw_key(fonts_set, submit, "OK", touched, role::SUCCESS);
    if touched && pressed {
        hit = Some(Key::Submit);
    }

    hit
}

/// Ordonnée de la première rangée de lettres, sous les chiffres.
fn letters_top() -> f32 {
    TOP + DIGIT_HEIGHT + GAP
}

fn draw_key(fonts_set: &Fonts, key: Rect, label: &str, touched: bool, accent: Color) {
    let background = if touched { role::ACCENT } else { accent };
    ui::panel(key, background);

    ui::text_centered(
        fonts_set,
        label,
        key.x + key.w / 2.0,
        key.y + (key.h - fonts::TEXT as f32) / 2.0,
        fonts::TEXT,
        if touched { role::BORDER } else { role::TEXT },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keyboard_stays_inside_the_canvas() {
        // Le clavier n'existe qu'en portrait : couché, le jeu se joue au
        // clavier physique et l'écran n'a pas la hauteur de l'accueillir.
        if !canvas::PORTRAIT {
            return;
        }

        // Une touche hors de la toile serait invisible et injouable.
        let widest = ROWS[0].chars().count() as f32;
        let width = widest * KEY_WIDTH + (widest - 1.0) * GAP;

        assert!(MARGIN + width <= canvas::WIDTH, "largeur : {width}");
        assert_eq!(DIGITS.chars().count(), ROWS[0].chars().count(), "rangées alignées");

        let bottom = letters_top() + 3.0 * KEY_HEIGHT + 2.0 * GAP;
        assert!(bottom <= canvas::HEIGHT, "bas du clavier : {bottom}");
    }

    #[test]
    fn every_digit_can_be_typed() {
        // Les kanji des nombres se repondent par un chiffre : sans eux, quatre
        // niveaux du chemin japonais restent injouables au doigt.
        for digit in '0'..='9' {
            assert!(DIGITS.contains(digit), "« {digit} » n'est sur aucune rangée");
        }
    }

    #[test]
    fn every_letter_of_the_alphabet_can_be_typed() {
        // Une lettre manquante rendrait certaines lectures impossibles à saisir,
        // et le niveau correspondant infranchissable au doigt.
        let typed: String = ROWS.concat();

        for letter in 'a'..='z' {
            assert!(typed.contains(letter), "« {letter} » n'est sur aucune rangée");
        }
    }
}
