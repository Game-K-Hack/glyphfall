use macroquad::prelude::*;
use crate::core::{
    GameState, 
    Screen, 
    TILE_WIDTH,
    TILE_HEIGHT,
    COLS,
    TARGET_Y,
    korean_font
};

pub fn game_screen(state: &mut GameState) {
    if state.game_over {
        // --- ÉCRAN DE GAME OVER ---
        clear_background(BLACK);
        draw_text("GAME OVER", screen_width() / 2.0 - 100.0, screen_height() / 2.0 - 20.0, 40.0, RED);
        draw_text(&format!("Score final : {}", state.score), screen_width() / 2.0 - 80.0, screen_height() / 2.0 + 20.0, 25.0, WHITE);
        draw_text("Appuyez sur ESPACE pour rejouer", screen_width() / 2.0 - 150.0, screen_height() / 2.0 + 60.0, 20.0, GRAY);
        draw_text("Appuyez sur ÉCHAP pour quitter et revenir au menu", screen_width() / 2.0 - 230.0, screen_height() / 2.0 + 80.0, 20.0, GRAY);

        if is_key_pressed(KeyCode::Space) {
            *state = GameState::new();
            state.current_screen = Screen::Playing;
        }
        if is_key_pressed(KeyCode::Escape) {
            state.current_screen = Screen::MainMenu;
        }
        next_frame().await;
        continue;
    }

    // --- 1. LOGIQUE & MISES À JOUR (UPDATE) ---
    let dt = get_frame_time();

    // Gestion de l'apparition des tuiles
    state.spawn_timer += dt;
    if state.spawn_timer > 1.0 { // Fait apparaître une tuile toutes les secondes
        state.spawn_tile();
        state.spawn_timer = 0.0;
        // Augmente légèrement la vitesse pour corser le jeu
        state.speed += 5.0; 
    }

    if is_key_pressed(KeyCode::Backspace) {
        state.input_buffer.pop();
    }

    if let Some(c) = get_char_pressed() {
        if c.is_alphanumeric() {
            // Cette fois on enregistre en MINUSCULE car nos traductions sont en minuscules
            state.input_buffer.push(c.to_ascii_lowercase()); 
        }
    }

    
    // --- 2. LOGIQUE DE VALIDATION ---
    let mut missed_tile = false;
    let input_validated = is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter);

    for tile in state.tiles.iter_mut() {
        tile.y += state.speed * dt;

        // Le joueur doit avoir tapé EXACTEMENT la bonne romanisation
        if input_validated && !tile.is_pressed && state.input_buffer == tile.romanization {
            tile.is_pressed = true;
            state.score += 10;
        }

        if tile.y > screen_height() && !tile.is_pressed {
            missed_tile = true;
        }
    }

    if input_validated {
        state.input_buffer.clear();
    }

    // Sanction si une tuile est manquée
    if missed_tile {
        if state.lives > 0 {
            state.lives -= 1;
        }
        if state.lives == 0 {
            state.game_over = true;
        }
    }

    // Nettoyage : on enlève les tuiles sorties de l'écran ou déjà validées
    state.tiles.retain(|tile| tile.y < screen_height() && !tile.is_pressed);

    // --- 2. RENDU GRAPHIQUE (DRAW) ---
    clear_background(DARKGRAY);

    // Dessin des 4 colonnes
    let start_x = (screen_width() - (COLS as f32 * TILE_WIDTH)) / 2.0;
    for i in 0..COLS {
        let x = start_x + i as f32 * TILE_WIDTH;
        draw_line(x, 0.0, x, screen_height(), 1.0, GRAY);
    }
    // Ligne de fin de la dernière colonne
    draw_line(start_x + COLS as f32 * TILE_WIDTH, 0.0, start_x + COLS as f32 * TILE_WIDTH, screen_height(), 1.0, GRAY);

    // Dessin de la ligne cible (Zone de validation)
    draw_line(start_x, TARGET_Y, start_x + (COLS as f32 * TILE_WIDTH), TARGET_Y, 3.0, RED);

    // Dessin des tuiles
    for tile in &state.tiles {
        let x = start_x + tile.col as f32 * TILE_WIDTH;
        
        // Dessin de la tuile (dans la boucle de rendu des tuiles)
        let color = if tile.is_pressed { GREEN } else { BLACK };
        draw_rectangle(x + 2.0, tile.y, TILE_WIDTH - 4.0, TILE_HEIGHT, color);

        // On affiche le caractère coréen !
        draw_text_ex(
            &tile.hangul, 
            x + (TILE_WIDTH / 2.0) - 15.0, 
            tile.y + (TILE_HEIGHT / 2.0) + 10.0, 
            TextParams {
                font: Some(&korean_font),
                font_size: 44,
                color: WHITE,
                ..Default::default()
            },
        );
    }

    // Interface utilisateur (Score & Vies)
    draw_text(&format!("SCORE: {}", state.score), 20.0, 40.0, 30.0, WHITE);
    draw_text(&format!("VIES: {}", "❤️".repeat(state.lives as usize)), 20.0, 80.0, 30.0, RED);

    let bar_width = 400.0;
    let bar_height = 50.0;
    let bar_x = (screen_width() - bar_width) / 2.0;
    let bar_y = screen_height() - 80.0;

    // Dessin du fond de la barre (Gris foncé avec une bordure blanche)
    draw_rectangle(bar_x, bar_y, bar_width, bar_height, BLACK);
    draw_rectangle_lines(bar_x, bar_y, bar_width, bar_height, 2.0, WHITE);

    // Affichage du texte saisi à l'intérieur de la barre
    if state.input_buffer.is_empty() {
        // Petit texte d'aide si la barre est vide
        draw_text("Tapez les lettres ici...", bar_x + 15.0, bar_y + 32.0, 20.0, GRAY);
    } else {
        // Affiche la saisie actuelle du joueur
        draw_text(&state.input_buffer, bar_x + 15.0, bar_y + 35.0, 26.0, YELLOW);
    }
}