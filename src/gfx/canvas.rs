//! La toile virtuelle : tout le jeu est dessiné dans une image de 384×216,
//! puis agrandie d'un facteur **entier** en filtrage « au plus proche ».
//!
//! C'est la seule façon d'obtenir de vrais pixels carrés. Dessiner directement
//! à la résolution de la fenêtre donnerait des bordures floues et des tailles
//! de texte variables selon l'écran : l'inverse de ce qu'on cherche.
//!
//! Conséquence pour tout le reste du code : les coordonnées sont exprimées en
//! pixels virtuels, jamais en pixels d'écran, et `screen_width()` ne doit plus
//! être appelé par les écrans — ils utilisent `canvas::WIDTH` / `HEIGHT`.

use macroquad::prelude::*;

use super::palette;

/// Largeur de la toile, en pixels virtuels.
pub const WIDTH: f32 = 384.0;
/// Hauteur de la toile, en pixels virtuels (16:9).
pub const HEIGHT: f32 = 216.0;

pub struct Canvas {
    target: RenderTarget,
    camera: Camera2D,
    /// Facteur d'agrandissement entier courant.
    scale: f32,
    /// Coin haut-gauche de l'image agrandie, en pixels d'écran.
    origin: Vec2,
}

impl Canvas {
    pub fn new() -> Self {
        let target = render_target(WIDTH as u32, HEIGHT as u32);
        // Sans cela, l'agrandissement interpolerait et flouterait chaque pixel.
        target.texture.set_filter(FilterMode::Nearest);

        let mut camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, WIDTH, HEIGHT));
        camera.render_target = Some(target.clone());

        Self { target, camera, scale: 1.0, origin: Vec2::ZERO }
    }

    /// Ouvre la frame : tout ce qui est dessiné ensuite atterrit sur la toile.
    pub fn begin(&mut self) {
        self.fit_to_window();
        set_camera(&self.camera);
    }

    /// Ferme la frame et affiche la toile agrandie, centrée dans la fenêtre.
    pub fn end(&self) {
        set_default_camera();

        // Les bandes de letterbox, quand le ratio de la fenêtre n'est pas 16:9.
        clear_background(palette::role::BACKGROUND);

        draw_texture_ex(
            &self.target.texture,
            self.origin.x,
            self.origin.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(WIDTH * self.scale, HEIGHT * self.scale)),
                // Une texture de rendu est stockée à l'envers par rapport à
                // l'écran : sans ce retournement, l'image serait la tête en bas.
                flip_y: true,
                ..Default::default()
            },
        );
    }

    /// Position de la souris **en pixels virtuels**.
    ///
    /// Peut sortir de la toile si le curseur est sur une bande de letterbox ;
    /// les tests de survol s'en chargent naturellement.
    pub fn mouse(&self) -> Vec2 {
        let (x, y) = mouse_position();
        (vec2(x, y) - self.origin) / self.scale
    }

    fn fit_to_window(&mut self) {
        let by_width = screen_width() / WIDTH;
        let by_height = screen_height() / HEIGHT;

        // Un facteur entier garde tous les pixels de la même taille. En dessous
        // de la taille native (fenêtre minuscule), on reste à 1 et on rogne
        // plutôt que de déformer.
        self.scale = by_width.min(by_height).floor().max(1.0);
        self.origin = vec2(
            ((screen_width() - WIDTH * self.scale) / 2.0).floor(),
            ((screen_height() - HEIGHT * self.scale) / 2.0).floor(),
        );
    }
}
