//! Le modèle de données décrivant une langue et ses niveaux.
//!
//! Ces structures sont le miroir exact des fichiers TOML de `assets/languages/`.
//! Tout ce qui pilote le jeu vit ici : aucune valeur de gameplay ne doit être
//! codée en dur ailleurs dans le projet.

use serde::Deserialize;

/// Une écriture à apprendre : coréen, hiragana, katakana, kanji…
///
/// Chargée depuis `assets/languages/<dossier>/language.toml`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Language {
    /// Identifiant stable, utilisé par la sauvegarde de progression.
    pub id: String,
    /// Nom affiché dans la langue de l'interface (« Coréen »).
    pub name: String,
    /// Nom dans l'écriture elle-même (« 한국어 »), affiché en gros.
    pub native_name: String,
    #[serde(default)]
    pub description: String,
    /// Les polices capables de rendre cette écriture, dans `assets/fonts/`.
    ///
    /// Plusieurs, et volontairement différentes : un signe ne se reconnaît
    /// vraiment que lorsqu'on le reconnaît dans plusieurs tracés. La première
    /// sert de référence, les suivantes sont tirées au sort en jeu si le joueur
    /// l'a demandé. Vide = l'écriture latine, que la police d'interface couvre.
    #[serde(default)]
    pub fonts: Vec<String>,

    /// Rempli par le chargeur depuis `levels/*.toml`, jamais par le manifeste.
    #[serde(skip)]
    pub levels: Vec<Level>,
}

impl Language {
    pub fn level(&self, id: &str) -> Option<&Level> {
        self.levels.iter().find(|level| level.id == id)
    }
}

/// Le mode de jeu d'un niveau. Le champ existe dès maintenant pour que
/// l'ajout d'un second jeu ne demande pas de migrer tous les fichiers TOML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    /// Les tuiles tombent, le joueur tape la romanisation avant la ligne rouge.
    #[default]
    TileFall,
}

/// Une étape du chemin d'apprentissage.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Level {
    /// Identifiant unique dans tout le catalogue (préfixé par la langue).
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    /// Position sur le chemin ; sert aussi de tri d'affichage.
    pub order: u32,
    /// Niveaux à terminer avant d'ouvrir celui-ci. Vide = point de départ.
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub mode: GameMode,
    #[serde(default)]
    pub rules: Rules,
    pub stars: Stars,
    pub glyphs: Vec<Glyph>,
}

/// Les paramètres de difficulté d'un niveau.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    #[serde(default = "default_lives")]
    pub lives: u32,
    /// Durée de la manche en secondes. `0` = pas de limite de temps.
    #[serde(default)]
    pub duration: f32,
    #[serde(default = "default_columns")]
    pub columns: i32,
    /// Secondes entre deux apparitions de tuile.
    #[serde(default = "default_spawn_interval")]
    pub spawn_interval: f32,
    #[serde(default)]
    pub speed: Speed,
    /// Part des tirages puisée dans les niveaux prérequis, entre 0 et 1.
    /// C'est ce qui empêche d'oublier ce qu'on vient d'apprendre.
    #[serde(default)]
    pub review_ratio: f32,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            lives: default_lives(),
            duration: 0.0,
            columns: default_columns(),
            spawn_interval: default_spawn_interval(),
            speed: Speed::default(),
            review_ratio: 0.0,
        }
    }
}

impl Rules {
    pub fn is_timed(&self) -> bool {
        self.duration > 0.0
    }
}

/// La montée en difficulté : on part lentement et on accélère à chaque tuile.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Speed {
    /// Vitesse de chute initiale, en pixels virtuels par seconde.
    pub start: f32,
    /// Gain de vitesse par tuile apparue.
    pub ramp: f32,
    /// Plafond, pour que le niveau reste jouable.
    pub max: f32,
}

impl Default for Speed {
    fn default() -> Self {
        Self { start: 120.0, ramp: 4.0, max: 420.0 }
    }
}

impl Speed {
    /// Vitesse après `spawned` apparitions, plafonnée.
    pub fn at(&self, spawned: u32) -> f32 {
        (self.start + self.ramp * spawned as f32).min(self.max)
    }
}

/// Les seuils de précision donnant 1, 2 ou 3 étoiles.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stars {
    pub one: f32,
    pub two: f32,
    pub three: f32,
}

