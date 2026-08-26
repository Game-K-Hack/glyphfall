//! La progression du joueur : combien d'étoiles pour chaque niveau, et donc
//! quels niveaux sont ouverts.
//!
//! La sauvegarde passe par `storage`, qui masque la difference entre un
//! fichier sur le bureau et le stockage local du navigateur.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::data::{Catalog, Language, Level};
use crate::session::Mode;
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
    /// Maîtrise de chaque signe, entre `WEAKEST` et `STRONGEST`, rangée par
    /// alphabet puis par signe.
    ///
    /// Rangée par alphabet parce qu'un signe n'existe pas en dehors du sien :
    /// mélanger le coréen et le japonais dans une même table rendait impossible
    /// de dire ce qui a été appris d'une écriture donnée.
    ///
    /// Dans un alphabet, l'index reste le signe et non le niveau : un signe
    /// appris à l'étape 2 et revu à l'étape 9 est le même signe.
    #[serde(default)]
    signs: BTreeMap<String, BTreeMap<String, i8>>,

    /// Ce que les modes rapides ont donné, par niveau.
    ///
    /// Séparé de `best` parce qu'il ne s'agit pas de la même monnaie : `best`
    /// compte des étoiles gagnées à la précision, ceci retient des sans-faute
    /// et un record.
    #[serde(default)]
    modes: BTreeMap<String, LevelModes>,

    /// L'ancienne table, toutes écritures confondues.
    ///
    /// Conservée le temps de la relire une fois, puis vidée. Sans elle, la
    /// sauvegarde d'une version précédente serait refusée en bloc et le joueur
    /// perdrait aussi ses étoiles.
    #[serde(default, rename = "mastery", skip_serializing_if = "BTreeMap::is_empty")]
    legacy_mastery: BTreeMap<String, i8>,
}

