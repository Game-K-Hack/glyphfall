//! Le rendu 8-bit : une toile virtuelle agrandie en pixels carrés, une palette
//! de seize couleurs, une police pixel et des briques d'interface.
//!
//! Aucun écran ne doit dessiner en dehors de ces outils, ni manipuler de
//! couleur ou de coordonnée en pixels d'écran.

pub mod canvas;
pub mod fonts;
pub mod palette;
pub mod ui;

pub use canvas::Canvas;
pub use fonts::Fonts;
