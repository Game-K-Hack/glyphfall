//! La progression du joueur : combien d'étoiles pour chaque niveau, et donc
//! quels niveaux sont ouverts.
//!
//! La sauvegarde passe par `storage`, qui masque la difference entre un
//! fichier sur le bureau et le stockage local du navigateur.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::data::{Language, Level};
use crate::storage;

/// Étoiles maximales pour un niveau.
pub const MAX_STARS: u8 = 3;

/// Nom du fichier de sauvegarde, ou clé de stockage en navigateur.
const SAVE_NAME: &str = "progress.toml";

/// Version du format de sauvegarde. Une sauvegarde d'une version inconnue est
/// ignorée plutôt que mal interprétée : mieux vaut repartir de zéro qu'ouvrir
/// des niveaux au hasard.
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default)]
    version: u32,
    /// Identifiant de niveau vers le meilleur score en étoiles.
    ///
    /// Une carte ordonnée, pour que le fichier de sauvegarde reste stable d'une
    /// écriture à l'autre et lisible à l'oeil.
    #[serde(default)]
    best: BTreeMap<String, u8>,
}

impl Progress {
    pub fn new() -> Self {
        Self { version: FORMAT_VERSION, ..Self::default() }
    }

    /// Relit la sauvegarde. Une sauvegarde absente, illisible ou d'un format
    /// inconnu donne une progression vide.
    pub fn load() -> Self {
        let Some(content) = storage::read(SAVE_NAME) else { return Self::new() };

        match toml::from_str::<Self>(&content) {
            Ok(progress) if progress.version == FORMAT_VERSION => progress,
            _ => Self::new(),
        }
    }

    /// Écrit la sauvegarde. Un échec est silencieux : ne plus pouvoir écrire ne
    /// doit pas interrompre une partie en cours.
    pub fn save(&self) {
        if let Ok(content) = toml::to_string(self) {
            storage::write(SAVE_NAME, &content);
        }
    }

    /// Le meilleur résultat obtenu sur ce niveau, 0 s'il n'a jamais été réussi.
    pub fn stars(&self, level_id: &str) -> u8 {
        self.best.get(level_id).copied().unwrap_or(0)
    }

    /// Un niveau compte comme terminé dès la première étoile : c'est ce qui
    /// ouvre la suite du chemin. Viser les trois étoiles reste facultatif.
    pub fn is_completed(&self, level_id: &str) -> bool {
        self.stars(level_id) > 0
    }

    /// Enregistre un résultat, sans jamais faire régresser le meilleur.
    ///
    /// Renvoie `true` si c'est un nouveau record, ce qui permet à l'écran de
    /// résultats de le signaler.
    pub fn record(&mut self, level_id: &str, stars: u8) -> bool {
        let stars = stars.min(MAX_STARS);
        let previous = self.stars(level_id);

        if stars > previous {
            self.best.insert(level_id.to_string(), stars);
            true
        } else {
            false
        }
    }

    /// Le niveau est-il jouable ? Il l'est quand tous ses prérequis sont faits.
    pub fn is_unlocked(&self, level: &Level) -> bool {
        level.requires.iter().all(|required| self.is_completed(required))
    }

    /// Étoiles gagnées et étoiles possibles pour une langue, pour l'affichage
    /// « 5 / 12 » du chemin d'apprentissage.
    pub fn language_stars(&self, language: &Language) -> (u32, u32) {
        let earned = language.levels.iter().map(|level| self.stars(&level.id) as u32).sum();
        let total = language.levels.len() as u32 * MAX_STARS as u32;

        (earned, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::model::{Glyph, Stars};

    fn level(id: &str, requires: &[&str]) -> Level {
        Level {
            id: id.to_string(),
            title: id.to_string(),
            subtitle: String::new(),
            order: 1,
            requires: requires.iter().map(|s| s.to_string()).collect(),
            mode: Default::default(),
            rules: Default::default(),
            stars: Stars { one: 0.5, two: 0.75, three: 0.9 },
            glyphs: vec![Glyph { char: "ㄱ".into(), answers: vec!["g".into()], hint: String::new() }],
        }
    }

    #[test]
    fn a_level_without_prerequisites_is_open_from_the_start() {
        let progress = Progress::new();

        assert!(progress.is_unlocked(&level("ko-01", &[])));
    }

    #[test]
    fn a_level_stays_locked_until_all_its_prerequisites_are_done() {
        let mut progress = Progress::new();
        let target = level("ko-03", &["ko-01", "ko-02"]);

        assert!(!progress.is_unlocked(&target));

        progress.record("ko-01", 3);
        assert!(!progress.is_unlocked(&target), "un seul prérequis ne suffit pas");

        progress.record("ko-02", 1);
        assert!(progress.is_unlocked(&target));
    }

    #[test]
    fn one_star_is_enough_to_open_the_next_level() {
        let mut progress = Progress::new();
        progress.record("ko-01", 1);

        assert!(progress.is_completed("ko-01"));
        assert!(progress.is_unlocked(&level("ko-02", &["ko-01"])));
    }

    #[test]
    fn a_worse_run_never_lowers_the_best_score() {
        let mut progress = Progress::new();

        assert!(progress.record("ko-01", 3));
        assert!(!progress.record("ko-01", 1), "ce n'est pas un nouveau record");
        assert_eq!(progress.stars("ko-01"), 3);
    }

    #[test]
    fn a_save_survives_a_round_trip() {
        let mut progress = Progress::new();
        progress.record("ko-01", 3);
        progress.record("hira-01", 1);

        let written = toml::to_string(&progress).expect("progression sérialisable");
        let read: Progress = toml::from_str(&written).expect("progression relisible");

        assert_eq!(read.stars("ko-01"), 3);
        assert_eq!(read.stars("hira-01"), 1);
        assert_eq!(read.version, FORMAT_VERSION);
    }

    #[test]
    fn a_save_from_an_unknown_format_is_ignored() {
        // Sans ce garde-fou, un futur format relu de travers pourrait ouvrir des
        // niveaux au hasard ou en refermer.
        let future = "version = 999\n\n[best]\n\"ko-01\" = 3\n";

        let parsed: Progress = toml::from_str(future).expect("TOML valide");

        assert_ne!(parsed.version, FORMAT_VERSION);
        assert_eq!(Progress::new().stars("ko-01"), 0, "on repart de zéro");
    }

    #[test]
    fn a_zero_star_run_does_not_complete_the_level() {
        let mut progress = Progress::new();
        progress.record("ko-01", 0);

        assert!(!progress.is_completed("ko-01"));
    }
}
