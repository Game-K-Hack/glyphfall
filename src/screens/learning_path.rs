//! Le chemin d'apprentissage d'une langue : la suite des niveaux, ce qui est
//! ouvert, ce qui reste verrouillé, et les étoiles déjà gagnées.
//!
//! L'ordre affiché vient du champ `order` des fichiers TOML, mais c'est bien
//! `requires` qui décide de ce qui est jouable : deux niveaux peuvent se
//! débloquer en parallèle.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
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

/// Hauteur utile du cadre, marges déduites.
const VIEW_HEIGHT: f32 = VIEWPORT_BOTTOM - VIEWPORT_TOP - 8.0;

/// Déplacement par cran de molette, en pixels virtuels.
///
/// La *magnitude* envoyée par la molette n'est pas portable — 120 par cran sous
/// Windows, un nombre de pixels ou de lignes selon le navigateur. On n'en garde
/// donc que le sens, et on applique un pas maison de moins d'une ligne.
const WHEEL_STEP: f32 = 20.0;

/// Au-delà de ce déplacement, un appui devient un glissement et ne vaut plus
/// pour un clic. En dessous, c'est la main qui tremble.
const DRAG_SLOP: f32 = 4.0;

/// L'état de l'écran, conservé d'une frame à l'autre.
pub struct PathView {
    /// `None` tant que l'écran ne s'est pas placé sur l'étape en cours.
    selected: Option<usize>,
    /// Décalage du contenu vers le haut, en pixels virtuels.
    scroll: f32,
    /// Appui en cours, à la souris ou au doigt.
    drag: Option<Drag>,
}

/// Un appui maintenu, qui fait défiler tant qu'il se déplace.
struct Drag {
    last_y: f32,
    /// Distance parcourue depuis l'appui, pour distinguer un glissement d'un clic.
    travelled: f32,
}

impl PathView {
    pub fn new() -> Self {
        Self { selected: None, scroll: 0.0, drag: None }
    }
}

pub fn learning_path_screen(
    app: &App,
    language_id: &str,
    view: &mut PathView,
    mouse: Vec2,
) -> Transition {
    clear_background(role::BACKGROUND);

    let Some(language) = app.catalog.language(language_id) else {
        // La langue a disparu du catalogue entre-temps : impossible en pratique,
        // mais mieux vaut revenir en arrière que d'indexer dans le vide.
        return Transition::Pop;
    };
    let count = language.levels.len();
    let limit = max_scroll(count);

    // --- Entrées, avant le rendu -----------------------------------------
    // Les traiter après dessinerait l'état de la frame précédente : au
    // glissement, ce retard d'une image se sent immédiatement.

    // À l'ouverture, on se place sur la première étape non terminée. Sur un
    // chemin d'une quinzaine de niveaux, repartir du premier à chaque visite
    // obligerait à redescendre toute la liste.
    let first_visit = view.selected.is_none();
    let mut selected = view.selected.unwrap_or_else(|| {
        language
            .levels
            .iter()
            .position(|level| !app.progress.is_completed(&level.id))
            .unwrap_or(count - 1)
    });
    selected = selected.min(count.saturating_sub(1));

    if is_key_pressed(KeyCode::Down) {
        selected = (selected + 1) % count;
        view.scroll = scrolled_into_view(view.scroll, selected);
        app.sfx.navigate();
    }
    if is_key_pressed(KeyCode::Up) {
        selected = (selected + count - 1) % count;
        view.scroll = scrolled_into_view(view.scroll, selected);
        app.sfx.navigate();
    }
    if first_visit {
        view.scroll = scrolled_into_view(view.scroll, selected);
    }

    // La molette défile sans toucher à la sélection : on veut pouvoir parcourir
    // la liste des yeux sans perdre l'étape que l'on visait.
    let wheel = mouse_wheel().1;
    if wheel != 0.0 {
        view.scroll -= wheel.signum() * WHEEL_STEP;
    }

    // Maintenir et tirer fait défiler. macroquad traduit les touchers en
    // événements souris : le même code sert donc au doigt, seule façon de faire
    // défiler sur un téléphone, où il n'y a ni molette ni flèches.
    if is_mouse_button_pressed(MouseButton::Left) {
        view.drag = Some(Drag { last_y: mouse.y, travelled: 0.0 });
    }
    if is_mouse_button_down(MouseButton::Left) {
        if let Some(drag) = &mut view.drag {
            let delta = mouse.y - drag.last_y;
            view.scroll -= delta;
            drag.travelled += delta.abs();
            drag.last_y = mouse.y;
        }
    }

    let mut tapped = false;
    if is_mouse_button_released(MouseButton::Left) {
        // Un glissement ne vaut pas pour un clic : sans cela, tirer la liste en
        // partant d'une étape la lancerait.
        tapped = view.drag.take().is_some_and(|drag| drag.travelled <= DRAG_SLOP);
    }

    view.scroll = view.scroll.clamp(0.0, limit);

    // --- Rendu ------------------------------------------------------------
    // Le survol est cherché avant de dessiner : sans cela, les lignes situées
    // au-dessus de celle survolée seraient dessinées avec l'ancienne sélection.
    for index in 0..count {
        if ui::hit(row_rect(index, view.scroll), mouse) {
            selected = index;
        }
    }

    let mut chosen = None;
    for (index, level) in language.levels.iter().enumerate() {
        let row = row_rect(index, view.scroll);
        if row.y + row.h <= VIEWPORT_TOP || row.y >= VIEWPORT_BOTTOM {
            continue;
        }

        let unlocked = app.progress.is_unlocked(level);
        if unlocked && tapped && ui::hit(row, mouse) {
            chosen = Some(level);
        }

        // Le trait qui relie cette étape à la suivante.
        if index + 1 < count {
            ui::dotted_line(
                NODE_X + NODE_SIZE / 2.0,
                row.y + NODE_SIZE,
                row_rect(index + 1, view.scroll).y,
                role::TEXT_DISABLED,
            );
        }

        draw_row(&app.fonts, level, row, index == selected, unlocked, app.progress.stars(&level.id));
    }

    // Les lignes sont dessinées entières, quitte à déborder, puis on recouvre ce
    // qui dépasse. Ne pas dessiner les lignes incomplètes ferait disparaître
    // d'un coup toute ligne à moitié sortie et laisserait un trou dans le cadre.
    ui::fill(Rect::new(0.0, 0.0, canvas::WIDTH, VIEWPORT_TOP), role::BACKGROUND);
    ui::fill(
        Rect::new(0.0, VIEWPORT_BOTTOM, canvas::WIDTH, canvas::HEIGHT - VIEWPORT_BOTTOM),
        role::BACKGROUND,
    );

    draw_header(&app.fonts, language, &app.progress);
    draw_scrollbar(view.scroll, limit);
    draw_footer(&app.fonts, language, selected, &app.progress);

    if is_key_pressed(KeyCode::Enter) {
        let level = &language.levels[selected];
        if app.progress.is_unlocked(level) {
            chosen = Some(level);
        }
    }

    view.selected = Some(selected);

    match chosen {
        Some(level) => {
            app.sfx.confirm();
            Transition::Push(Screen::Briefing {
                language: language.id.clone(),
                level: level.id.clone(),
            })
        }
        None => Transition::Stay,
    }
}

