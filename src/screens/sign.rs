//! La fiche d'un signe : à quoi il ressemble, comment il se lit, et comment le
//! retenir.
//!
//! C'est le seul écran où l'on peut s'arrêter sur un caractère sans qu'il
//! tombe. Le briefing en montre trente d'un coup ; ici il n'y en a qu'un, en
//! grand, avec tout ce qui aide à l'ancrer.

use macroquad::prelude::*;

use crate::app::{App, Transition};
use crate::data::{Glyph, Language, Level};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};

/// Le pavé qui porte le signe : centré en haut debout, à gauche couché.
const CARD: Rect = Rect { x: canvas::pick(60.0, 16.0), y: canvas::pick(32.0, 34.0), w: 96.0, h: 96.0 };
/// Taille du signe dans son pavé.
const GLYPH_SIZE: u16 = 64;

/// La bande des tracés : sous les lectures debout, sous les deux colonnes
/// couché.
const TRACINGS_Y: f32 = canvas::pick(190.0, 166.0);

/// La colonne de texte.
///
/// En portrait la fiche s'empile au lieu de se lire en deux colonnes, qui
/// feraient huit caractères chacune : le pavé et les lectures en haut, les
/// moyens de retenir dessous, sur toute la largeur.
const TEXT_X: f32 = canvas::pick(10.0, CARD.x + CARD.w + 14.0);
const TEXT_WIDTH: f32 = canvas::WIDTH - TEXT_X - canvas::pick(10.0, 16.0);

/// Distance à parcourir du doigt pour changer de signe, en pixels virtuels.
///
/// Un quart de la largeur : plus court, un appui un peu traînant ferait changer
/// de fiche ; plus long, le geste ne tiendrait pas sur un petit écran.
const SWIPE: f32 = canvas::WIDTH / 4.0;

pub fn sign_screen(
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

    // Les flèches passent d'un signe à l'autre sans repasser par le briefing :
    // on lit rarement une seule fiche.
    if is_key_pressed(KeyCode::Right) {
        *index = (*index + 1) % level.glyphs.len();
    }
    if is_key_pressed(KeyCode::Left) {
        *index = (*index + level.glyphs.len() - 1) % level.glyphs.len();
    }

    // Le glissement horizontal feuillette les fiches, comme on tourne une page.
    // Seul le point de départ est retenu : suivre le doigt en continu ferait
    // défiler plusieurs signes d'un seul geste un peu long.
    if is_mouse_button_pressed(MouseButton::Left) {
        *swipe = Some(mouse.x);
    }
    if let Some(start) = *swipe {
        let travelled = mouse.x - start;

        if travelled.abs() >= SWIPE {
            *index = if travelled < 0.0 {
                (*index + 1) % level.glyphs.len()
            } else {
                (*index + level.glyphs.len() - 1) % level.glyphs.len()
            };
            // Le geste est consommé : on repart de la position courante, ce qui
            // permet d'enchaîner sans lever le doigt.
            *swipe = Some(mouse.x);
        }
    }
    if is_mouse_button_released(MouseButton::Left) {
        *swipe = None;
    }

    let glyph = &level.glyphs[*index];

    draw_header(&app.fonts, level, *index);
    draw_card(app, language, glyph);
    draw_readings(&app.fonts, glyph);
    draw_mnemonics(&app.fonts, glyph);

    let transition = draw_navigation(app, level, index, mouse);

    // Flèches du clavier ou boutons de l'écran : changer de fiche est un
    // déplacement, et s'entend comme tel.
    if *index != before {
        app.sfx.navigate();
    }

    transition
}

