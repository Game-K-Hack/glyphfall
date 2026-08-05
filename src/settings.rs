//! Les réglages du joueur, pour l'instant les deux volumes.
//!
//! Ils sont sauvegardés à part de la progression : effacer sa progression pour
//! recommencer un alphabet ne doit pas remettre le son à fond.

use serde::{Deserialize, Serialize};

use crate::storage;

/// Nom du fichier de sauvegarde, ou clé de stockage en navigateur.
const SAVE_NAME: &str = "settings.toml";

/// Version du format. Un fichier d'une version inconnue est ignoré plutôt que
/// mal interprété.
const FORMAT_VERSION: u32 = 1;

/// Les volumes sont des crans entiers et non des flottants : une jauge à dix
/// segments se règle sans effort au clavier, là où un réglage continu obligerait
/// à viser.
pub const MAX_LEVEL: u8 = 10;

/// Les durées proposées pour l'objectif quotidien, en minutes.
///
/// Zéro désactive l'alerte. Des paliers plutôt qu'un réglage continu : personne
/// ne veut viser 37 minutes, et neuf crans se parcourent d'un geste.
pub const DAILY_GOALS: [u32; 9] = [0, 5, 10, 15, 20, 30, 60, 90, 120];

/// Le libellé d'une durée d'objectif.
pub fn goal_label(minutes: u32) -> String {
    match minutes {
        0 => "AUCUN".to_string(),
        minutes if minutes < 60 => format!("{minutes} MIN"),
        minutes if minutes % 60 == 0 => format!("{}H", minutes / 60),
        minutes => format!("{}H{:02}", minutes / 60, minutes % 60),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    version: u32,
    /// Volume de la musique des menus, de 0 à `MAX_LEVEL`.
    #[serde(default = "default_music")]
    pub music: u8,
    /// Volume de la musique pendant une manche.
    ///
    /// Séparé de celui des menus, et plus bas par défaut : en partie,
    /// l'information passe par les bruitages, qu'une musique trop forte
    /// couvrirait au moment précis où ils comptent.
    #[serde(default = "default_music_game")]
    pub music_game: u8,
    /// Volume des bruitages, de 0 à `MAX_LEVEL`.
    #[serde(default = "default_sfx")]
    pub sfx: u8,
    /// Minutes d'apprentissage visées par jour, `0` pour ne pas être alerté.
    ///
    /// `None` signifie que la question n'a jamais été posée — ce n'est pas la
    /// même chose que d'avoir répondu « désactivé », et c'est ce qui déclenche
    /// l'écran de réglage au premier lancement.
    #[serde(default)]
    pub daily_goal: Option<u32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: FORMAT_VERSION,
            music: default_music(),
            music_game: default_music_game(),
            sfx: default_sfx(),
            daily_goal: None,
        }
    }
}

impl Settings {
    /// Relit les réglages. Un fichier absent, illisible ou d'une version
    /// inconnue redonne les valeurs par défaut.
    pub fn load() -> Self {
        let Some(content) = storage::read(SAVE_NAME) else { return Self::default() };

        match toml::from_str::<Self>(&content) {
            Ok(settings) if settings.version == FORMAT_VERSION => settings.clamped(),
            _ => Self::default(),
        }
    }

    pub fn save(&self) {
        if let Ok(content) = toml::to_string(self) {
            storage::write(SAVE_NAME, &content);
        }
    }

    /// Le multiplicateur à appliquer au son, entre 0 et 1.
    pub fn music_gain(&self) -> f32 {
        gain(self.music)
    }

    /// Le volume de la musique pendant une manche.
    pub fn music_game_gain(&self) -> f32 {
        gain(self.music_game)
    }

    pub fn sfx_gain(&self) -> f32 {
        gain(self.sfx)
    }

    /// L'objectif du jour en minutes, `0` si l'alerte est coupée ou si la
    /// question n'a pas encore été posée.
    pub fn daily_goal_minutes(&self) -> u32 {
        self.daily_goal.unwrap_or(0)
    }

    /// Le cran correspondant dans `DAILY_GOALS`.
    pub fn daily_goal_step(&self) -> usize {
        let minutes = self.daily_goal_minutes();
        DAILY_GOALS.iter().position(|goal| *goal == minutes).unwrap_or(0)
    }

