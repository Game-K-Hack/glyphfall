//! Une manche en cours.
//!
//! Tout ce qui décide du déroulement — vies, durée, colonnes, vitesse, tirage
//! des glyphes, seuils d'étoiles — vient du fichier TOML du niveau. Rien n'est
//! codé en dur ici : ajouter une langue ou régler une difficulté ne doit jamais
//! demander de toucher au code.

use macroquad::prelude::*;

use crate::data::{Catalog, GameMode, Glyph, Rules, Stars};

/// Hauteur d'une tuile, en pixels virtuels.
pub const TILE_HEIGHT: f32 = 40.0;
/// Largeur totale de la zone de jeu, partagée entre les colonnes.
pub const PLAYFIELD_WIDTH: f32 = 192.0;
/// La ligne de validation : une tuile qui la franchit est perdue.
pub const TARGET_Y: f32 = 160.0;

/// Points gagnés par glyphe reconnu.
const POINTS_PER_HIT: u32 = 10;
/// Durée du flash vert d'une tuile validée, en secondes.
const CLEAR_FLASH: f32 = 0.15;

pub struct Tile {
    pub column: i32,
    pub y: f32,
    pub glyph: Glyph,
    /// Décompte du flash de validation ; `None` tant que la tuile tombe.
    pub cleared: Option<f32>,
}

impl Tile {
    fn is_cleared(&self) -> bool {
        self.cleared.is_some()
    }
}

pub struct Session {
    pub language_id: String,
    pub level_id: String,
    pub level_title: String,

    /// Copiées depuis le niveau : la manche ne doit pas garder d'emprunt sur le
    /// catalogue, qui vit dans `App` à côté de la pile d'écrans.
    pub rules: Rules,
    stars: Stars,
    /// Les glyphes que ce niveau enseigne.
    glyphs: Vec<Glyph>,
    /// Ceux des niveaux prérequis, piochés selon `review_ratio`.
    review: Vec<Glyph>,

    pub tiles: Vec<Tile>,
    pub score: u32,
    pub lives: u32,
    pub input: String,
    /// Temps restant ; ignoré si le niveau n'est pas chronométré.
    pub time_left: f32,

    spawn_timer: f32,
    spawned: u32,

    hits: u32,
    /// Tuiles tombées sans avoir été reconnues.
    missed: u32,
    /// Validations qui ne correspondaient à aucune tuile.
    wrong: u32,
    /// Les glyphes ratés, pour les rappeler à la fin.
    missed_glyphs: Vec<String>,
}

/// Pourquoi la manche s'est arrêtée.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    OutOfLives,
    TimeUp,
}

/// Le bilan d'une manche terminée.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub language_id: String,
    pub level_id: String,
    pub level_title: String,
    pub score: u32,
    pub hits: u32,
    pub accuracy: f32,
    pub stars: u8,
    pub reason: EndReason,
    /// Renseigne par la boucle principale : ce resultat bat-il le precedent ?
    pub is_record: bool,
    /// Les glyphes ratés, sans doublon, pour la correction de fin.
    pub missed_glyphs: Vec<String>,
}

impl Session {
    /// Prépare une manche. `None` si le niveau n'existe pas dans le catalogue.
    pub fn new(catalog: &Catalog, language_id: &str, level_id: &str) -> Option<Self> {
        let language = catalog.language(language_id)?;
        let level = language.level(level_id)?;

        // Un seul mode pour l'instant : ce `match` obligera à traiter les
        // suivants le jour où ils apparaîtront.
        match level.mode {
            GameMode::TileFall => {}
        }

        // La révision pioche dans les prérequis directs : ce que l'on vient
        // d'apprendre, pas tout l'historique de la langue.
        let review = level
            .requires
            .iter()
            .filter_map(|required| language.level(required))
            .flat_map(|required| required.glyphs.iter().cloned())
            .collect();

        Some(Self {
            language_id: language_id.to_string(),
            level_id: level_id.to_string(),
            level_title: level.title.clone(),
            rules: level.rules.clone(),
            stars: level.stars,
            glyphs: level.glyphs.clone(),
            review,
            tiles: Vec::new(),
            score: 0,
            lives: level.rules.lives,
            input: String::new(),
            time_left: level.rules.duration,
            spawn_timer: 0.0,
            spawned: 0,
            hits: 0,
            missed: 0,
            wrong: 0,
            missed_glyphs: Vec::new(),
        })
    }

