//! La question posée au premier lancement : combien de temps par jour ?
//!
//! Un objectif qu'on se fixe soi-même tient mieux qu'un objectif imposé, et
//! le poser avant la première partie évite de l'oublier ensuite. Il reste
//! modifiable à tout moment dans les options.

use macroquad::prelude::*;

use crate::app::{App, Screen, Transition};
use crate::gfx::palette::role;
use crate::gfx::ui::{self, Button};
use crate::gfx::{canvas, fonts};
use crate::settings::{DAILY_GOALS, goal_label};

/// La barre de choix, assez large pour que neuf crans restent distincts.
const BAR: Rect = Rect { x: 42.0, y: 108.0, w: 300.0, h: 14.0 };

pub fn daily_goal_screen(
    app: &mut App,
    step: &mut usize,
    dragging: &mut bool,
    mouse: Vec2,
) -> Transition {
    clear_background(role::BACKGROUND);

    *step = (*step).min(DAILY_GOALS.len() - 1);

    ui::text_centered(
        &app.fonts,
        "TON OBJECTIF",
        canvas::WIDTH / 2.0,
        26.0,
        fonts::TITLE,
        role::TITLE,
    );

    for (index, line) in [
        "Combien de temps par jour ?",
        "Le jeu te previendra quand tu y seras.",
    ]
    .iter()
    .enumerate()
    {
        ui::text_centered(
            &app.fonts,
            line,
            canvas::WIDTH / 2.0,
            54.0 + index as f32 * 11.0,
            fonts::TEXT,
            role::TEXT_MUTED,
        );
    }

    // La valeur choisie, en gros au-dessus de la barre : c'est elle qu'on
    // regarde en déplaçant le curseur, pas le curseur lui-même.
    let minutes = DAILY_GOALS[*step];
    let (label, color) =
        if minutes == 0 { ("SANS ALERTE".to_string(), role::TEXT_MUTED) } else { (goal_label(minutes), role::ACCENT) };
    ui::text_centered(&app.fonts, &label, canvas::WIDTH / 2.0, 84.0, fonts::TITLE, color);

    let before = *step;
    *step = pick(*step, dragging, BAR, mouse);
    // Le curseur qui saute d'un cran est un déplacement comme un autre.
    if *step != before {
        app.sfx.navigate();
    }
    ui::slider(BAR, DAILY_GOALS.len(), *step, role::ACCENT);

    // Les deux extrémités, pour que la course de la barre se lise d'un coup.
    ui::text(&app.fonts, "AUCUN", BAR.x, BAR.y + BAR.h + 4.0, fonts::TEXT, role::TEXT_DISABLED);
    let last = goal_label(DAILY_GOALS[DAILY_GOALS.len() - 1]);
    let width = ui::text_width(&app.fonts, &last, fonts::TEXT);
    ui::text(
        &app.fonts,
        &last,
        BAR.x + BAR.w - width,
        BAR.y + BAR.h + 4.0,
        fonts::TEXT,
        role::TEXT_DISABLED,
    );

    ui::text_centered(
        &app.fonts,
        "MODIFIABLE PLUS TARD DANS LES OPTIONS",
        canvas::WIDTH / 2.0,
        158.0,
        fonts::TEXT,
        role::TEXT_DISABLED,
    );

    let confirm = Rect::new(((canvas::WIDTH - 140.0) / 2.0).floor(), 178.0, 140.0, 20.0);
    let pressed = ui::button(&app.fonts, mouse, Button::new(confirm, "C'EST PARTI").focused(true));

    if pressed || is_key_pressed(KeyCode::Enter) {
        app.settings.daily_goal = Some(minutes);
        app.settings.save();
        app.sfx.confirm();

        // `Replace` et non `Push` : la question ne doit pas réapparaître en
        // revenant de l'écran suivant.
        return Transition::Replace(match app.settings.random_fonts {
            None => Screen::FontChoice,
            Some(_) => Screen::LanguageSelect { selected: 0 },
        });
    }

    Transition::Stay
}

/// Le cran choisi, au clavier ou en tirant le curseur.
fn pick(step: usize, dragging: &mut bool, bar: Rect, mouse: Vec2) -> usize {
    if is_key_pressed(KeyCode::Right) {
        return (step + 1).min(DAILY_GOALS.len() - 1);
    }
    if is_key_pressed(KeyCode::Left) {
        return step.saturating_sub(1);
    }

    // Le curseur s'attrape par un appui sur la barre, puis suit la main jusqu'au
    // relâchement. Sans cette prise, il faudrait rester sur un rail de quatorze
    // pixels pendant tout le geste, et le moindre écart le ferait décrocher.
    if is_mouse_button_pressed(MouseButton::Left) && ui::slider_step_at(bar, 2, mouse).is_some() {
        *dragging = true;
    }
    if !is_mouse_button_down(MouseButton::Left) {
        *dragging = false;
    }

    if *dragging {
        return ui::slider_step_from_x(bar, DAILY_GOALS.len(), mouse.x);
    }

    step
}
