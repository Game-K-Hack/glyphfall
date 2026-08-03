use macroquad::prelude::*;
use crate::core::{GameState, Screen};

pub fn main_menu_screen(state: &mut GameState) {
    clear_background(BLACK);

    // Titre du jeu
    draw_text("ALPHA TILES", screen_width() / 2.0 - 120.0, 150.0, 45.0, WHITE);

    // Coordonnées et tailles des boutons
    let btn_width = 250.0;
    let btn_height = 50.0;
    let btn_x = (screen_width() - btn_width) / 2.0;
    
    let play_y = 280.0;
    let quit_y = 360.0;

    // --- BOUTON 1 : LANCER ALPHABET ---
    // Dessin du bouton
    draw_rectangle(btn_x, play_y, btn_width, btn_height, BLUE);
    draw_text("LANCER ALPHABET", btn_x + 25.0, play_y + 32.0, 22.0, WHITE);

    // Clic sur Lancer
    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        if mx >= btn_x && mx <= btn_x + btn_width && my >= play_y && my <= play_y + btn_height {
            *state = GameState::new(); // Réinitialise les variables
            state.current_screen = Screen::Playing; // Lance la partie !
        }
    }

    // --- BOUTON 2 : QUITTER ---
    draw_rectangle(btn_x, quit_y, btn_width, btn_height, RED);
    draw_text("QUITTER", btn_x + 75.0, quit_y + 32.0, 22.0, WHITE);

    // Clic sur Quitter
    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        if mx >= btn_x && mx <= btn_x + btn_width && my >= quit_y && my <= quit_y + btn_height {
            std::process::exit(0); // Ferme proprement le programme en Rust
        }
    }
}