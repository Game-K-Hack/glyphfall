//! Le temps d'apprentissage du jour.
//!
//! Compte les secondes passées à apprendre : une manche en cours, ou la fiche
//! d'un signe que l'on étudie. Rien d'autre — ni les menus, ni le briefing, que
//! l'on peut laisser ouvert sans rien apprendre.

use macroquad::miniquad;
use serde::{Deserialize, Serialize};

use crate::storage;

const SAVE_NAME: &str = "daily.toml";
const FORMAT_VERSION: u32 = 1;

/// Secondes entre deux écritures sur disque.
///
/// Sauvegarder à chaque frame userait le disque pour rien ; ne sauvegarder
/// qu'à la fermeture perdrait la séance si le jeu est tué.
const SAVE_EVERY: f32 = 20.0;

#[derive(Debug, Serialize, Deserialize)]
pub struct Daily {
    #[serde(default)]
    version: u32,
    /// Jour compté depuis 1970. Sert à savoir quand tout remettre à zéro.
    #[serde(default)]
    day: i64,
    #[serde(default)]
    seconds: f32,
    /// L'alerte a-t-elle déjà été montrée aujourd'hui ?
    #[serde(default)]
    alerted: bool,

    /// Temps accumulé depuis la dernière écriture. Jamais sauvegardé.
    #[serde(skip)]
    since_save: f32,
}

impl Default for Daily {
    fn default() -> Self {
        Self { version: FORMAT_VERSION, day: today(), seconds: 0.0, alerted: false, since_save: 0.0 }
    }
}

impl Daily {
    pub fn load() -> Self {
        let Some(content) = storage::read(SAVE_NAME) else { return Self::default() };

        match toml::from_str::<Self>(&content) {
            Ok(daily) if daily.version == FORMAT_VERSION => daily.rolled_over(today()),
            _ => Self::default(),
        }
    }

    pub fn save(&self) {
        if let Ok(content) = toml::to_string(self) {
            storage::write(SAVE_NAME, &content);
        }
    }

    /// Ajoute du temps d'apprentissage.
    ///
    /// Le changement de jour est vérifié ici plutôt qu'au lancement : une
    /// séance qui traverse minuit doit repartir de zéro sans quitter le jeu.
    pub fn add(&mut self, dt: f32) {
        let today = today();
        if self.day != today {
            *self = Self { since_save: self.since_save, ..Self::default() };
        }

        self.seconds += dt;
        self.since_save += dt;

        if self.since_save >= SAVE_EVERY {
            self.since_save = 0.0;
            self.save();
        }
    }

    pub fn minutes(&self) -> u32 {
        (self.seconds / 60.0) as u32
    }

    /// L'objectif du jour est-il atteint, sans que l'alerte ait déjà été vue ?
    pub fn goal_reached(&self, goal_minutes: u32) -> bool {
        goal_minutes > 0 && !self.alerted && self.minutes() >= goal_minutes
    }

    /// Note que l'alerte a été montrée, pour qu'elle ne revienne pas demain
    /// matin ni cinq minutes plus tard.
    pub fn mark_alerted(&mut self) {
        self.alerted = true;
        self.save();
    }

    /// Remet le compteur à zéro si la sauvegarde date d'un autre jour.
    fn rolled_over(mut self, today: i64) -> Self {
        if self.day != today {
            self = Self::default();
        }
        self
    }
}

/// Le jour courant, compté depuis 1970.
///
/// La journée bascule à minuit UTC, soit une ou deux heures du matin en France.
/// C'est une approximation assumée — et plutôt heureuse ici : une séance tardive
/// compte encore pour la journée qu'on croit être en train de finir.
fn today() -> i64 {
    (miniquad::date::now() / 86_400.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: i64, seconds: f32) -> Daily {
        Daily { version: FORMAT_VERSION, day, seconds, alerted: false, since_save: 0.0 }
    }

    #[test]
    fn a_save_from_another_day_starts_over() {
        // Sans cela, le compteur d'hier ferait croire l'objectif atteint dès le
        // lancement du lendemain.
        let yesterday = at(19_000, 3_600.0).rolled_over(19_001);

        assert_eq!(yesterday.seconds, 0.0);
        assert!(!yesterday.alerted);
    }

    #[test]
    fn a_save_from_today_is_kept() {
        let same_day = at(19_000, 600.0).rolled_over(19_000);

        assert_eq!(same_day.seconds, 600.0);
    }

    #[test]
    fn the_goal_is_reached_at_the_exact_minute() {
        let mut daily = at(today(), 0.0);

        daily.seconds = 29.0 * 60.0;
        assert!(!daily.goal_reached(30));

        daily.seconds = 30.0 * 60.0;
        assert!(daily.goal_reached(30));
    }

    #[test]
    fn a_disabled_goal_never_alerts() {
        let daily = at(today(), 10_000.0);

        assert!(!daily.goal_reached(0));
    }

    #[test]
    fn the_alert_only_shows_once() {
        let mut daily = at(today(), 60.0 * 60.0);

        assert!(daily.goal_reached(30));
        daily.alerted = true;
        assert!(!daily.goal_reached(30), "l'alerte reviendrait toutes les frames");
    }

    #[test]
    fn crossing_midnight_resets_without_leaving_the_game() {
        // Une seance qui traverse minuit doit repartir de zero.
        let mut daily = at(today() - 1, 3_600.0);
        daily.alerted = true;

        daily.add(1.0);

        assert!(daily.seconds < 2.0, "le compteur d'hier doit disparaitre");
        assert!(!daily.alerted);
    }
}
