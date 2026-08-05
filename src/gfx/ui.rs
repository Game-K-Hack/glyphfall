//! Les briques d'interface 8-bit : panneaux, boutons, texte, étoiles, coeurs.
//!
//! Toutes les coordonnées sont en pixels virtuels et doivent tomber sur des
//! entiers. Un panneau posé en x = 12.5 serait rendu à cheval sur deux pixels
//! et trahirait immédiatement l'illusion.

use std::cell::Cell;

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

// --- Survol ----------------------------------------------------------------
//
// Le bruit de déplacement ne peut pas rester l'affaire des flèches : passer la
// souris d'un bouton à l'autre est un déplacement comme un autre, et le
// silence donnait l'impression que la souris était moins bien traitée que le
// clavier.
//
// Les éléments signalent ce qu'ils voient pendant le rendu, et la boucle
// principale compare d'une frame à l'autre — elle seule a le son sous la main.

/// Aucun élément sous le curseur.
const NOTHING: u64 = 0;

thread_local! {
    /// Ce que le curseur survole pendant la frame en cours.
    static HOVERED: Cell<u64> = const { Cell::new(NOTHING) };
    /// Ce qu'il survolait à la frame précédente.
    static PREVIOUS: Cell<u64> = const { Cell::new(NOTHING) };
    /// La prochaine comparaison doit-elle être passée sous silence ?
    static MUTED: Cell<bool> = const { Cell::new(false) };
}

/// Signale que l'élément `id` est sous le curseur.
///
/// Le dernier à parler l'emporte : deux éléments superposés désignent le plus
/// petit, dessiné en dernier, qui est aussi celui que le clic atteindra.
pub fn focus(id: u64) {
    HOVERED.with(|hovered| hovered.set(id));
}

/// Un identifiant d'élément, tiré de son intitulé et de sa taille.
///
/// Volontairement indépendant de sa **position** : dans une liste qui défile,
/// une identité géométrique changerait à chaque frame et ferait crépiter le son
/// pendant tout le défilement.
pub fn widget_id(label: &str, rect: Rect) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;

    for byte in
        label.bytes().chain(rect.w.to_bits().to_be_bytes()).chain(rect.h.to_bits().to_be_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    // `NOTHING` doit rester réservé, sans quoi un élément deviendrait muet.
    hash | 1
}

/// Le curseur vient-il d'entrer dans un nouvel élément ?
///
/// À appeler une fois par frame, après le rendu : l'appel consomme ce qui a été
/// signalé. En sortir pour aller sur du vide ne compte pas — c'est l'arrivée
/// sur quelque chose de cliquable qui mérite d'être entendue.
pub fn focus_moved() -> bool {
    let hovered = HOVERED.with(Cell::take);
    let previous = PREVIOUS.with(|previous| previous.replace(hovered));

    // Le premier relevé d'un écran ne dit rien d'un déplacement : il constate
    // seulement où se trouve déjà le curseur.
    if MUTED.with(|muted| muted.replace(false)) {
        return false;
    }

    hovered != NOTHING && hovered != previous
}

/// Oublie le survol : à faire en changeant d'écran, faute de quoi le bouton se
/// trouvant sous le curseur à l'arrivée claquerait sans qu'on ait rien fait.
pub fn forget_focus() {
    HOVERED.with(|hovered| hovered.set(NOTHING));
    PREVIOUS.with(|previous| previous.set(NOTHING));
    MUTED.with(|muted| muted.set(true));
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
    /// Mis en avant par la navigation au clavier, ou parce que c'est l'action
    /// attendue de l'écran.
    pub focused: bool,
    pub accent: Color,
}

