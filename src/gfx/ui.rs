//! Les briques d'interface 8-bit : panneaux, boutons, texte, étoiles, coeurs.
//!
//! Toutes les coordonnées sont en pixels virtuels et doivent tomber sur des
//! entiers. Un panneau posé en x = 12.5 serait rendu à cheval sur deux pixels
//! et trahirait immédiatement l'illusion.

use macroquad::prelude::*;

use super::fonts::Fonts;
use super::palette::role;

// --- Primitives ------------------------------------------------------------

/// Un rectangle plein aux bords nets.
pub fn fill(rect: Rect, color: Color) {
    draw_rectangle(rect.x.floor(), rect.y.floor(), rect.w.floor(), rect.h.floor(), color);
}

/// Un contour de 1 pixel exactement.
///
/// `draw_rectangle_lines` centre son trait sur la bordure et déborde d'un
/// demi-pixel : on dessine donc les quatre côtés à la main.
pub fn stroke(rect: Rect, color: Color) {
    let (x, y, w, h) = (rect.x.floor(), rect.y.floor(), rect.w.floor(), rect.h.floor());
    draw_rectangle(x, y, w, 1.0, color);
    draw_rectangle(x, y + h - 1.0, w, 1.0, color);
    draw_rectangle(x, y, 1.0, h, color);
    draw_rectangle(x + w - 1.0, y, 1.0, h, color);
}

/// Dessine une image faite de pixels : `#` allumé, tout le reste éteint.
pub fn blit(pattern: &[&str], x: f32, y: f32, color: Color) {
    blit_scaled(pattern, x, y, 1.0, color);
}

