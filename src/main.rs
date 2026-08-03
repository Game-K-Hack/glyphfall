use macroquad::prelude::*;

mod core;
mod window;
mod screens;

use crate::core::{Assets, GameState, Screen};
use crate::window::window_conf;
use crate::screens::main_menu::main_menu_screen;
use crate::screens::game::game_screen;


#[macroquad::main(window_conf)]
async fn main() {
    // Initialise le générateur de nombres aléatoires
    rand::srand(miniquad::date::now() as u64);
    
    // Chargées une seule fois : recharger les polices à chaque partie serait coûteux.
    let assets = Assets::load();
    let mut state = GameState::new();

    loop {
        // --- GESTION DES ÉCRANS ---
        match state.current_screen {
            Screen::MainMenu => main_menu_screen(&mut state),
            Screen::Playing => game_screen(&mut state, &assets),
        }
        next_frame().await
    }
}