fn draw_header(fonts_set: &Fonts, level: &Level, index: usize) {
    ui::text_truncated(
        fonts_set,
        &level.title,
        8.0,
        8.0,
        fonts::TEXT,
        role::TITLE,
        canvas::WIDTH - canvas::pick(62.0, 90.0),
    );

    let position = format!("{} / {}", index + 1, level.glyphs.len());
    let width = ui::text_width(fonts_set, &position, fonts::TEXT);
    ui::text(fonts_set, &position, canvas::WIDTH - 8.0 - width, 8.0, fonts::TEXT, role::TEXT_MUTED);

    draw_rectangle(8.0, 22.0, canvas::WIDTH - 16.0, 1.0, role::HIGHLIGHT);
}

/// Le signe en grand, et l'état de sa maîtrise.
fn draw_card(app: &App, language: &Language, glyph: &Glyph) {
    ui::panel(CARD, role::PANEL);
    ui::glyph_fitted(
        app.fonts.script(&language.id),
        &glyph.char,
        CARD,
        GLYPH_SIZE,
        role::TEXT,
    );

    draw_tracings(app, language, glyph);

    // Un rappel de ce que le jeu sait du joueur sur ce signe précis : c'est ce
    // qui distingue une fiche d'un simple dictionnaire.
    let (label, color) = match app.progress.mastery(&language.id, &glyph.char) {
        score if score < 0 => ("A CONSOLIDER", role::SHAKY),
        0 => ("PAS ENCORE VU", role::TEXT_DISABLED),
        score if score >= 3 => ("ACQUIS", role::SUCCESS),
        _ => ("EN COURS", role::TEXT_MUTED),
    };
    ui::text_centered(
        &app.fonts,
        label,
        CARD.x + CARD.w / 2.0,
        CARD.y + CARD.h + 6.0,
        fonts::TEXT,
        color,
    );
}

/// Le même signe dans les autres tracés disponibles.
///
/// C'est ici que la fiche gagne son nom : reconnaître un signe dans une seule
/// police, c'est reconnaître une image, pas un caractère. Les variantes sont
/// montrées même quand le joueur a refusé de les voir tomber — il a le temps de
/// les comparer, ce qui est justement ce que la manche ne permet pas.
fn draw_tracings(app: &App, language: &Language, glyph: &Glyph) {
    const Y: f32 = TRACINGS_Y;
    const SIZE: f32 = canvas::pick(24.0, 22.0);
    const GAP: f32 = 3.0;

    let count = app.fonts.script_count(&language.id);
    if count < 2 {
        return;
    }

    // La bande est centrée sous le pavé : sept cases y tiennent tout juste, et
    // les rétrécir davantage les rendrait illisibles — or c'est précisément
    // leur lisibilité que la fiche démontre.
    //
    // Le tracé de référence figure dans la rangée avec les autres : une case
    // isolée ne dirait pas ce qu'elle montre, alors qu'une rangée du même signe
    // se lit d'elle-même comme une comparaison.
    // Couché, la bande laisse sa gauche à l'intitulé ; debout elle est centrée.
    let total = SIZE * count as f32 + GAP * (count - 1) as f32;
    let start_x = if canvas::PORTRAIT {
        ((canvas::WIDTH - total) / 2.0).floor()
    } else {
        ui::text(&app.fonts, "TRACES", 16.0, Y + 8.0, fonts::TEXT, role::TEXT_DISABLED);
        100.0
    };

    for variant in 0..count {
        let cell = Rect::new(start_x + variant as f32 * (SIZE + GAP), Y, SIZE, SIZE);
        ui::fill(cell, role::PANEL);
        ui::glyph_fitted(
            app.fonts.script_variant(&language.id, variant),
            &glyph.char,
            cell,
            18,
            role::TEXT_MUTED,
        );
    }
}