    /// Largeur d'une colonne, arrondie pour rester sur des pixels entiers.
    pub fn tile_width(&self) -> f32 {
        (PLAYFIELD_WIDTH / self.rules.columns as f32).floor()
    }

    /// Bord gauche de la zone de jeu, centrée sur la toile.
    pub fn playfield_x(&self) -> f32 {
        ((crate::gfx::canvas::WIDTH - self.tile_width() * self.rules.columns as f32) / 2.0).floor()
    }

    /// Part du temps écoulée, entre 0 et 1. Toujours 0 si le niveau n'est pas
    /// chronométré, pour que la jauge reste vide plutôt que de mentir.
    pub fn time_ratio(&self) -> f32 {
        if self.rules.is_timed() {
            (self.time_left / self.rules.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// Avance la manche d'une frame. Renvoie le bilan quand elle se termine.
    pub fn update(&mut self, dt: f32) -> Option<Outcome> {
        self.read_input();
        self.spawn(dt);
        self.advance_tiles(dt);

        if self.rules.is_timed() {
            self.time_left -= dt;
            if self.time_left <= 0.0 {
                self.time_left = 0.0;
                return Some(self.outcome(EndReason::TimeUp));
            }
        }

        if self.lives == 0 {
            return Some(self.outcome(EndReason::OutOfLives));
        }

        None
    }

    fn read_input(&mut self) {
        if is_key_pressed(KeyCode::Backspace) {
            self.input.pop();
        }

        while let Some(character) = get_char_pressed() {
            if character.is_alphanumeric() {
                self.input.push(character.to_lowercase().next().unwrap_or(character));
            }
        }

        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            self.validate();
        }
    }

    fn validate(&mut self) {
        let answer = std::mem::take(&mut self.input);
        if answer.is_empty() {
            return;
        }

        // La tuile la plus basse d'abord : c'est celle qui est sur le point
        // d'être perdue, et deux tuiles identiques ne doivent pas tomber
        // ensemble sur une seule frappe.
        let target = self
            .tiles
            .iter_mut()
            .filter(|tile| !tile.is_cleared() && tile.glyph.accepts(&answer))
            .max_by(|a, b| a.y.total_cmp(&b.y));

        match target {
            Some(tile) => {
                tile.cleared = Some(CLEAR_FLASH);
                self.hits += 1;
                self.score += POINTS_PER_HIT;
            }
            None => self.wrong += 1,
        }
    }

    fn spawn(&mut self, dt: f32) {
        self.spawn_timer += dt;
        if self.spawn_timer < self.rules.spawn_interval {
            return;
        }
        self.spawn_timer = 0.0;

        let glyph = self.pick_glyph().clone();
        let column = rand::gen_range(0, self.rules.columns);

        self.tiles.push(Tile { column, y: -TILE_HEIGHT, glyph, cleared: None });
        self.spawned += 1;
    }

    /// Tire un glyphe : soit du niveau, soit — selon `review_ratio` — de ses
    /// prérequis, pour ne pas oublier ce qui vient d'être appris.
    fn pick_glyph(&self) -> &Glyph {
        let reviewing = !self.review.is_empty()
            && self.rules.review_ratio > 0.0
            && rand::gen_range(0.0, 1.0) < self.rules.review_ratio;

        let pool = if reviewing { &self.review } else { &self.glyphs };
        &pool[rand::gen_range(0, pool.len())]
    }

    fn advance_tiles(&mut self, dt: f32) {
        let speed = self.rules.speed.at(self.spawned);
        let limit = TARGET_Y;

        let mut lost = 0;
        for tile in self.tiles.iter_mut() {
            match &mut tile.cleared {
                // Une tuile validée reste un instant en place, le temps du flash.
                Some(remaining) => *remaining -= dt,
                None => {
                    tile.y += speed * dt;
                    if tile.y + TILE_HEIGHT >= limit {
                        lost += 1;
                        self.missed += 1;
                        if !self.missed_glyphs.contains(&tile.glyph.char) {
                            self.missed_glyphs.push(tile.glyph.char.clone());
                        }
                    }
                }
            }
        }

        // Plusieurs tuiles perdues sur la même frame coûtent bien plusieurs vies.
        self.lives = self.lives.saturating_sub(lost);

        self.tiles.retain(|tile| match tile.cleared {
            Some(remaining) => remaining > 0.0,
            None => tile.y + TILE_HEIGHT < limit,
        });
    }

    fn outcome(&self, reason: EndReason) -> Outcome {
        let accuracy = self.accuracy();

        Outcome {
            language_id: self.language_id.clone(),
            level_id: self.level_id.clone(),
            level_title: self.level_title.clone(),
            score: self.score,
            hits: self.hits,
            accuracy,
            stars: self.stars.rate(accuracy),
            reason,
            is_record: false,
            missed_glyphs: self.missed_glyphs.clone(),
        }
    }

    /// Part de réussite : les glyphes reconnus rapportés à tout ce qui a été
    /// tenté, y compris les réponses fausses. Une manche sans aucune tentative
    /// vaut 0 — sinon rester immobile donnerait trois étoiles.
    fn accuracy(&self) -> f32 {
        let attempts = self.hits + self.missed + self.wrong;
        if attempts == 0 {
            return 0.0;
        }

        self.hits as f32 / attempts as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::model::{Glyph, Level, Speed, Stars};
    use crate::data::{Catalog, Language};

    fn glyph(character: &str, answer: &str) -> Glyph {
        Glyph {
            char: character.to_string(),
            answers: vec![answer.to_string()],
            hint: String::new(),
        }
    }

    fn level(id: &str, requires: &[&str], glyphs: Vec<Glyph>) -> Level {
        Level {
            id: id.to_string(),
            title: id.to_string(),
            subtitle: String::new(),
            order: 1,
            requires: requires.iter().map(|s| s.to_string()).collect(),
            mode: GameMode::TileFall,
            rules: Rules { speed: Speed { start: 100.0, ramp: 0.0, max: 100.0 }, ..Rules::default() },
            stars: Stars { one: 0.5, two: 0.75, three: 0.9 },
            glyphs,
        }
    }

    fn catalog(levels: Vec<Level>) -> Catalog {
        Catalog {
            languages: vec![Language {
                id: "ko".into(),
                name: "Coréen".into(),
                native_name: "한국어".into(),
                description: String::new(),
                font: None,
                levels,
            }],
        }
    }

    fn session(levels: Vec<Level>, level_id: &str) -> Session {
        let catalog = catalog(levels);
        Session::new(&catalog, "ko", level_id).expect("niveau présent")
    }

    fn falling_tile(session: &mut Session, character: &str, answer: &str, y: f32) {
        session.tiles.push(Tile { column: 0, y, glyph: glyph(character, answer), cleared: None });
    }

    #[test]
    fn rules_come_from_the_level_file() {
        let mut level = level("ko-01", &[], vec![glyph("ㄱ", "g")]);
        level.rules.lives = 5;
        level.rules.columns = 6;
        level.rules.duration = 42.0;

        let session = session(vec![level], "ko-01");

        assert_eq!(session.lives, 5);
        assert_eq!(session.rules.columns, 6);
        assert_eq!(session.time_left, 42.0);
    }

    #[test]
    fn a_correct_answer_clears_the_lowest_matching_tile_only() {
        // Deux tuiles identiques à l'écran : une frappe n'en valide qu'une, la
        // plus basse, celle qui est sur le point d'être perdue.
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");
        falling_tile(&mut session, "ㄱ", "g", 10.0);
        falling_tile(&mut session, "ㄱ", "g", 90.0);

        session.input = "g".into();
        session.validate();

        assert_eq!(session.hits, 1);
        let cleared: Vec<_> = session.tiles.iter().filter(|tile| tile.is_cleared()).collect();
        assert_eq!(cleared.len(), 1);
        assert_eq!(cleared[0].y, 90.0, "la tuile la plus basse doit partir");
    }

    #[test]
    fn a_wrong_answer_counts_against_accuracy() {
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");
        falling_tile(&mut session, "ㄱ", "g", 10.0);

        session.input = "zzz".into();
        session.validate();

        assert_eq!(session.wrong, 1);
        assert_eq!(session.hits, 0);
        assert_eq!(session.accuracy(), 0.0);
    }

    #[test]
    fn an_empty_validation_is_ignored() {
        // Marteler Entrée ne doit pas ruiner la precision.
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");

        session.validate();
        session.validate();

        assert_eq!(session.wrong, 0);
    }

    #[test]
    fn several_tiles_lost_on_one_frame_cost_several_lives() {
        // Le bug historique : un simple drapeau « une tuile ratée » ne retirait
        // qu'une vie quelle que soit le nombre de tuiles perdues.
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");
        session.lives = 3;
        falling_tile(&mut session, "ㄱ", "g", TARGET_Y - TILE_HEIGHT - 1.0);
        falling_tile(&mut session, "ㄴ", "n", TARGET_Y - TILE_HEIGHT - 1.0);

        session.advance_tiles(0.5);

        assert_eq!(session.lives, 1);
        assert_eq!(session.missed, 2);
        assert!(session.tiles.is_empty(), "les tuiles perdues quittent l'écran");
    }

    #[test]
    fn missed_glyphs_are_listed_once_each() {
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");
        session.lives = 9;
        for _ in 0..3 {
            falling_tile(&mut session, "ㄱ", "g", TARGET_Y - TILE_HEIGHT - 1.0);
        }

        session.advance_tiles(0.5);

        assert_eq!(session.missed_glyphs, vec!["ㄱ".to_string()]);
    }

    #[test]
    fn a_cleared_tile_stops_falling_and_leaves_after_its_flash() {
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");
        falling_tile(&mut session, "ㄱ", "g", 10.0);
        session.input = "g".into();
        session.validate();

        session.advance_tiles(CLEAR_FLASH / 2.0);
        assert_eq!(session.tiles.len(), 1);
        assert_eq!(session.tiles[0].y, 10.0, "une tuile validée ne bouge plus");

        session.advance_tiles(CLEAR_FLASH);
        assert!(session.tiles.is_empty());
    }

    #[test]
    fn review_draws_from_prerequisites() {
        let levels = vec![
            level("ko-01", &[], vec![glyph("ㄱ", "g")]),
            level("ko-02", &["ko-01"], vec![glyph("ㅏ", "a")]),
        ];
        let session = session(levels, "ko-02");

        assert_eq!(session.review.len(), 1);
        assert_eq!(session.review[0].char, "ㄱ");
    }

    #[test]
    fn an_entry_level_has_nothing_to_review() {
        let session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");

        assert!(session.review.is_empty());
        assert_eq!(session.pick_glyph().char, "ㄱ");
    }

    #[test]
    fn doing_nothing_earns_no_star() {
        let session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");

        let outcome = session.outcome(EndReason::TimeUp);

        assert_eq!(outcome.accuracy, 0.0);
        assert_eq!(outcome.stars, 0);
    }

    #[test]
    fn a_flawless_run_earns_three_stars() {
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");
        for _ in 0..10 {
            falling_tile(&mut session, "ㄱ", "g", 10.0);
            session.input = "g".into();
            session.validate();
        }

        let outcome = session.outcome(EndReason::TimeUp);

        assert_eq!(outcome.accuracy, 1.0);
        assert_eq!(outcome.stars, 3);
        assert_eq!(outcome.score, 10 * POINTS_PER_HIT);
    }

    #[test]
    fn columns_divide_the_playfield_into_whole_pixels() {
        let mut level = level("ko-01", &[], vec![glyph("ㄱ", "g")]);
        level.rules.columns = 5;

        let session = session(vec![level], "ko-01");

        assert_eq!(session.tile_width(), 38.0);
        assert_eq!(session.tile_width().fract(), 0.0);
    }
}