/// Ce qu'un niveau a rendu dans ses modes au-delà du normal.
#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct LevelModes {
    /// Le mode rapide a été bouclé sans la moindre faute.
    #[serde(default)]
    pub fast_perfect: bool,
    /// Le mode ultra aussi.
    #[serde(default)]
    pub ultra_perfect: bool,
    /// Meilleur score en mode infini. Il n'y a rien d'autre à y gagner.
    #[serde(default)]
    pub endless_best: u32,
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
    pub fn mastery(&self, language_id: &str, character: &str) -> i8 {
        self.signs
            .get(language_id)
            .and_then(|signs| signs.get(character))
            .copied()
            .unwrap_or(0)
    }

    /// Ce signe est-il encore fragile ?
    pub fn is_shaky(&self, language_id: &str, character: &str) -> bool {
        self.mastery(language_id, character) < 0
    }

    /// Poids de tirage d'un signe : plus il est mal su, plus il revient.
    ///
    /// C'est le coeur de la révision. Tirer uniformément ferait revenir aussi
    /// souvent un signe acquis depuis dix étapes qu'un signe raté hier.
    pub fn draw_weight(&self, language_id: &str, character: &str) -> u32 {
        (STRONGEST - self.mastery(language_id, character) + 1) as u32
    }

    /// Enregistre le bilan d'un signe sur une manche.
    pub fn note(&mut self, language_id: &str, character: &str, hits: u32, misses: u32) {
        let delta = hits as i32 * HIT_GAIN as i32 - misses as i32 * MISS_COST as i32;
        if delta == 0 {
            return;
        }

        let updated = (self.mastery(language_id, character) as i32 + delta)
            .clamp(WEAKEST as i32, STRONGEST as i32) as i8;

        self.signs
            .entry(language_id.to_string())
            .or_default()
            .insert(character.to_string(), updated);
    }

    /// Les signes déjà rencontrés dans cet alphabet.
    ///
    /// Un signe entre dans cette liste dès sa première apparition, réussie ou
    /// non : c'est ce qui a été *vu*, et donc ce qu'il y a à réviser.
    pub fn learned_signs(&self, language_id: &str) -> Vec<&str> {
        self.signs
            .get(language_id)
            .map(|signs| signs.keys().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Reprend l'ancienne table à plat et range chaque signe sous son alphabet.
    ///
    /// Le catalogue est nécessaire : rien dans l'ancienne sauvegarde ne dit à
    /// quelle écriture appartient un signe. Renvoie `true` si quelque chose a
    /// été déplacé, auquel cas il faut réécrire le fichier.
    pub fn migrate(&mut self, catalog: &Catalog) -> bool {
        if self.legacy_mastery.is_empty() {
            return false;
        }

        for (character, score) in std::mem::take(&mut self.legacy_mastery) {
            let owner = catalog.languages.iter().find(|language| {
                language
                    .levels
                    .iter()
                    .any(|level| level.glyphs.iter().any(|glyph| glyph.char == character))
            });

            // Un signe qui n'appartient plus à aucune écriture est abandonné :
            // le garder sans savoir où le ranger ne servirait à personne.
            if let Some(language) = owner {
                self.signs
                    .entry(language.id.clone())
                    .or_default()
                    .insert(character, score);
            }
        }

        true
    }

    /// Ce que les modes ont donné sur ce niveau.
    pub fn modes(&self, level_id: &str) -> LevelModes {
        self.modes.get(level_id).copied().unwrap_or_default()
    }

    /// Ce mode est-il ouvert sur ce niveau ?
    ///
    /// Chaque mode s'ouvre en maîtrisant le précédent : trois étoiles au
    /// normal, puis un sans-faute à chaque étage. On ne saute pas de marche.
    pub fn mode_unlocked(&self, level_id: &str, mode: Mode) -> bool {
        let modes = self.modes(level_id);

        match mode {
            Mode::Normal => true,
            Mode::Fast => self.stars(level_id) >= MAX_STARS,
            Mode::Ultra => modes.fast_perfect,
            Mode::Endless => modes.ultra_perfect,
        }
    }

    /// Ce que le joueur doit faire pour ouvrir ce mode.
    pub fn unlock_requirement(mode: Mode) -> &'static str {
        match mode {
            Mode::Normal => "",
            Mode::Fast => "3 ETOILES EN NORMAL",
            Mode::Ultra => "SANS FAUTE EN RAPIDE",
            Mode::Endless => "SANS FAUTE EN ULTRA",
        }
    }

    /// Ce mode a-t-il déjà été maîtrisé sur ce niveau ?
    pub fn mode_mastered(&self, level_id: &str, mode: Mode) -> bool {
        let modes = self.modes(level_id);

        match mode {
            Mode::Normal => self.stars(level_id) >= MAX_STARS,
            Mode::Fast => modes.fast_perfect,
            Mode::Ultra => modes.ultra_perfect,
            // L'infini ne se maîtrise pas, il se pousse.
            Mode::Endless => false,
        }
    }

    /// Enregistre une manche jouée dans un mode au-delà du normal.
    ///
    /// Renvoie `true` si quelque chose de nouveau a été décroché : une étoile
    /// de couleur, ou un record en infini.
    pub fn record_mode(&mut self, level_id: &str, mode: Mode, perfect: bool, score: u32) -> bool {
        let entry = self.modes.entry(level_id.to_string()).or_default();

        match mode {
            // Le normal passe par `record`, qui compte en étoiles.
            Mode::Normal => false,
            Mode::Fast if perfect && !entry.fast_perfect => {
                entry.fast_perfect = true;
                true
            }
            Mode::Ultra if perfect && !entry.ultra_perfect => {
                entry.ultra_perfect = true;
                true
            }
            Mode::Endless if score > entry.endless_best => {
                entry.endless_best = score;
                true
            }
            _ => false,
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
            glyphs: vec![Glyph {
                char: "ㄱ".into(),
                answers: vec!["g".into()],
                name: String::new(),
                pronunciation: String::new(),
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
    fn a_mode_opens_only_once_the_previous_one_is_mastered() {
        // On ne saute pas de marche : chaque etage demande le precedent.
        let mut progress = Progress::new();

        assert!(progress.mode_unlocked("ko-01", Mode::Normal), "le normal est toujours ouvert");
        assert!(!progress.mode_unlocked("ko-01", Mode::Fast));

        progress.record("ko-01", 2);
        assert!(!progress.mode_unlocked("ko-01", Mode::Fast), "deux etoiles ne suffisent pas");

        progress.record("ko-01", 3);
        assert!(progress.mode_unlocked("ko-01", Mode::Fast));
        assert!(!progress.mode_unlocked("ko-01", Mode::Ultra));

        progress.record_mode("ko-01", Mode::Fast, true, 0);
        assert!(progress.mode_unlocked("ko-01", Mode::Ultra));
        assert!(!progress.mode_unlocked("ko-01", Mode::Endless));

        progress.record_mode("ko-01", Mode::Ultra, true, 0);
        assert!(progress.mode_unlocked("ko-01", Mode::Endless));
    }

    #[test]
    fn only_a_flawless_round_earns_a_coloured_star() {
        // Une manche presque parfaite reste une manche a refaire.
        let mut progress = Progress::new();

        assert!(!progress.record_mode("ko-01", Mode::Fast, false, 0));
        assert!(!progress.modes("ko-01").fast_perfect);

        assert!(progress.record_mode("ko-01", Mode::Fast, true, 0));
        assert!(progress.modes("ko-01").fast_perfect);
    }

    #[test]
    fn an_already_earned_star_is_not_earned_twice() {
        let mut progress = Progress::new();
        progress.record_mode("ko-01", Mode::Fast, true, 0);

        assert!(!progress.record_mode("ko-01", Mode::Fast, true, 0), "plus rien de neuf");
    }

    #[test]
    fn the_endless_mode_only_keeps_the_best_score() {
        let mut progress = Progress::new();

        assert!(progress.record_mode("ko-01", Mode::Endless, false, 300));
        assert_eq!(progress.modes("ko-01").endless_best, 300);

        assert!(!progress.record_mode("ko-01", Mode::Endless, false, 120), "moins bien");
        assert_eq!(progress.modes("ko-01").endless_best, 300);

        assert!(progress.record_mode("ko-01", Mode::Endless, false, 900));
        assert_eq!(progress.modes("ko-01").endless_best, 900);
    }

    #[test]
    fn modes_survive_a_round_trip() {
        let mut progress = Progress::new();
        progress.record_mode("ko-01", Mode::Fast, true, 0);
        progress.record_mode("ko-01", Mode::Endless, false, 750);

        let written = toml::to_string(&progress).expect("progression serialisable");
        let read: Progress = toml::from_str(&written).expect("progression relisible");

        assert!(read.modes("ko-01").fast_perfect);
        assert!(!read.modes("ko-01").ultra_perfect);
        assert_eq!(read.modes("ko-01").endless_best, 750);
    }

    #[test]
    fn an_older_save_simply_has_no_modes_yet() {
        // Les sauvegardes d'avant les modes doivent rester lisibles telles
        // quelles, sans rien perdre.
        let old = "version = 1\n\n[best]\n\"ko-01\" = 3\n";

        let progress: Progress = toml::from_str(old).expect("ancienne sauvegarde relisible");

        assert_eq!(progress.stars("ko-01"), 3);
        assert!(progress.mode_unlocked("ko-01", Mode::Fast), "les trois etoiles comptent");
        assert!(!progress.modes("ko-01").fast_perfect);
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
        progress.note("ko", "\u{3131}", 3, 0);
        progress.note("ko", "\u{3134}", 0, 1);

        assert!(
            progress.draw_weight("ko", "\u{3134}") > progress.draw_weight("ko", "\u{3131}"),
            "le signe rate doit peser plus lourd dans le tirage"
        );
        assert!(progress.is_shaky("ko", "\u{3134}"));
        assert!(!progress.is_shaky("ko", "\u{3131}"));
    }

    #[test]
    fn a_mistake_costs_more_than_a_success_earns() {
        // Une manche a deux tiers de reussite ne doit pas consolider un signe.
        let mut progress = Progress::new();

        progress.note("ko", "\u{3131}", 2, 1);

        assert_eq!(progress.mastery("ko", "\u{3131}"), 0, "deux bonnes et une ratee s'annulent");
    }

    #[test]
    fn mastery_stays_within_its_bounds() {
        // Sans bornes, un signe travaille cent fois deviendrait impossible a
        // faire ressortir en revision.
        let mut progress = Progress::new();

        progress.note("ko", "\u{3131}", 100, 0);
        assert_eq!(progress.mastery("ko", "\u{3131}"), STRONGEST);
        assert_eq!(progress.draw_weight("ko", "\u{3131}"), 1, "un signe acquis garde une chance");

        progress.note("ko", "\u{3134}", 0, 100);
        assert_eq!(progress.mastery("ko", "\u{3134}"), WEAKEST);
    }

    #[test]
    fn an_unseen_sign_sits_in_the_middle() {
        let progress = Progress::new();

        assert_eq!(progress.mastery("ko", "\u{3131}"), 0);
        assert!(!progress.is_shaky("ko", "\u{3131}"), "jamais vu n'est pas fragile, juste inconnu");
    }

    #[test]
    fn mastery_survives_a_round_trip() {
        let mut progress = Progress::new();
        progress.note("ko", "\u{3131}", 4, 0);
        progress.note("ko", "\u{3134}", 0, 2);

        let written = toml::to_string(&progress).expect("progression serialisable");
        let read: Progress = toml::from_str(&written).expect("progression relisible");

        assert_eq!(read.mastery("ko", "\u{3131}"), 4);
        assert_eq!(read.mastery("ko", "\u{3134}"), -4);
    }

    #[test]
    fn a_zero_star_run_does_not_complete_the_level() {
        let mut progress = Progress::new();
        progress.record("ko-01", 0);

        assert!(!progress.is_completed("ko-01"));
    }
}