/// Les romanisations acceptées, la principale en évidence.
fn draw_readings(fonts_set: &Fonts, glyph: &Glyph) {
    // Debout, les lectures sont centrées sous le pavé ; couché elles ouvrent la
    // colonne de droite.
    if canvas::PORTRAIT {
        ui::text_centered(
            fonts_set,
            glyph.primary_answer(),
            canvas::WIDTH / 2.0,
            148.0,
            fonts::TITLE,
            role::ACCENT,
        );
    } else {
        ui::text(fonts_set, glyph.primary_answer(), TEXT_X, 34.0, fonts::TITLE, role::ACCENT);
    }

    if glyph.answers.len() > 1 {
        // Les variantes tolérées : les taire ferait croire à une seule bonne
        // réponse, alors que le jeu en accepte plusieurs.
        let others = glyph.answers[1..].join("  ");
        let label = format!("aussi : {others}");

        if canvas::PORTRAIT {
            ui::text_centered(
                fonts_set,
                &label,
                canvas::WIDTH / 2.0,
                170.0,
                fonts::TEXT,
                role::TEXT_MUTED,
            );
        } else {
            ui::text_truncated(
                fonts_set,
                &label,
                TEXT_X,
                54.0,
                fonts::TEXT,
                role::TEXT_MUTED,
                TEXT_WIDTH,
            );
        }
    }
}

fn draw_mnemonics(fonts_set: &Fonts, glyph: &Glyph) {
    const TOP: f32 = canvas::pick(226.0, 72.0);
    const LINE: f32 = 10.0;
    /// Au-delà, la fiche déborderait sur les boutons du bas. Le portrait laisse
    /// moins de largeur mais plus de hauteur : dix lignes de vingt-cinq
    /// caractères, contre sept de vingt-neuf auparavant.
    const MAX_LINES: usize = if canvas::PORTRAIT { 10 } else { 7 };

    ui::text(fonts_set, "POUR LE RETENIR", TEXT_X, TOP, fonts::TEXT, role::TEXT_MUTED);

    let mut y = TOP + 14.0;
    let mut written = 0;

    for mnemonic in &glyph.mnemonics {
        // Le tiret n'est mis qu'à la première ligne : les suivantes s'alignent
        // sous le texte, pour que chaque moyen se lise comme un bloc.
        for (line_number, line) in
            ui::wrap(fonts_set, mnemonic, fonts::TEXT, TEXT_WIDTH - 8.0).iter().enumerate()
        {
            if written >= MAX_LINES {
                return;
            }
            if line_number == 0 {
                ui::text(fonts_set, "-", TEXT_X, y, fonts::TEXT, role::HINT);
            }
            ui::text(fonts_set, line, TEXT_X + 8.0, y, fonts::TEXT, role::TEXT);

            y += LINE;
            written += 1;
        }
        y += 4.0;
    }
}

fn draw_navigation(app: &App, level: &Level, index: &mut usize, mouse: Vec2) -> Transition {
    const Y: f32 = canvas::pick(356.0, 194.0);
    const ARROW: f32 = canvas::pick(30.0, 22.0);
    const HEIGHT: f32 = canvas::pick(20.0, 16.0);

    let previous = Rect::new(10.0, Y, ARROW, HEIGHT);
    if ui::button(&app.fonts, mouse, Button::new(previous, "<")) {
        *index = (*index + level.glyphs.len() - 1) % level.glyphs.len();
    }

    let next = Rect::new(10.0 + ARROW + 4.0, Y, ARROW, HEIGHT);
    if ui::button(&app.fonts, mouse, Button::new(next, ">")) {
        *index = (*index + 1) % level.glyphs.len();
    }

    let back = Rect::new(canvas::WIDTH - 10.0 - 76.0, Y, 76.0, HEIGHT);
    if ui::button(&app.fonts, mouse, Button::new(back, "RETOUR").accent(role::TEXT_MUTED)) {
        return Transition::Pop;
    }

    // Centré entre les flèches et le bouton, et non sur la toile : centré sur
    // la toile, le rappel passait sous le bouton de retour.
    ui::text_centered(
        &app.fonts,
        canvas::label("GLISSE", "FLECHES POUR CHANGER"),
        (next.x + next.w + back.x) / 2.0,
        Y + canvas::pick(6.0, 4.0),
        fonts::TEXT,
        role::TEXT_DISABLED,
    );

    Transition::Stay
}