/// Hauteur totale de la liste, en pixels virtuels.
fn content_height(count: usize) -> f32 {
    count.saturating_sub(1) as f32 * ROW_STEP + ROW_HEIGHT
}

fn max_scroll(count: usize) -> f32 {
    (content_height(count) - VIEW_HEIGHT).max(0.0)
}

/// Décale la liste juste assez pour ramener une étape dans le cadre.
///
/// Sauter au centre serait plus simple mais désorientant : la liste bougerait
/// même quand l'étape suivante est déjà sous les yeux.
fn scrolled_into_view(scroll: f32, index: usize) -> f32 {
    let top = index as f32 * ROW_STEP;
    let bottom = top + ROW_HEIGHT;

    if top < scroll {
        top
    } else if bottom > scroll + VIEW_HEIGHT {
        bottom - VIEW_HEIGHT
    } else {
        scroll
    }
}

fn row_rect(index: usize, scroll: f32) -> Rect {
    let y = VIEWPORT_TOP + 4.0 + index as f32 * ROW_STEP - scroll;
    Rect::new(NODE_X - 4.0, y.round(), canvas::WIDTH - (NODE_X - 4.0) * 2.0, ROW_HEIGHT)
}

/// Une barre discrète à droite, qui dit où l'on se trouve dans la liste.
///
/// Sans elle, rien n'indique qu'il reste des étapes hors du cadre.
fn draw_scrollbar(scroll: f32, limit: f32) {
    if limit <= 0.0 {
        return;
    }

    const WIDTH: f32 = 2.0;
    let track = Rect::new(
        canvas::WIDTH - 6.0,
        VIEWPORT_TOP + 4.0,
        WIDTH,
        VIEWPORT_BOTTOM - VIEWPORT_TOP - 8.0,
    );
    ui::fill(track, role::PANEL);

    let visible_part = VIEW_HEIGHT / (VIEW_HEIGHT + limit);
    let height = (track.h * visible_part).max(6.0).floor();
    let travel = track.h - height;

    ui::fill(
        Rect::new(track.x, track.y + (travel * scroll / limit).floor(), WIDTH, height),
        role::TEXT_MUTED,
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Quinze étapes, comme le chemin coréen.
    const COUNT: usize = 15;

    #[test]
    fn the_frame_only_moves_when_the_selection_leaves_it() {
        // Une étape déjà visible ne doit rien faire bouger.
        assert_eq!(scrolled_into_view(0.0, 2), 0.0);

        // Juste en dessous du cadre : il descend du strict nécessaire.
        let just_below = scrolled_into_view(0.0, 5);
        assert!(just_below > 0.0 && just_below < ROW_STEP * 2.0, "décalage : {just_below}");

        // Au-dessus : il remonte pile sur l'étape choisie.
        assert_eq!(scrolled_into_view(200.0, 2), 2.0 * ROW_STEP);
    }

    #[test]
    fn the_last_step_can_be_reached() {
        // Une fois la liste défilée à fond, la dernière étape doit tenir
        // entièrement dans le cadre, sinon elle resterait inatteignable.
        let limit = max_scroll(COUNT);

        assert_eq!(scrolled_into_view(0.0, COUNT - 1), limit);
    }

    #[test]
    fn a_short_list_never_scrolls() {
        // Moins d'étapes que de place : rien à faire défiler, et un calcul non
        // borné donnerait ici une limite négative.
        assert_eq!(max_scroll(3), 0.0);
        assert_eq!(max_scroll(0), 0.0);
        assert_eq!(scrolled_into_view(0.0, 2), 0.0);
    }

    #[test]
    fn one_wheel_notch_moves_less_than_a_row() {
        // La magnitude envoyée par la molette n'est pas portable : sous Windows
        // un cran vaut 120. L'employer telle quelle envoyait la liste en butée
        // dès le premier cran.
        assert!(WHEEL_STEP < ROW_STEP, "un cran ne doit pas sauter une étape entière");
    }
}
