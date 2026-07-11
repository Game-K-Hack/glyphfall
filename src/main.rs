use macroquad::prelude::*;

const TILE_WIDTH: f32 = 100.0;
const TILE_HEIGHT: f32 = 140.0;
const COLS: i32 = 4; // 4 colonnes comme dans Piano Tiles
const TARGET_Y: f32 = 500.0; // La ligne de validation en bas de l'écran

struct Tile {
    col: i32,
    y: f32,
    key: char,
    is_pressed: bool,
}

struct GameState {
    tiles: Vec<Tile>,
    score: u32,
    lives: u32,
    speed: f32,
    spawn_timer: f32,
    game_over: bool,
    current_screen: Screen,
    input_buffer: String,
}

impl GameState {
    fn new() -> Self {
        Self {
            tiles: Vec::new(),
            score: 0,
            lives: 3,
            speed: 200.0,
            spawn_timer: 0.0,
            game_over: false,
            current_screen: Screen::MainMenu,
            input_buffer: String::new(),
        }
    }

    fn spawn_tile(&mut self) {
        let col = rand::gen_range(0, COLS);
        // Génère une lettre majuscule aléatoire entre A et Z
        let key = (rand::gen_range(65, 91) as u8) as char;
        
        self.tiles.push(Tile {
            col,
            y: -TILE_HEIGHT, // Démarre juste au-dessus de l'écran
            key,
            is_pressed: false,
        });
    }
}

#[derive(PartialEq)]
enum Screen {
    MainMenu,
    Playing,
}

fn window_conf() -> Conf {
    // 1. On charge tes fichiers (qu'ils soient en PNG, RAW ou autre)
    let icon16_bytes = include_bytes!("assets/icons/icon16.rgba");
    let icon32_bytes = include_bytes!("assets/icons/icon32.rgba");
    let icon64_bytes = include_bytes!("assets/icons/icon64.rgba");
    
    // 2. On prépare les conteneurs vides aux tailles strictes exigées par miniquad
    let mut small_array = [0u8; 16 * 16 * 4];   // 1024 octets
    let mut medium_array = [0u8; 32 * 32 * 4];  // 4096 octets
    let mut big_array = [0u8; 64 * 64 * 4];     // 16384 octets

    // 3. On copie intelligemment les données sans dépasser les limites
    let len_small = icon16_bytes.len().min(small_array.len());
    small_array[..len_small].copy_from_slice(&icon16_bytes[..len_small]);

    let len_medium = icon32_bytes.len().min(medium_array.len());
    medium_array[..len_medium].copy_from_slice(&icon32_bytes[..len_medium]);

    let len_big = icon64_bytes.len().min(big_array.len());
    big_array[..len_big].copy_from_slice(&icon64_bytes[..len_big]);

    // 4. On crée la structure finale
    let icon = miniquad::conf::Icon {
        small: small_array,
        medium: medium_array,
        big: big_array,
    };

    Conf {
        window_title: "AlphaTiles".to_string(),
        fullscreen: false,
        icon: Some(icon),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // Initialise le générateur de nombres aléatoires
    rand::srand(miniquad::date::now() as u64);
    
    let mut state = GameState::new();

    loop {
        // --- GESTION DES ÉCRANS ---
        match state.current_screen {
            Screen::MainMenu => {
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
                        state = GameState::new(); // Réinitialise les variables
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

            Screen::Playing => {
                if state.game_over {
                    // --- ÉCRAN DE GAME OVER ---
                    clear_background(BLACK);
                    draw_text("GAME OVER", screen_width() / 2.0 - 100.0, screen_height() / 2.0 - 20.0, 40.0, RED);
                    draw_text(&format!("Score final : {}", state.score), screen_width() / 2.0 - 80.0, screen_height() / 2.0 + 20.0, 25.0, WHITE);
                    draw_text("Appuyez sur ESPACE pour rejouer", screen_width() / 2.0 - 150.0, screen_height() / 2.0 + 60.0, 20.0, GRAY);
                    draw_text("Appuyez sur ÉCHAP pour quitter et revenir au menu", screen_width() / 2.0 - 230.0, screen_height() / 2.0 + 80.0, 20.0, GRAY);

                    if is_key_pressed(KeyCode::Space) {
                        state = GameState::new();
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
                    // On ignore les espaces ou les touches spéciales, on ne garde que les lettres/chiffres
                    if c.is_alphanumeric() {
                        state.input_buffer.push(c.to_ascii_uppercase());
                    }
                }

                // Déplacement des tuiles et vérification des entrées
                let mut missed_tile = false;
                
                for tile in state.tiles.iter_mut() {
                    tile.y += state.speed * dt;

                    // Si le joueur a écrit EXACTEMENT la lettre de la tuile dans sa barre
                    // (Tu peux aussi adapter si tu veux qu'il écrive des mots entiers plus tard !)
                    if !tile.is_pressed && state.input_buffer.contains(tile.key) {
                        tile.is_pressed = true;
                        state.score += 10;
                        
                        // On vide la barre une fois qu'on a validé la tuile
                        state.input_buffer.clear();
                    }

                    if tile.y > screen_height() && !tile.is_pressed {
                        missed_tile = true;
                    }
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
                    
                    // Couleur de la tuile
                    let color = if tile.is_pressed { GREEN } else { BLACK };
                    draw_rectangle(x + 2.0, tile.y, TILE_WIDTH - 4.0, TILE_HEIGHT, color);

                    // Affichage de la lettre au centre de la tuile
                    let text = &tile.key.to_string();
                    draw_text(
                        text, 
                        x + (TILE_WIDTH / 2.0) - 10.0, 
                        tile.y + (TILE_HEIGHT / 2.0) + 10.0, 
                        30.0, 
                        WHITE
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
        }
        next_frame().await
    }
}
