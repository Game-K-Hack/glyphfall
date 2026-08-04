use macroquad::prelude::*;

use crate::core::{COLS, GameState, Screen, TARGET_Y, TILE_HEIGHT, TILE_WIDTH};
use crate::gfx::palette::role;
use crate::gfx::ui;
use crate::gfx::{Fonts, canvas, fonts};

/// Provisoire : la langue jouée viendra du niveau choisi.
const LANGUAGE: &str = "ko";

/// Bord gauche de la grille, centrée sur la toile.
const PLAYFIELD_X: f32 = (canvas::WIDTH - COLS as f32 * TILE_WIDTH) / 2.0;

pub fn game_screen(state: &mut GameState, fonts_set: &Fonts, mouse: Vec2) {
    if state.game_over {
        game_over_screen(state, fonts_set, mouse);
        return;
    }

    update(state);
    draw(state, fonts_set);
}

fn update(state: &mut GameState) {
    let dt = get_frame_time();

    state.spawn_timer += dt;
    if state.spawn_timer > 1.4 {
        state.spawn_tile();
        state.spawn_timer = 0.0;
        state.speed += 1.5;
    }

    if is_key_pressed(KeyCode::Backspace) {
        state.input_buffer.pop();
    }
    if let Some(character) = get_char_pressed() {
        if character.is_alphanumeric() {
            state.input_buffer.push(character.to_ascii_lowercase());
        }
    }

    let validated = is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter);
    let mut missed = 0;

    for tile in state.tiles.iter_mut() {
        tile.y += state.speed * dt;

        if validated && !tile.is_pressed && state.input_buffer == tile.romanization {
            tile.is_pressed = true;
            state.score += 10;
        }

        if tile.y > canvas::HEIGHT && !tile.is_pressed {
            missed += 1;
        }
    }

    if validated {
        state.input_buffer.clear();
    }

    state.lives = state.lives.saturating_sub(missed);
    if state.lives == 0 {
        state.game_over = true;
    }

    state.tiles.retain(|tile| tile.y < canvas::HEIGHT && !tile.is_pressed);
}

fn draw(state: &GameState, fonts_set: &Fonts) {
    clear_background(role::BACKGROUND);

    let playfield =
        Rect::new(PLAYFIELD_X, 0.0, COLS as f32 * TILE_WIDTH, canvas::HEIGHT);
    ui::fill(playfield, role::PANEL);

    for column in 0..=COLS {
        let x = PLAYFIELD_X + column as f32 * TILE_WIDTH;
        draw_rectangle(x, 0.0, 1.0, canvas::HEIGHT, role::BORDER);
    }

    // La ligne de validation : passé ce trait, la tuile est perdue.
    draw_rectangle(playfield.x, TARGET_Y, playfield.w, 1.0, role::DANGER);

    let script = fonts_set.script(LANGUAGE);
    for tile in &state.tiles {
        let rect = Rect::new(
            PLAYFIELD_X + tile.col as f32 * TILE_WIDTH + 1.0,
            tile.y.floor(),
            TILE_WIDTH - 2.0,
            TILE_HEIGHT,
        );

        ui::panel(rect, if tile.is_pressed { role::SUCCESS } else { role::BORDER });
        ui::glyph_centered(script, &tile.glyph, rect, 24, role::TEXT);
    }

    draw_hud(state, fonts_set);
    draw_input_bar(state, fonts_set);
}

fn draw_hud(state: &GameState, fonts_set: &Fonts) {
    ui::text(fonts_set, &format!("{:05}", state.score), 6.0, 6.0, fonts::TEXT, role::TEXT);
    ui::hearts_row(6.0, 18.0, state.lives, 3);
}

fn draw_input_bar(state: &GameState, fonts_set: &Fonts) {
    const WIDTH: f32 = 160.0;
    let bar = Rect::new(((canvas::WIDTH - WIDTH) / 2.0).floor(), 192.0, WIDTH, 16.0);
    ui::panel(bar, role::BORDER);

    if state.input_buffer.is_empty() {
        ui::text(fonts_set, "tapez la lecture", bar.x + 5.0, bar.y + 4.0, fonts::TEXT, role::TEXT_DISABLED);
    } else {
        ui::text(fonts_set, &state.input_buffer, bar.x + 5.0, bar.y + 4.0, fonts::TEXT, role::STAR);
    }
}

fn game_over_screen(state: &mut GameState, fonts_set: &Fonts, mouse: Vec2) {
    clear_background(role::BACKGROUND);

    ui::text_centered(fonts_set, "GAME OVER", canvas::WIDTH / 2.0, 56.0, fonts::TITLE, role::DANGER);
    ui::text_centered(
        fonts_set,
        &format!("SCORE {:05}", state.score),
        canvas::WIDTH / 2.0,
        84.0,
        fonts::TEXT,
        role::TEXT,
    );

    const BUTTON_WIDTH: f32 = 168.0;
    let x = ((canvas::WIDTH - BUTTON_WIDTH) / 2.0).floor();

    let retry = Rect::new(x, 120.0, BUTTON_WIDTH, 20.0);
    if ui::button(fonts_set, mouse, ui::Button::new(retry, "REJOUER"))
        || is_key_pressed(KeyCode::Space)
    {
        *state = GameState::new();
        state.current_screen = Screen::Playing;
    }

    let menu = Rect::new(x, 148.0, BUTTON_WIDTH, 20.0);
    if ui::button(fonts_set, mouse, ui::Button::new(menu, "MENU").accent(role::TEXT_MUTED))
        || is_key_pressed(KeyCode::Escape)
    {
        state.current_screen = Screen::MainMenu;
    }
}