impl<'a> Button<'a> {
    pub fn new(rect: Rect, label: &'a str) -> Self {
        Self { rect, label, focused: false, accent: role::ACCENT }
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
    let hovered = hit(button.rect, mouse);
    if hovered {
        focus(widget_id(button.label, button.rect));
    }

    let (background, label_color) = if hovered || button.focused {
        (button.accent, role::BORDER)
    } else {
        (role::PANEL, role::TEXT)
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

/// Un curseur horizontal à crans, pour choisir parmi une petite liste ordonnée.
///
/// Contrairement à une jauge de volume, les crans n'ont pas de sens cumulatif :
/// on ne « remplit » pas une durée, on la désigne. La barre reste donc unie et
/// seul le curseur bouge.
pub fn slider(rect: Rect, steps: usize, step: usize, accent: Color) {
    const CURSOR_WIDTH: f32 = 7.0;
    const TICK_HEIGHT: f32 = 3.0;

    let middle = (rect.y + rect.h / 2.0).floor();
    let travel = rect.w - CURSOR_WIDTH;

    // Le rail, et un repère par cran pour montrer qu'il y a des positions
    // discrètes plutôt qu'un réglage continu.
    draw_rectangle(rect.x, middle, rect.w, 1.0, role::TEXT_DISABLED);
    for index in 0..steps {
        let x = rect.x + tick_offset(travel, steps, index) + (CURSOR_WIDTH / 2.0).floor();
        draw_rectangle(x, middle - TICK_HEIGHT, 1.0, TICK_HEIGHT * 2.0 + 1.0, role::TEXT_DISABLED);
    }

    let cursor = Rect::new(
        rect.x + tick_offset(travel, steps, step),
        rect.y,
        CURSOR_WIDTH,
        rect.h,
    );
    panel(cursor, accent);
}

/// Le cran désigné par le curseur, s'il est sur la barre.
///
/// Sert à *attraper* le curseur. Une fois attrapé, c'est `slider_step_from_x`
/// qui suit le geste.
pub fn slider_step_at(rect: Rect, steps: usize, mouse: Vec2) -> Option<usize> {
    if !hit(grab_area(rect), mouse) {
        return None;
    }

    Some(slider_step_from_x(rect, steps, mouse.x))
}

/// Le cran désigné par une abscisse, sans regarder la hauteur.
///
/// C'est ce qu'il faut pendant un glissement : la main dérive verticalement
/// bien au-delà d'une barre de quelques pixels, et exiger d'y rester ferait
/// décrocher le curseur au premier écart.
pub fn slider_step_from_x(rect: Rect, steps: usize, x: f32) -> usize {
    if steps < 2 {
        return 0;
    }

    // On arrondit au cran le plus proche : viser un rail au pixel demanderait
    // une précision que personne n'a, à la souris comme au doigt.
    let ratio = ((x - rect.x) / rect.w).clamp(0.0, 1.0);
    (ratio * (steps - 1) as f32).round() as usize
}

/// La zone où l'on peut attraper un élément fin.
///
/// Un rail de quelques pixels de haut est presque impossible à viser au doigt :
/// on accepte l'appui un peu au-dessus et un peu en dessous.
pub fn grab_area(rect: Rect) -> Rect {
    const TOLERANCE: f32 = 6.0;

    Rect::new(rect.x, rect.y - TOLERANCE, rect.w, rect.h + TOLERANCE * 2.0)
}

fn tick_offset(travel: f32, steps: usize, index: usize) -> f32 {
    if steps < 2 {
        return 0.0;
    }

    (travel * index as f32 / (steps - 1) as f32).round()
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
pub const HEART_WIDTH: f32 = 7.0;

pub fn star(x: f32, y: f32, earned: bool) {
    star_scaled(x, y, 1.0, earned);
}

pub fn star_scaled(x: f32, y: f32, scale: f32, earned: bool) {
    blit_scaled(&STAR, x, y, scale, if earned { role::STAR } else { role::STAR_EMPTY });
}

/// Une étoile dans sa propre teinte, éteinte tant qu'elle n'est pas gagnée.
///
/// Éteinte plutôt qu'absente, et dans sa couleur plutôt qu'en gris : une étoile
/// bleue sombre annonce qu'il y a une étoile bleue à décrocher. La faire
/// disparaître ne dirait rien, la griser en ferait une étoile ordinaire.
pub fn star_colored(x: f32, y: f32, scale: f32, color: Color, earned: bool) {
    blit_scaled(&STAR, x, y, scale, if earned { color } else { dimmed(color) });
}

/// Une couleur assombrie, assez pour dire « pas encore » sans perdre sa teinte.
pub fn dimmed(color: Color) -> Color {
    const KEEP: f32 = 0.34;

    Color { r: color.r * KEEP, g: color.g * KEEP, b: color.b * KEEP, a: color.a }
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

/// Le palmarès complet d'un niveau : les étoiles dorées, puis celles des modes.
///
/// Les cinq sont toujours dessinées. Ce qui reste à décrocher se lit d'un coup
/// d'oeil, ce qui est le seul intérêt de les montrer.
pub fn level_stars(x: f32, y: f32, gold: u8, gold_total: u8, fast: bool, ultra: bool) {
    const GAP: f32 = 2.0;

    stars_row(x, y, gold, gold_total);

    let mut next = x + stars_row_width(gold_total) + GAP + 2.0;
    for (color, earned) in [(role::STAR_FAST, fast), (role::STAR_ULTRA, ultra)] {
        star_colored(next, y, 1.0, color, earned);
        next += STAR_WIDTH + GAP;
    }
}

/// Largeur du palmarès complet.
pub fn level_stars_width(gold_total: u8) -> f32 {
    stars_row_width(gold_total) + 2.0 + (STAR_WIDTH + 2.0) * 2.0
}

/// Les vies restantes, coeurs pleins puis éteints.
pub fn hearts_row(x: f32, y: f32, remaining: u32, total: u32) {
    const GAP: f32 = 2.0;
    for index in 0..total {
        heart(x + index as f32 * (HEART_WIDTH + GAP), y, index < remaining);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BAR: Rect = Rect { x: 100.0, y: 50.0, w: 200.0, h: 12.0 };

    #[test]
    fn clicking_the_bar_picks_the_nearest_step() {
        // Viser un cran au pixel pres serait impossible : on arrondit.
        assert_eq!(slider_step_at(BAR, 9, vec2(100.0, 56.0)), Some(0));
        assert_eq!(slider_step_at(BAR, 9, vec2(300.0, 56.0)), Some(8));
        assert_eq!(slider_step_at(BAR, 9, vec2(200.0, 56.0)), Some(4), "le milieu");
        assert_eq!(slider_step_at(BAR, 9, vec2(206.0, 56.0)), Some(4), "encore le plus proche");
    }

    #[test]
    fn clicking_outside_the_bar_picks_nothing() {
        assert_eq!(slider_step_at(BAR, 9, vec2(50.0, 56.0)), None);
        assert_eq!(slider_step_at(BAR, 9, vec2(200.0, 10.0)), None);
    }

    #[test]
    fn a_slightly_high_press_still_catches_the_bar() {
        // Un rail de douze pixels de haut est difficile a viser, surtout au
        // doigt : on tolere un peu de marge.
        assert_eq!(slider_step_at(BAR, 9, vec2(200.0, BAR.y - 4.0)), Some(4));
        assert_eq!(slider_step_at(BAR, 9, vec2(200.0, BAR.y + BAR.h + 4.0)), Some(4));
    }

    #[test]
    fn arriving_on_an_element_is_heard_once() {
        forget_focus();
        assert!(!focus_moved(), "le premier releve d'un ecran ne dit rien");

        focus(widget_id("JOUER", BAR));
        assert!(focus_moved(), "le curseur vient d'arriver sur le bouton");

        focus(widget_id("JOUER", BAR));
        assert!(!focus_moved(), "rester dessus ne se reentend pas");
    }

    #[test]
    fn leaving_for_empty_space_stays_silent() {
        // Sinon chaque aller-retour entre deux boutons compterait double.
        forget_focus();
        focus_moved();
        focus(widget_id("OPTIONS", BAR));
        focus_moved();

        assert!(!focus_moved(), "rien sous le curseur");
    }

    #[test]
    fn changing_screen_swallows_the_first_look() {
        // Le bouton qui se trouve sous le curseur a l'arrivee claquerait sans
        // que personne n'ait bouge.
        forget_focus();
        focus_moved();
        focus(widget_id("RETOUR", BAR));
        assert!(focus_moved());

        forget_focus();
        focus(widget_id("REVISION", BAR));

        assert!(!focus_moved());
    }

    #[test]
    fn an_element_keeps_its_identity_while_the_list_scrolls() {
        // Une identite geometrique changerait a chaque frame et ferait crepiter
        // le son pendant tout le defilement.
        let high = Rect::new(0.0, 10.0, 80.0, 16.0);
        let low = Rect::new(0.0, 190.0, 80.0, 16.0);

        assert_eq!(widget_id("ETAPE 3", high), widget_id("ETAPE 3", low));
        assert_ne!(widget_id("ETAPE 3", high), widget_id("ETAPE 4", high));
    }

    #[test]
    fn a_drag_follows_the_hand_beyond_the_bar() {
        // Pendant un glissement la main derive : exiger de rester sur le rail
        // ferait decrocher le curseur au premier ecart.
        assert_eq!(slider_step_from_x(BAR, 9, 300.0), 8);
        assert_eq!(slider_step_from_x(BAR, 9, 1_000.0), 8, "borne a droite");
        assert_eq!(slider_step_from_x(BAR, 9, -500.0), 0, "borne a gauche");
    }

    #[test]
    fn the_cursor_reaches_both_ends() {
        // Un decalage mal calcule laisserait le dernier cran hors de la barre.
        let travel = BAR.w - 7.0;

        assert_eq!(tick_offset(travel, 9, 0), 0.0);
        assert_eq!(tick_offset(travel, 9, 8), travel);
    }
}
