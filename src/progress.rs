//! La progression du joueur : combien d'étoiles pour chaque niveau, et donc
//! quels niveaux sont ouverts.
//!
//! Volontairement en mémoire pour l'instant ; la persistance sur disque et en
//! navigateur viendra se brancher derrière cette même interface.

use std::collections::HashMap;

use crate::data::{Language, Level};

/// Étoiles maximales pour un niveau.
pub const MAX_STARS: u8 = 3;

#[derive(Debug, Default)]
pub struct Progress {
    /// Identifiant de niveau vers le meilleur score en étoiles.
    best: HashMap<String, u8>,
}

impl Progress {
    pub fn new() -> Self {
        Self::default()
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
    fn a_zero_star_run_does_not_complete_the_level() {
        let mut progress = Progress::new();
        progress.record("ko-01", 0);

        assert!(!progress.is_completed("ko-01"));
    }
}
