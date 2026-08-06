//! Le choix de l'écriture à apprendre.
//!
//! Chaque carte montre le nom de la langue dans sa propre écriture : c'est le
//! premier contact avec les glyphes, avant même d'avoir lancé un niveau.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::data::Language;
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{Fonts, canvas, fonts};
use crate::screens::learning_path::PathView;

const CARD_X: f32 = canvas::pick(12.0, 32.0);
const CARD_WIDTH: f32 = canvas::WIDTH - CARD_X * 2.0;
const CARD_HEIGHT: f32 = canvas::pick(38.0, 28.0);
const CARD_GAP: f32 = canvas::pick(8.0, 6.0);
const FIRST_CARD_Y: f32 = canvas::pick(60.0, 30.0);

/// Le pavé qui affiche le nom natif, à gauche de chaque carte.
const BADGE_WIDTH: f32 = 44.0;
const BADGE_HEIGHT: f32 = canvas::pick(26.0, 20.0);
const BADGE_FONT: u16 = 14;

pub fn language_select_screen(app: &App, selected: &mut usize, mouse: Vec2) -> Transition {
    clear_background(role::BACKGROUND);

    let languages = &app.catalog.languages;
    if languages.is_empty() {
        // `Catalog::validate` l'interdit, mais l'écran ne doit pas dépendre de
        // cette garantie pour ne pas indexer dans le vide.
        ui::text_centered(
            &app.fonts,
            "aucun alphabet disponible",
            canvas::WIDTH / 2.0,
            100.0,
            fonts::TEXT,
            role::TEXT_MUTED,
        );
        return Transition::Stay;
    }

    *selected = (*selected).min(languages.len() - 1);
    // La carte désignée en arrivant, pour repérer ensuite tout déplacement,
    // qu'il vienne des flèches ou de la souris.
    let before = *selected;

    ui::text_centered(
        &app.fonts,
        "CHOISIS TON ALPHABET",
        canvas::WIDTH / 2.0,
        canvas::pick(28.0, 10.0),
        fonts::TEXT,
        role::TITLE,
    );

    // Un téléphone n'a pas d'Échap : les écrans d'où l'on peut repartir portent
    // leur propre retour, en haut à gauche. Couché, la touche suffit et le coin
    // appartient au titre.
    if canvas::PORTRAIT
        && ui::button(
            &app.fonts,
            mouse,
            Button::new(Rect::new(6.0, 4.0, 26.0, 16.0), "<").accent(role::TEXT_MUTED),
        )
    {
        return Transition::Pop;
    }

    let mut chosen = None;

    for (index, language) in languages.iter().enumerate() {
        let card = card_rect(index);

        // Survoler déplace la sélection : le clavier et la souris désignent
        // ainsi toujours la même carte, y compris pour le texte du bas.
        if ui::hit(card, mouse) {
            *selected = index;
            if is_mouse_button_pressed(MouseButton::Left) {
                chosen = Some(language);
            }
        }

        draw_card(&app.fonts, language, card, index == *selected);
    }

    draw_description(&app.fonts, &languages[*selected]);

    ui::text_centered(
        &app.fonts,
        canvas::label("TOUCHE UN ALPHABET", "ENTREE VALIDER   ECHAP RETOUR"),
        canvas::WIDTH / 2.0,
        canvas::pick(360.0, 200.0),
        fonts::TEXT,
        role::TEXT_DISABLED,
    );

    if is_key_pressed(KeyCode::Down) {
        *selected = (*selected + 1) % languages.len();
    }
    if is_key_pressed(KeyCode::Up) {
        *selected = (*selected + languages.len() - 1) % languages.len();
    }

    // Le bruit suit la sélection, pas la touche : survoler une autre carte la
    // déplace tout autant qu'une flèche, et se taire alors donnerait
    // l'impression que la souris compte moins.
    if *selected != before {
        app.sfx.navigate();
    }
    if is_key_pressed(KeyCode::Enter) {
        chosen = Some(&languages[*selected]);
    }

    match chosen {
        Some(language) => {
            app.sfx.confirm();
            Transition::Push(Screen::LearningPath {
                language: language.id.clone(),
                view: PathView::new(),
            })
        }
        None => Transition::Stay,
    }
}

fn card_rect(index: usize) -> Rect {
    Rect::new(
        CARD_X,
        FIRST_CARD_Y + index as f32 * (CARD_HEIGHT + CARD_GAP),
        CARD_WIDTH,
        CARD_HEIGHT,
    )
}

fn draw_card(fonts_set: &Fonts, language: &Language, card: Rect, selected: bool) {
    let (background, title_color, detail_color) = if selected {
        (role::ACCENT, role::BORDER, role::BORDER)
    } else {
        (role::PANEL, role::TEXT, role::TEXT_MUTED)
    };

    ui::panel(card, background);

    let badge = Rect::new(card.x + 5.0, card.y + 6.0, BADGE_WIDTH, BADGE_HEIGHT);
    ui::fill(badge, role::BACKGROUND);
    ui::glyph_fitted(
        fonts_set.script(&language.id),
        &language.native_name,
        badge,
        BADGE_FONT,
        role::TEXT,
    );

    // Le nom passe sur deux lignes plutôt que d'être coupé : « Japonais —
    // Hiragana » tronqué perdrait justement le mot qui distingue une écriture
    // de sa voisine, et il ne reste que dix-sept caractères par ligne à côté du
    // pavé du nom natif.
    let text_x = badge.x + BADGE_WIDTH + 8.0;
    let text_width = card.x + card.w - text_x - 5.0;
    for (line, content) in
        ui::wrap(fonts_set, &language.name, fonts::TEXT, text_width).iter().take(2).enumerate()
    {
        ui::text(
            fonts_set,
            content,
            text_x,
            card.y + canvas::pick(5.0, 4.0) + line as f32 * 10.0,
            fonts::TEXT,
            title_color,
        );
    }

    let count = language.levels.len();
    let plural = if count > 1 { "NIVEAUX" } else { "NIVEAU" };
    ui::text(
        fonts_set,
        &format!("{count} {plural}"),
        text_x,
        card.y + canvas::pick(25.0, 16.0),
        fonts::TEXT,
        detail_color,
    );
}

/// La description de la langue survolée, en bas de l'écran.
fn draw_description(fonts_set: &Fonts, language: &Language) {
    const PANEL_Y: f32 = canvas::pick(300.0, 166.0);
    const PANEL_HEIGHT: f32 = canvas::pick(46.0, 26.0);
    const PADDING: f32 = 5.0;
    /// Au-delà, le texte déborderait du panneau : la description est tronquée
    /// plutôt que d'écrire par-dessus le reste de l'écran.
    const MAX_LINES: usize = if canvas::PORTRAIT { 4 } else { 2 };

    let panel = Rect::new(CARD_X, PANEL_Y, CARD_WIDTH, PANEL_HEIGHT);
    ui::panel(panel, role::PANEL);

    let lines = ui::wrap(fonts_set, &language.description, fonts::TEXT, CARD_WIDTH - PADDING * 2.0);
    for (index, line) in lines.iter().take(MAX_LINES).enumerate() {
        ui::text(
            fonts_set,
            line,
            panel.x + PADDING,
            panel.y + PADDING + index as f32 * 10.0,
            fonts::TEXT,
            role::TEXT_MUTED,
        );
    }
}
