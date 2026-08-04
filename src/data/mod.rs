//! Le contenu pédagogique du jeu : langues, niveaux, glyphes.
//!
//! `model` décrit les données, `catalog` les valide, `loader` les lit depuis
//! `assets/languages/`.

pub mod catalog;
pub mod loader;
pub mod model;

pub use catalog::Catalog;
pub use loader::{font_bytes, load_catalog};
pub use model::{GameMode, Glyph, Language, Level, Rules, Stars};
