//! Les polices : une police pixel pour l'interface, une police par écriture
//! pour les glyphes à apprendre.
//!
//! Les deux sont rendues en filtrage « au plus proche ». La police pixel est
//! dessinée sur une grille de 8 px : l'afficher à un multiple de 8 la rend
//! parfaitement nette, toute autre taille la déforme.

use std::collections::HashMap;

use macroquad::prelude::*;

use crate::data::{Catalog, font_bytes};

/// Police de l'interface. Le hangeul et les kana en sont absents : ils passent
/// par la police de leur langue.
const UI_FONT: &str = "PressStart2P-Regular.ttf";

/// Taille de base du texte d'interface, en pixels virtuels.
pub const TEXT: u16 = 8;
/// Taille des titres d'écran.
pub const TITLE: u16 = 16;

pub struct Fonts {
    pub ui: Font,
    /// Indexées par nom de fichier : plusieurs langues partagent une police.
    by_file: HashMap<String, Font>,
    /// Identifiant de langue vers ses noms de fichiers, dans l'ordre déclaré.
    by_language: HashMap<String, Vec<String>>,
}

impl Fonts {
    /// Charge la police d'interface et celles déclarées par les langues.
    ///
    /// Une langue dont la police est absente ou illisible retombe sur la police
    /// d'interface : mieux vaut du tofu à l'écran qu'un jeu qui refuse de
    /// démarrer parce qu'une écriture secondaire est mal configurée.
    pub fn load(catalog: &Catalog) -> Self {
        let ui = load(UI_FONT).unwrap_or_else(|| panic!("{UI_FONT} est introuvable ou illisible"));

        let mut by_file: HashMap<String, Font> = HashMap::new();
        let mut by_language: HashMap<String, Vec<String>> = HashMap::new();

        for language in &catalog.languages {
            let mut loaded = Vec::new();

            for file in &language.fonts {
                if !by_file.contains_key(file.as_str()) {
                    match load(file) {
                        Some(font) => {
                            by_file.insert(file.clone(), font);
                        }
                        // Une police absente est simplement sautée : les autres
                        // suffisent, et le jeu ne doit pas refuser de démarrer
                        // pour une variante manquante.
                        None => continue,
                    }
                }
                loaded.push(file.clone());
            }

            by_language.insert(language.id.clone(), loaded);
        }

        Self { ui, by_file, by_language }
    }

    /// La police de référence de cette écriture, la première déclarée.
    pub fn script(&self, language_id: &str) -> &Font {
        self.script_variant(language_id, 0)
    }

    /// L'une des polices de cette écriture, choisie par son rang.
    ///
    /// Le rang est ramené dans les bornes plutôt que refusé : une manche tire
    /// des rangs au hasard et ne doit pas avoir à connaître leur nombre.
    pub fn script_variant(&self, language_id: &str, index: usize) -> &Font {
        let files = self.by_language.get(language_id);
        let count = files.map(Vec::len).unwrap_or(0);
        if count == 0 {
            return &self.ui;
        }

        files
            .and_then(|files| files.get(index % count))
            .and_then(|file| self.by_file.get(file))
            .unwrap_or(&self.ui)
    }

    /// Combien de tracés différents existent pour cette écriture.
    pub fn script_count(&self, language_id: &str) -> usize {
        self.by_language.get(language_id).map(Vec::len).unwrap_or(0).max(1)
    }
}

/// Le nom de fichier de la police d'interface, pour le test qui verifie
/// qu'elle sait dessiner tous les textes du catalogue.
#[cfg_attr(not(test), allow(dead_code))]
pub const UI_FONT_FILE: &str = UI_FONT;

fn load(file_name: &str) -> Option<Font> {
    let mut font = load_ttf_font_from_bytes(font_bytes(file_name)?).ok()?;
    // Sans cela, macroquad lisserait l'atlas et les pixels deviendraient flous.
    font.set_filter(FilterMode::Nearest);
    Some(font)
}
