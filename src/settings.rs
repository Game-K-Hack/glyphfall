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

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    version: u32,
    /// Volume de la musique de fond, de 0 à `MAX_LEVEL`.
    #[serde(default = "default_music")]
    pub music: u8,
    /// Volume des bruitages, de 0 à `MAX_LEVEL`.
    #[serde(default = "default_sfx")]
    pub sfx: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self { version: FORMAT_VERSION, music: default_music(), sfx: default_sfx() }
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

    pub fn sfx_gain(&self) -> f32 {
        gain(self.sfx)
    }

    /// Un fichier retouché à la main pourrait annoncer un volume de 200 ; on ne
    /// laisse pas cette valeur atteindre le moteur audio.
    fn clamped(mut self) -> Self {
        self.music = self.music.min(MAX_LEVEL);
        self.sfx = self.sfx.min(MAX_LEVEL);
        self
    }
}

fn gain(level: u8) -> f32 {
    level.min(MAX_LEVEL) as f32 / MAX_LEVEL as f32
}

fn default_music() -> u8 {
    6
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
    fn settings_survive_a_round_trip() {
        let settings = Settings { version: FORMAT_VERSION, music: 3, sfx: 9 };

        let written = toml::to_string(&settings).expect("réglages sérialisables");
        let read: Settings = toml::from_str(&written).expect("réglages relisibles");

        assert_eq!(read.music, 3);
        assert_eq!(read.sfx, 9);
    }

    #[test]
    fn a_hand_edited_file_cannot_push_the_volume_past_the_maximum() {
        let parsed: Settings =
            toml::from_str("version = 1\nmusic = 200\nsfx = 42\n").expect("TOML valide");

        let settings = parsed.clamped();

        assert_eq!(settings.music, MAX_LEVEL);
        assert_eq!(settings.sfx, MAX_LEVEL);
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
