//! La palette du jeu : seize couleurs, pas une de plus.
//!
//! C'est « Sweetie 16 » (GrafxKid, domaine public), une palette 8-bit dont les
//! teintes sont pensées pour aller ensemble. S'y tenir strictement est ce qui
//! donne à l'interface son unité : aucune couleur ne doit être écrite en dur
//! ailleurs dans le projet.

// La palette est livree entiere : ses seize couleurs forment un ensemble
// coherent, et n'en declarer que celles utilisees aujourd'hui obligerait a
// retrouver les valeurs exactes a chaque nouvel ecran.
#![allow(dead_code)]

use macroquad::prelude::Color;

const fn hex(value: u32) -> Color {
    Color {
        r: ((value >> 16) & 0xFF) as f32 / 255.0,
        g: ((value >> 8) & 0xFF) as f32 / 255.0,
        b: (value & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

pub const VOID: Color = hex(0x1a1c2c);
pub const PLUM: Color = hex(0x5d275d);
pub const CRIMSON: Color = hex(0xb13e53);
pub const EMBER: Color = hex(0xef7d57);
pub const GOLD: Color = hex(0xffcd75);
pub const LIME: Color = hex(0xa7f070);
pub const GRASS: Color = hex(0x38b764);
pub const TEAL: Color = hex(0x257179);
pub const NAVY: Color = hex(0x29366f);
pub const BLUE: Color = hex(0x3b5dc9);
pub const SKY: Color = hex(0x41a6f6);
pub const CYAN: Color = hex(0x73eff7);
pub const WHITE: Color = hex(0xf4f4f4);
pub const SILVER: Color = hex(0x94b0c2);
pub const SLATE: Color = hex(0x566c86);
pub const CHARCOAL: Color = hex(0x333c57);

/// Les rôles d'interface, pour que les écrans parlent d'intention plutôt que
/// de teinte : changer la palette ne demandera pas de relire chaque écran.
pub mod role {
    use super::*;

    /// Fond général et barres de letterbox.
    pub const BACKGROUND: Color = VOID;
    /// Intérieur d'un panneau.
    pub const PANEL: Color = CHARCOAL;
    /// Bordure extérieure d'un panneau ou d'un bouton.
    pub const BORDER: Color = VOID;
    /// Arête claire en haut à gauche, qui donne le relief 8-bit.
    pub const HIGHLIGHT: Color = SLATE;

    pub const TEXT: Color = WHITE;
    /// Titre d'écran.
    pub const TITLE: Color = GOLD;
    /// Aide mnémotechnique.
    pub const HINT: Color = GOLD;
    /// Signe encore mal su, à retravailler.
    pub const SHAKY: Color = EMBER;
    /// Texte secondaire : sous-titres, aides, unités.
    pub const TEXT_MUTED: Color = SILVER;
    /// Texte sur un élément désactivé.
    pub const TEXT_DISABLED: Color = SLATE;

    pub const ACCENT: Color = SKY;
    pub const SUCCESS: Color = GRASS;
    pub const DANGER: Color = CRIMSON;
    /// Étoiles gagnées et récompenses.
    pub const STAR: Color = GOLD;
    /// Étoiles non gagnées.
    pub const STAR_EMPTY: Color = SLATE;
}