impl Stars {
    /// Note sur 3 pour une précision entre 0 et 1.
    pub fn rate(&self, accuracy: f32) -> u8 {
        if accuracy >= self.three {
            3
        } else if accuracy >= self.two {
            2
        } else if accuracy >= self.one {
            1
        } else {
            0
        }
    }
}

/// Un caractère à reconnaître et les réponses acceptées.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glyph {
    /// Le caractère lui-même (« ㄱ », « あ »…).
    pub char: String,
    /// Toutes les romanisations correctes. La première est celle qu'on montre
    /// au joueur ; les suivantes sont des variantes tolérées (« si » / « shi »).
    pub answers: Vec<String>,
    /// Comment retenir ce signe. Au moins un, souvent deux ; la fiche du signe
    /// les montre tous.
    ///
    /// Volontairement écrits sans caractère de l'écriture enseignée, pour
    /// qu'ils restent rendus par la police pixel — voir le test de couverture.
    pub mnemonics: Vec<String>,
}

impl Glyph {
    /// La saisie du joueur est-elle acceptée ? Insensible à la casse et aux
    /// espaces parasites : on note la connaissance, pas la dextérité.
    pub fn accepts(&self, input: &str) -> bool {
        let input = input.trim().to_lowercase();
        self.answers.iter().any(|answer| answer.to_lowercase() == input)
    }

    /// La romanisation de référence, montrée en briefing et en correction.
    pub fn primary_answer(&self) -> &str {
        self.answers.first().map(String::as_str).unwrap_or("")
    }
}

fn default_lives() -> u32 {
    3
}

fn default_columns() -> i32 {
    4
}

fn default_spawn_interval() -> f32 {
    1.2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stars_are_awarded_by_threshold() {
        let stars = Stars { one: 0.5, two: 0.75, three: 0.9 };

        assert_eq!(stars.rate(0.0), 0);
        assert_eq!(stars.rate(0.49), 0);
        assert_eq!(stars.rate(0.5), 1);
        assert_eq!(stars.rate(0.74), 1);
        assert_eq!(stars.rate(0.75), 2);
        assert_eq!(stars.rate(0.89), 2);
        assert_eq!(stars.rate(0.9), 3);
        assert_eq!(stars.rate(1.0), 3);
    }

    #[test]
    fn glyph_accepts_any_variant_ignoring_case_and_spaces() {
        let glyph = Glyph {
            char: "し".into(),
            answers: vec!["shi".into(), "si".into()],
            mnemonics: vec!["se prononce chi".into()],
        };

        assert!(glyph.accepts("shi"));
        assert!(glyph.accepts("si"));
        assert!(glyph.accepts("  SHI "));
        assert!(!glyph.accepts("sh"));
        assert!(!glyph.accepts(""));
        assert_eq!(glyph.primary_answer(), "shi");
    }

    #[test]
    fn speed_ramps_then_plateaus() {
        let speed = Speed { start: 100.0, ramp: 10.0, max: 150.0 };

        assert_eq!(speed.at(0), 100.0);
        assert_eq!(speed.at(3), 130.0);
        assert_eq!(speed.at(5), 150.0);
        assert_eq!(speed.at(500), 150.0, "la vitesse doit rester plafonnée");
    }

    #[test]
    fn omitted_rules_fall_back_to_playable_defaults() {
        let level: Level = toml::from_str(
            r#"
            id = "ko-01"
            title = "Consonnes"
            order = 1
            stars = { one = 0.5, two = 0.75, three = 0.9 }
            [[glyphs]]
            char = "ㄱ"
            answers = ["g"]
            mnemonics = ["un coin"]
            "#,
        )
        .expect("le TOML minimal doit être accepté");

        assert_eq!(level.mode, GameMode::TileFall);
        assert_eq!(level.rules.lives, 3);
        assert_eq!(level.rules.columns, 4);
        assert!(!level.rules.is_timed());
        assert!(level.requires.is_empty());
    }

    #[test]
    fn unknown_field_is_rejected_rather_than_silently_ignored() {
        // Une faute de frappe dans un TOML doit se voir au chargement, pas se
        // traduire par un niveau qui se comporte bizarrement en jeu.
        let error = toml::from_str::<Level>(
            r#"
            id = "ko-01"
            title = "Consonnes"
            order = 1
            lifes = 5
            stars = { one = 0.5, two = 0.75, three = 0.9 }
            [[glyphs]]
            char = "ㄱ"
            answers = ["g"]
            mnemonics = ["un coin"]
            "#,
        );

        assert!(error.is_err());
    }
}