    /// Un fichier retouché à la main pourrait annoncer un volume de 200 ; on ne
    /// laisse pas cette valeur atteindre le moteur audio.
    fn clamped(mut self) -> Self {
        self.music = self.music.min(MAX_LEVEL);
        self.music_game = self.music_game.min(MAX_LEVEL);
        self.sfx = self.sfx.min(MAX_LEVEL);
        // Une durée inconnue vaut mieux ignorée que prise au mot : elle
        // n'aurait aucun cran sur le curseur.
        if self.daily_goal.is_some_and(|goal| !DAILY_GOALS.contains(&goal)) {
            self.daily_goal = None;
        }
        self
    }
}

fn gain(level: u8) -> f32 {
    level.min(MAX_LEVEL) as f32 / MAX_LEVEL as f32
}

fn default_music() -> u8 {
    6
}

fn default_music_game() -> u8 {
    4
}

fn default_sfx() -> u8 {
    8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_level_maps_to_a_gain_between_zero_and_one() {
        let mut settings = Settings::default();

        settings.music = 0;
        assert_eq!(settings.music_gain(), 0.0);

        settings.music = MAX_LEVEL;
        assert_eq!(settings.music_gain(), 1.0);

        settings.music = 5;
        assert_eq!(settings.music_gain(), 0.5);
    }

    #[test]
    fn the_round_is_quieter_than_the_menus_by_default() {
        // Les bruitages portent l'information pendant une partie.
        let settings = Settings::default();

        assert!(settings.music_game < settings.music);
    }

    #[test]
    fn settings_survive_a_round_trip() {
        let settings = Settings {
            version: FORMAT_VERSION,
            music: 3,
            music_game: 2,
            sfx: 9,
            daily_goal: Some(30),
        };

        let written = toml::to_string(&settings).expect("réglages sérialisables");
        let read: Settings = toml::from_str(&written).expect("réglages relisibles");

        assert_eq!(read.music, 3);
        assert_eq!(read.music_game, 2);
        assert_eq!(read.sfx, 9);
        assert_eq!(read.daily_goal, Some(30));
    }

    #[test]
    fn a_hand_edited_file_cannot_push_the_volume_past_the_maximum() {
        let parsed: Settings = toml::from_str("version = 1\nmusic = 200\nsfx = 42\nmusic_game = 99\n")
            .expect("TOML valide");

        let settings = parsed.clamped();

        assert_eq!(settings.music, MAX_LEVEL);
        assert_eq!(settings.sfx, MAX_LEVEL);
        assert_eq!(settings.music_game, MAX_LEVEL);
        assert_eq!(settings.music_gain(), 1.0);
    }

    #[test]
    fn a_partial_file_keeps_the_defaults_for_what_is_missing() {
        // Un fichier écrit par une version plus ancienne peut ne pas avoir tous
        // les champs ; ce qui manque ne doit pas valoir zéro.
        let settings: Settings = toml::from_str("version = 1\nmusic = 2\n").expect("TOML valide");

        assert_eq!(settings.music, 2);
        assert_eq!(settings.sfx, default_sfx());
    }
}

#[cfg(test)]
mod goal_tests {
    use super::*;

    #[test]
    fn durations_read_the_way_one_says_them() {
        assert_eq!(goal_label(0), "AUCUN");
        assert_eq!(goal_label(5), "5 MIN");
        assert_eq!(goal_label(30), "30 MIN");
        assert_eq!(goal_label(60), "1H");
        assert_eq!(goal_label(90), "1H30");
        assert_eq!(goal_label(120), "2H");
    }

    #[test]
    fn never_answered_is_not_the_same_as_disabled() {
        // C'est cette distinction qui declenche la question au premier
        // lancement, sans la reposer a quelqu'un qui a repondu « desactive ».
        let untouched = Settings::default();
        assert!(untouched.daily_goal.is_none());
        assert_eq!(untouched.daily_goal_minutes(), 0);

        let disabled = Settings { daily_goal: Some(0), ..Settings::default() };
        assert!(disabled.daily_goal.is_some());
        assert_eq!(disabled.daily_goal_minutes(), 0);
    }

    #[test]
    fn every_step_maps_back_to_itself() {
        for (index, minutes) in DAILY_GOALS.iter().enumerate() {
            let settings = Settings { daily_goal: Some(*minutes), ..Settings::default() };
            assert_eq!(settings.daily_goal_step(), index);
        }
    }

    #[test]
    fn a_hand_edited_duration_outside_the_steps_is_dropped() {
        // Sans cela le curseur n'aurait aucune position a montrer.
        let parsed = Settings { daily_goal: Some(37), ..Settings::default() }.clamped();

        assert_eq!(parsed.daily_goal, None);
    }
}
