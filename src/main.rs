use macroquad::prelude::*;

mod core;
mod data;
mod screens;
mod window;

use crate::core::{Assets, GameState, Screen};
use crate::window::window_conf;
use crate::screens::main_menu::main_menu_screen;
use crate::screens::game::game_screen;


#[macroquad::main(window_conf)]
async fn main() {
    // Initialise le générateur de nombres aléatoires
    rand::srand(miniquad::date::now() as u64);
    
    // Le contenu est validé au démarrage : mieux vaut un écran d'erreur lisible
    // qu'un chemin d'apprentissage silencieusement cassé.
    let _catalog = match data::load_catalog() {
        Ok(catalog) => catalog,
        Err(error) => return fatal_error_screen(&error.to_string()).await,
    };

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

/// Affiche une erreur de contenu jusqu'à ce que la fenêtre soit fermée.
/// Un `panic!` ne serait vu par personne : le jeu se lance sans terminal.
async fn fatal_error_screen(message: &str) {
    loop {
        clear_background(BLACK);
        draw_text("CONTENU INVALIDE", 40.0, 80.0, 34.0, RED);
        draw_text(message, 40.0, 130.0, 18.0, WHITE);
        draw_text("Corrigez le fichier TOML puis relancez.", 40.0, 170.0, 18.0, GRAY);
        next_frame().await
    }
}
