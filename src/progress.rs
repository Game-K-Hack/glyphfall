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

/// Un signe jamais rencontré part de zéro ; les bornes encadrent ce que la
/// maîtrise peut valoir.
const WEAKEST: i8 = -4;
const STRONGEST: i8 = 4;

/// Une erreur pèse plus lourd qu'une réussite.
///
/// Sans cette asymétrie, un signe raté une fois sur trois finirait par passer
/// pour acquis alors qu'il ne l'est pas.
const HIT_GAIN: i8 = 1;
const MISS_COST: i8 = 2;

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
    /// Maîtrise de chaque signe, entre `WEAKEST` et `STRONGEST`.
    ///
    /// Indexée par le signe lui-même et non par niveau : un signe appris à
    /// l'étape 2 et revu à l'étape 9 est le même signe, et c'est bien sa
    /// solidité que l'on veut suivre.
    #[serde(default)]
    mastery: BTreeMap<String, i8>,
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

    /// La solidité d'un signe. Zéro pour un signe jamais rencontré.
    pub fn mastery(&self, character: &str) -> i8 {
        self.mastery.get(character).copied().unwrap_or(0)
    }

    /// Ce signe est-il encore fragile ?
    pub fn is_shaky(&self, character: &str) -> bool {
        self.mastery(character) < 0
    }

    /// Poids de tirage d'un signe : plus il est mal su, plus il revient.
    ///
    /// C'est le coeur de la révision. Tirer uniformément ferait revenir aussi
    /// souvent un signe acquis depuis dix étapes qu'un signe raté hier.
    pub fn draw_weight(&self, character: &str) -> u32 {
        (STRONGEST - self.mastery(character) + 1) as u32
    }

    /// Enregistre le bilan d'un signe sur une manche.
    pub fn note(&mut self, character: &str, hits: u32, misses: u32) {
        let delta = hits as i32 * HIT_GAIN as i32 - misses as i32 * MISS_COST as i32;
        if delta == 0 {
            return;
        }

        let updated =
            (self.mastery(character) as i32 + delta).clamp(WEAKEST as i32, STRONGEST as i32) as i8;
        self.mastery.insert(character.to_string(), updated);
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
            glyphs: vec![Glyph {
                char: "ㄱ".into(),
                answers: vec!["g".into()],
                mnemonics: vec!["un coin".into()],
            }],
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
    fn a_missed_sign_comes_back_more_often_than_a_known_one() {
        let mut progress = Progress::new();

        // Trois reussites d'affilee sur le premier, un rate sur le second.
        progress.note("\u{3131}", 3, 0);
        progress.note("\u{3134}", 0, 1);

        assert!(
            progress.draw_weight("\u{3134}") > progress.draw_weight("\u{3131}"),
            "le signe rate doit peser plus lourd dans le tirage"
        );
        assert!(progress.is_shaky("\u{3134}"));
        assert!(!progress.is_shaky("\u{3131}"));
    }

    #[test]
    fn a_mistake_costs_more_than_a_success_earns() {
        // Une manche a deux tiers de reussite ne doit pas consolider un signe.
        let mut progress = Progress::new();

        progress.note("\u{3131}", 2, 1);

        assert_eq!(progress.mastery("\u{3131}"), 0, "deux bonnes et une ratee s'annulent");
    }

    #[test]
    fn mastery_stays_within_its_bounds() {
        // Sans bornes, un signe travaille cent fois deviendrait impossible a
        // faire ressortir en revision.
        let mut progress = Progress::new();

        progress.note("\u{3131}", 100, 0);
        assert_eq!(progress.mastery("\u{3131}"), STRONGEST);
        assert_eq!(progress.draw_weight("\u{3131}"), 1, "un signe acquis garde une chance");

        progress.note("\u{3134}", 0, 100);
        assert_eq!(progress.mastery("\u{3134}"), WEAKEST);
    }

    #[test]
    fn an_unseen_sign_sits_in_the_middle() {
        let progress = Progress::new();

        assert_eq!(progress.mastery("\u{3131}"), 0);
        assert!(!progress.is_shaky("\u{3131}"), "jamais vu n'est pas fragile, juste inconnu");
    }

    #[test]
    fn mastery_survives_a_round_trip() {
        let mut progress = Progress::new();
        progress.note("\u{3131}", 4, 0);
        progress.note("\u{3134}", 0, 2);

        let written = toml::to_string(&progress).expect("progression serialisable");
        let read: Progress = toml::from_str(&written).expect("progression relisible");

        assert_eq!(read.mastery("\u{3131}"), 4);
        assert_eq!(read.mastery("\u{3134}"), -4);
    }

    #[test]
    fn a_zero_star_run_does_not_complete_the_level() {
        let mut progress = Progress::new();
        progress.record("ko-01", 0);

        assert!(!progress.is_completed("ko-01"));
    }
}