/// Comme `blit`, mais chaque pixel du motif devient un carré de `scale` côtés.
///
/// Agrandir le motif plutôt que d'étirer une image garde les bords parfaitement
/// nets, quel que soit le facteur.
pub fn blit_scaled(pattern: &[&str], x: f32, y: f32, scale: f32, color: Color) {
    let scale = scale.max(1.0).floor();

    for (row, line) in pattern.iter().enumerate() {
        for (column, cell) in line.chars().enumerate() {
            if cell == '#' {
                draw_rectangle(
                    x.floor() + column as f32 * scale,
                    y.floor() + row as f32 * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

/// Le curseur est-il sur ce rectangle ?
pub fn hit(rect: Rect, mouse: Vec2) -> bool {
    rect.contains(mouse)
}

// --- Texte -----------------------------------------------------------------

/// Écrit à partir du coin **haut-gauche**, contrairement à `draw_text_ex` qui
/// part de la ligne de base : raisonner en boîtes est plus simple pour poser
/// une interface au pixel.
pub fn text(fonts: &Fonts, content: &str, x: f32, y: f32, size: u16, color: Color) {
    draw_text_ex(
        content,
        x.floor(),
        y.floor() + size as f32,
        TextParams { font: Some(&fonts.ui), font_size: size, color, ..Default::default() },
    );
}

pub fn text_width(fonts: &Fonts, content: &str, size: u16) -> f32 {
    measure_text(content, Some(&fonts.ui), size, 1.0).width
}

/// Écrit sur une ligne, en coupant avec des points de suspension ce qui
/// dépasserait `max_width`.
///
/// Les titres viennent de fichiers TOML écrits à la main : mieux vaut un texte
/// tronqué proprement qu'un texte qui sort de l'écran.
pub fn text_truncated(
    fonts: &Fonts,
    content: &str,
    x: f32,
    y: f32,
    size: u16,
    color: Color,
    max_width: f32,
) {
    const ELLIPSIS: &str = "...";

    if text_width(fonts, content, size) <= max_width {
        text(fonts, content, x, y, size, color);
        return;
    }

    let mut shortened = content.to_string();
    while !shortened.is_empty()
        && text_width(fonts, &format!("{shortened}{ELLIPSIS}"), size) > max_width
    {
        shortened.pop();
    }
    shortened.push_str(ELLIPSIS);

    text(fonts, &shortened, x, y, size, color);
}

/// Écrit centré horizontalement sur `center_x`.
pub fn text_centered(fonts: &Fonts, content: &str, center_x: f32, y: f32, size: u16, color: Color) {
    let x = center_x - text_width(fonts, content, size) / 2.0;
    text(fonts, content, x.round(), y, size, color);
}

/// Écrit un glyphe de la langue étudiée, centré dans `rect`.
///
/// Les écritures CJK ont des métriques très différentes de la police pixel :
/// on centre donc sur les dimensions mesurées plutôt qu'à l'estime.
pub fn glyph_centered(font: &Font, content: &str, rect: Rect, size: u16, color: Color) {
    let measured = measure_text(content, Some(font), size, 1.0);
    let x = rect.x + (rect.w - measured.width) / 2.0;
    let y = rect.y + (rect.h + measured.offset_y) / 2.0;

    draw_text_ex(
        content,
        x.round(),
        y.round(),
        TextParams { font: Some(font), font_size: size, color, ..Default::default() },
    );
}

/// Comme `glyph_centered`, mais réduit la taille jusqu'à ce que le texte tienne
/// dans `rect`.
///
/// Les noms natifs ne font pas tous la même longueur — « 漢字 » tient à l'aise
/// là où « ひらがな » déborde — et une taille fixe en laisserait fuir certains
/// hors de leur pavé.
pub fn glyph_fitted(font: &Font, content: &str, rect: Rect, max_size: u16, color: Color) {
    const MIN_SIZE: u16 = 7;
    const PADDING: f32 = 2.0;

    let mut size = max_size;
    while size > MIN_SIZE && measure_text(content, Some(font), size, 1.0).width > rect.w - PADDING {
        size -= 1;
    }

    glyph_centered(font, content, rect, size, color);
}

/// Découpe un texte pour qu'aucune ligne ne dépasse `max_width`.
pub fn wrap(fonts: &Fonts, content: &str, size: u16, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();

    for word in content.split_whitespace() {
        let candidate =
            if current.is_empty() { word.to_string() } else { format!("{current} {word}") };

        if text_width(fonts, &candidate, size) <= max_width || current.is_empty() {
            current = candidate;
        } else {
            lines.push(std::mem::take(&mut current));
            current = word.to_string();
        }
    }

    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

// --- Panneaux et boutons ---------------------------------------------------

/// Un panneau : fond, contour sombre, et arête claire en haut à gauche qui
/// donne le relief caractéristique des interfaces 8-bit.
pub fn panel(rect: Rect, background: Color) {
    fill(rect, background);
    stroke(rect, role::BORDER);
    draw_rectangle(rect.x + 1.0, rect.y + 1.0, rect.w - 2.0, 1.0, role::HIGHLIGHT);
    draw_rectangle(rect.x + 1.0, rect.y + 1.0, 1.0, rect.h - 2.0, role::HIGHLIGHT);
}

pub struct Button<'a> {
    pub rect: Rect,
    pub label: &'a str,
    /// Un bouton désactivé s'affiche en grisé et ne peut pas être cliqué.
    pub enabled: bool,
    /// Mis en avant par la navigation au clavier.
    pub focused: bool,
    pub accent: Color,
}

impl<'a> Button<'a> {
    pub fn new(rect: Rect, label: &'a str) -> Self {
        Self { rect, label, enabled: true, focused: false, accent: role::ACCENT }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn accent(mut self, accent: Color) -> Self {
        self.accent = accent;
        self
    }
}

/// Dessine le bouton et indique s'il vient d'être cliqué.
///
/// L'activation au clavier reste à la charge de l'écran, qui seul sait quel
/// élément a le focus.
pub fn button(fonts: &Fonts, mouse: Vec2, button: Button) -> bool {
    let hovered = button.enabled && hit(button.rect, mouse);
    let highlighted = hovered || (button.enabled && button.focused);

    let (background, label_color) = match (button.enabled, highlighted) {
        (false, _) => (role::PANEL, role::TEXT_DISABLED),
        (true, true) => (button.accent, role::BORDER),
        (true, false) => (role::PANEL, role::TEXT),
    };

    panel(button.rect, background);
    text_centered(
        fonts,
        button.label,
        button.rect.x + button.rect.w / 2.0,
        button.rect.y + (button.rect.h - super::fonts::TEXT as f32) / 2.0,
        super::fonts::TEXT,
        label_color,
    );

    hovered && is_mouse_button_pressed(MouseButton::Left)
}

/// Une jauge horizontale remplie à `ratio` (borné entre 0 et 1).
pub fn progress_bar(rect: Rect, ratio: f32, color: Color) {
    panel(rect, role::PANEL);

    let inner = Rect::new(rect.x + 2.0, rect.y + 2.0, rect.w - 4.0, rect.h - 4.0);
    let filled = (inner.w * ratio.clamp(0.0, 1.0)).floor();
    if filled >= 1.0 {
        draw_rectangle(inner.x, inner.y, filled, inner.h, color);
    }
}

// --- Icônes ----------------------------------------------------------------

const STAR: [&str; 7] = [
    "   #   ",
    "   #   ",
    "#######",
    " ##### ",
    "  ###  ",
    " ## ## ",
    "##   ##",
];

const HEART: [&str; 6] = [
    " ## ## ",
    "#######",
    "#######",
    " ##### ",
    "  ###  ",
    "   #   ",
];

const LOCK: [&str; 7] = [
    "  ###  ",
    " #   # ",
    " #   # ",
    "#######",
    "### ###",
    "### ###",
    "#######",
];

pub const STAR_WIDTH: f32 = 7.0;
pub const STAR_HEIGHT: f32 = 7.0;
pub const HEART_WIDTH: f32 = 7.0;

pub fn star(x: f32, y: f32, earned: bool) {
    star_scaled(x, y, 1.0, earned);
}

pub fn star_scaled(x: f32, y: f32, scale: f32, earned: bool) {
    blit_scaled(&STAR, x, y, scale, if earned { role::STAR } else { role::STAR_EMPTY });
}

pub fn heart(x: f32, y: f32, remaining: bool) {
    blit(&HEART, x, y, if remaining { role::DANGER } else { role::TEXT_DISABLED });
}

pub fn lock(x: f32, y: f32, color: Color) {
    blit(&LOCK, x, y, color);
}

/// Un trait vertical pointillé, qui relie deux étapes du chemin.
pub fn dotted_line(x: f32, from_y: f32, to_y: f32, color: Color) {
    let mut y = from_y.floor();
    while y < to_y {
        draw_rectangle(x.floor(), y, 1.0, 1.0, color);
        y += 3.0;
    }
}

/// Trois étoiles alignées, dont `earned` sont gagnées.
pub fn stars_row(x: f32, y: f32, earned: u8, total: u8) {
    const GAP: f32 = 2.0;
    for index in 0..total {
        star(x + index as f32 * (STAR_WIDTH + GAP), y, index < earned);
    }
}

pub fn stars_row_width(total: u8) -> f32 {
    total as f32 * STAR_WIDTH + (total.saturating_sub(1)) as f32 * 2.0
}

/// Les vies restantes, coeurs pleins puis éteints.
pub fn hearts_row(x: f32, y: f32, remaining: u32, total: u32) {
    const GAP: f32 = 2.0;
    for index in 0..total {
        heart(x + index as f32 * (HEART_WIDTH + GAP), y, index < remaining);
    }
}
