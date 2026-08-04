//! Une manche en cours.
//!
//! Tout ce qui décide du déroulement — vies, durée, colonnes, vitesse, tirage
//! des glyphes, seuils d'étoiles — vient du fichier TOML du niveau. Rien n'est
//! codé en dur ici : ajouter une langue ou régler une difficulté ne doit jamais
//! demander de toucher au code.

use macroquad::prelude::*;

use std::collections::{BTreeMap, HashSet, VecDeque};

use crate::data::{Catalog, GameMode, Glyph, Language, Level, Rules, Stars};
use crate::progress::Progress;

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
/// Duree du tremblement quand une vie est perdue.
const SHAKE_DURATION: f32 = 0.2;
/// Amplitude maximale du tremblement, en pixels virtuels.
const SHAKE_PIXELS: f32 = 3.0;

/// Une tuile ratée revient au bout de ce nombre d'apparitions.
///
/// Assez tôt pour que la correction serve encore, assez tard pour laisser le
/// temps de la lire sur l'écran. La renvoyer aussitôt ne laisserait pas
/// réfléchir ; ne jamais la renvoyer laisse partir avec l'erreur.
const RETRY_AFTER: u32 = 3;

/// Au-delà, la file d'attente des ratés est purgée par la tête.
///
/// Une manche catastrophique accumulerait sinon une file plus longue que la
/// manche elle-même, et ne présenterait plus jamais de signe neuf.
const MAX_RETRIES: usize = 6;

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
    /// Les signes neufs, présentés un par un au démarrage.
    warmup: VecDeque<Glyph>,
    /// Les signes ratés, à représenter rapidement.
    retry: VecDeque<Glyph>,
    /// Poids de tirage venus de la progression, par signe.
    weights: BTreeMap<String, u32>,
    /// Bilan par signe : réussites et ratés, pour nourrir la maîtrise.
    tally: BTreeMap<String, (u32, u32)>,

    pub tiles: Vec<Tile>,
    pub score: u32,
    pub lives: u32,
    pub input: String,
    /// Temps restant ; ignoré si le niveau n'est pas chronométré.
    pub time_left: f32,

    spawn_timer: f32,
    spawned: u32,
    /// Apparitions depuis le dernier rappel d'un signe raté.
    since_retry: u32,

    hits: u32,
    /// Tuiles tombées sans avoir été reconnues.
    missed: u32,
    /// Validations qui ne correspondaient à aucune tuile.
    wrong: u32,
    /// Les glyphes ratés, pour les rappeler à la fin.
    missed_glyphs: Vec<String>,

    /// Ce qui vient de se produire, a lire une fois par frame par l'ecran.
    /// La manche ne joue pas de son elle-meme : elle ne connait que ses regles.
    events: Vec<Event>,
    /// Secondes de tremblement restantes.
    shake: f32,
}

/// Un fait de jeu, remonte a l'ecran le temps d'une frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Hit,
    Wrong,
    Missed,
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
    /// Ce que chaque signe a donné, pour mettre la maîtrise à jour.
    pub signs: Vec<SignResult>,
}

/// Le bilan d'un signe sur une manche.
#[derive(Debug, Clone)]
pub struct SignResult {
    pub character: String,
    pub hits: u32,
    pub misses: u32,
}

impl Session {
    /// Prépare une manche. `None` si le niveau n'existe pas dans le catalogue.
    pub fn new(
        catalog: &Catalog,
        progress: &Progress,
        language_id: &str,
        level_id: &str,
    ) -> Option<Self> {
        let language = catalog.language(language_id)?;
        let level = language.level(level_id)?;

        // Un seul mode pour l'instant : ce `match` obligera à traiter les
        // suivants le jour où ils apparaîtront.
        match level.mode {
            GameMode::TileFall => {}
        }

        let review = review_pool(language, level);

        // Les poids sont copiés une fois pour toutes : la manche ne doit pas
        // garder d'emprunt sur la progression, qui est modifiée à la fin.
        let weights = level
            .glyphs
            .iter()
            .chain(review.iter())
            .map(|glyph| (glyph.char.clone(), progress.draw_weight(&glyph.char)))
            .collect();

        // Chaque signe neuf passe une première fois, dans l'ordre du fichier.
        // Un tirage purement aléatoire pourrait montrer le même trois fois de
        // suite et en oublier un autre jusqu'à la fin de la manche.
        let warmup = level.glyphs.iter().cloned().collect();

        Some(Self {
            language_id: language_id.to_string(),
            level_id: level_id.to_string(),
            level_title: level.title.clone(),
            rules: level.rules.clone(),
            stars: level.stars,
            glyphs: level.glyphs.clone(),
            review,
            warmup,
            retry: VecDeque::new(),
            weights,
            tally: BTreeMap::new(),
            tiles: Vec::new(),
            score: 0,
            lives: level.rules.lives,
            input: String::new(),
            time_left: level.rules.duration,
            spawn_timer: 0.0,
            spawned: 0,
            since_retry: 0,
            hits: 0,
            missed: 0,
            wrong: 0,
            missed_glyphs: Vec::new(),
            events: Vec::new(),
            shake: 0.0,
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

    /// Ce qui s'est produit depuis la derniere lecture, puis remet a zero.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// De combien decaler le rendu de la zone de jeu.
    ///
    /// Perdre une vie secoue l'ecran : le coeur qui s'eteint est trop discret
    /// quand on a les yeux sur les tuiles.
    pub fn shake_offset(&self) -> Vec2 {
        if self.shake <= 0.0 {
            return Vec2::ZERO;
        }

        let strength = (self.shake / SHAKE_DURATION * SHAKE_PIXELS).ceil();
        vec2(rand::gen_range(-strength, strength).round(), 0.0)
    }

    /// Avance la manche d'une frame. Renvoie le bilan quand elle se termine.
    pub fn update(&mut self, dt: f32) -> Option<Outcome> {
        self.shake = (self.shake - dt).max(0.0);
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
                let character = tile.glyph.char.clone();
                self.hits += 1;
                self.score += POINTS_PER_HIT;
                self.events.push(Event::Hit);
                self.tally.entry(character).or_default().0 += 1;
            }
            None => {
                self.wrong += 1;
                self.events.push(Event::Wrong);
            }
        }
    }

    fn spawn(&mut self, dt: f32) {
        self.spawn_timer += dt;
        if self.spawn_timer < self.rules.spawn_interval {
            return;
        }
        self.spawn_timer = 0.0;

        let glyph = self.pick_glyph();
        let column = rand::gen_range(0, self.rules.columns);

        self.tiles.push(Tile { column, y: -TILE_HEIGHT, glyph, cleared: None });
        self.spawned += 1;
        self.since_retry += 1;
    }

    /// Choisit le prochain signe à faire tomber.
    ///
    /// Trois priorités, dans cet ordre : présenter les signes neufs, revenir sur
    /// ce qui vient d'être raté, puis tirer au sort en favorisant le mal su.
    fn pick_glyph(&mut self) -> Glyph {
        if let Some(glyph) = self.warmup.pop_front() {
            return glyph;
        }

        if self.since_retry >= RETRY_AFTER {
            if let Some(glyph) = self.retry.pop_front() {
                self.since_retry = 0;
                return glyph;
            }
        }

        let reviewing = !self.review.is_empty()
            && self.rules.review_ratio > 0.0
            && rand::gen_range(0.0, 1.0) < self.rules.review_ratio;

        let pool = if reviewing { &self.review } else { &self.glyphs };
        weighted_pick(pool, &self.weights).clone()
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
                        self.tally.entry(tile.glyph.char.clone()).or_default().1 += 1;

                        if !self.missed_glyphs.contains(&tile.glyph.char) {
                            self.missed_glyphs.push(tile.glyph.char.clone());
                        }

                        // Un signe raté doit revenir : c'est le seul moment où
                        // la correction porte encore.
                        self.retry.push_back(tile.glyph.clone());
                        if self.retry.len() > MAX_RETRIES {
                            self.retry.pop_front();
                        }
                    }
                }
            }
        }

        // Plusieurs tuiles perdues sur la même frame coûtent bien plusieurs vies.
        self.lives = self.lives.saturating_sub(lost);
        if lost > 0 {
            self.events.push(Event::Missed);
            self.shake = SHAKE_DURATION;
        }

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
            signs: self
                .tally
                .iter()
                .map(|(character, (hits, misses))| SignResult {
                    character: character.clone(),
                    hits: *hits,
                    misses: *misses,
                })
                .collect(),
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

/// Tire un signe au hasard, en favorisant ceux que le joueur maîtrise mal.
///
/// Un tirage uniforme ferait revenir aussi souvent un signe acquis depuis dix
/// étapes qu'un signe raté la veille : la révision passerait le plus clair de
/// son temps sur ce qui est déjà su.
fn weighted_pick<'a>(pool: &'a [Glyph], weights: &BTreeMap<String, u32>) -> &'a Glyph {
    let weight = |glyph: &Glyph| weights.get(&glyph.char).copied().unwrap_or(1).max(1);
    let total: u32 = pool.iter().map(weight).sum();

    let mut ticket = rand::gen_range(0, total);
    for glyph in pool {
        let share = weight(glyph);
        if ticket < share {
            return glyph;
        }
        ticket -= share;
    }

    // Inatteignable : les parts couvrent tout le total.
    &pool[0]
}

/// Tous les signes déjà rencontrés avant ce niveau.
///
/// La collecte remonte **toute** la chaîne de prérequis, pas seulement le
/// niveau juste avant. Sur un chemin d'une quinzaine d'étapes, ne réviser que
/// l'étape précédente laisserait le début s'effacer : c'est précisément ce
/// qu'un apprentissage progressif doit empêcher.
fn review_pool(language: &Language, level: &Level) -> Vec<Glyph> {
    let mut pending: Vec<&str> = level.requires.iter().map(String::as_str).collect();
    let mut visited: HashSet<&str> = HashSet::new();
    let mut seen_chars: HashSet<&str> = HashSet::new();
    let mut pool = Vec::new();

    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        let Some(previous) = language.level(id) else { continue };

        for glyph in &previous.glyphs {
            // Un signe réintroduit par un niveau de révision ne doit pas peser
            // double dans le tirage.
            if seen_chars.insert(glyph.char.as_str()) {
                pool.push(glyph.clone());
            }
        }
        pending.extend(previous.requires.iter().map(String::as_str));
    }

    pool
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::model::{Glyph, Level, Speed, Stars};
    use crate::data::{Catalog, Language};
    use crate::progress::Progress;

    fn glyph(character: &str, answer: &str) -> Glyph {
        Glyph {
            char: character.to_string(),
            answers: vec![answer.to_string()],
            mnemonics: vec!["un moyen".to_string()],
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
        Session::new(&catalog, &Progress::new(), "ko", level_id).expect("niveau présent")
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
    fn review_reaches_back_through_the_whole_chain() {
        // Sur un chemin d'une quinzaine d'etapes, ne reviser que l'etape
        // precedente laisserait le debut s'effacer.
        let levels = vec![
            level("ko-01", &[], vec![glyph("ㄱ", "g")]),
            level("ko-02", &["ko-01"], vec![glyph("ㄴ", "n")]),
            level("ko-03", &["ko-02"], vec![glyph("ㄷ", "d")]),
            level("ko-04", &["ko-03"], vec![glyph("ㄹ", "r")]),
        ];
        let session = session(levels, "ko-04");

        let mut revised: Vec<&str> = session.review.iter().map(|g| g.char.as_str()).collect();
        revised.sort_unstable();
        assert_eq!(revised, vec!["ㄱ", "ㄴ", "ㄷ"]);
    }

    #[test]
    fn a_sign_repeated_by_a_revision_level_is_not_drawn_twice_as_often() {
        // Les niveaux de revision reprennent des signes deja enseignes ; sans
        // dedoublonnage, ceux-la sortiraient deux fois plus souvent que les
        // autres dans le tirage.
        let levels = vec![
            level("ko-01", &[], vec![glyph("ㄱ", "g")]),
            level("ko-02", &["ko-01"], vec![glyph("ㄴ", "n")]),
            // Une revision qui reprend les deux precedents.
            level("ko-03", &["ko-02"], vec![glyph("ㄱ", "g"), glyph("ㄴ", "n")]),
            level("ko-04", &["ko-03"], vec![glyph("ㄷ", "d")]),
        ];
        let session = session(levels, "ko-04");

        assert_eq!(session.review.len(), 2, "chaque signe ne doit compter qu'une fois");
    }

    #[test]
    fn an_entry_level_has_nothing_to_review() {
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");

        assert!(session.review.is_empty());
        assert_eq!(session.pick_glyph().char, "ㄱ");
    }

    #[test]
    fn every_new_sign_is_shown_once_before_any_repeat() {
        // Un tirage purement aleatoire peut montrer trois fois le meme signe et
        // en oublier un autre jusqu'a la fin de la manche. Les signes neufs
        // passent donc d'abord, chacun son tour.
        let glyphs = vec![glyph("ㄱ", "g"), glyph("ㄴ", "n"), glyph("ㅁ", "m")];
        let mut session = session(vec![level("ko-01", &[], glyphs)], "ko-01");

        let opening: Vec<String> =
            (0..3).map(|_| session.pick_glyph().char.clone()).collect();

        assert_eq!(opening, vec!["ㄱ", "ㄴ", "ㅁ"]);
    }

    #[test]
    fn a_missed_sign_comes_back_within_the_round() {
        // C'est le seul moment ou la correction porte encore : laisser filer un
        // signe rate sans jamais le representer, c'est le laisser mal appris.
        let glyphs = vec![glyph("ㄱ", "g"), glyph("ㄴ", "n")];
        let mut session = session(vec![level("ko-01", &[], glyphs)], "ko-01");
        session.lives = 9;
        session.warmup.clear();

        falling_tile(&mut session, "ㅁ", "m", TARGET_Y - TILE_HEIGHT - 1.0);
        session.advance_tiles(0.5);
        assert_eq!(session.retry.len(), 1, "le signe rate entre en file");

        // Les premieres apparitions laissent le temps de lire la correction,
        // puis le signe revient.
        let drawn: Vec<String> = (0..RETRY_AFTER + 1)
            .map(|_| {
                session.since_retry += 1;
                session.pick_glyph().char.clone()
            })
            .collect();

        assert!(drawn.contains(&"ㅁ".to_string()), "le signe rate doit revenir : {drawn:?}");
    }

    #[test]
    fn the_retry_queue_cannot_swallow_the_round() {
        // Une manche catastrophique accumulerait une file plus longue que la
        // manche, et ne presenterait plus jamais de signe neuf.
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");
        session.lives = 99;

        for _ in 0..MAX_RETRIES * 3 {
            falling_tile(&mut session, "ㄱ", "g", TARGET_Y - TILE_HEIGHT - 1.0);
            session.advance_tiles(0.5);
        }

        assert!(session.retry.len() <= MAX_RETRIES);
    }

    #[test]
    fn the_round_reports_what_each_sign_gave() {
        // C'est ce bilan qui nourrit la maitrise d'une manche a l'autre.
        let mut session = session(vec![level("ko-01", &[], vec![glyph("ㄱ", "g")])], "ko-01");
        session.lives = 9;

        falling_tile(&mut session, "ㄱ", "g", 10.0);
        session.input = "g".into();
        session.validate();

        falling_tile(&mut session, "ㄴ", "n", TARGET_Y - TILE_HEIGHT - 1.0);
        session.advance_tiles(0.5);

        let outcome = session.outcome(EndReason::TimeUp);
        let find = |c: &str| outcome.signs.iter().find(|s| s.character == c).cloned();

        assert_eq!(find("ㄱ").map(|s| (s.hits, s.misses)), Some((1, 0)));
        assert_eq!(find("ㄴ").map(|s| (s.hits, s.misses)), Some((0, 1)));
    }

    #[test]
    fn a_shaky_sign_is_drawn_more_often_than_a_solid_one() {
        // La ponderation vient de la progression : sans elle, une revision
        // passerait le plus clair de son temps sur ce qui est deja su.
        let mut progress = Progress::new();
        progress.note("ㄱ", 0, 4); // fragile
        progress.note("ㄴ", 4, 0); // solide

        let catalog = catalog(vec![level(
            "ko-01",
            &[],
            vec![glyph("ㄱ", "g"), glyph("ㄴ", "n")],
        )]);
        let mut session = Session::new(&catalog, &progress, "ko", "ko-01").expect("niveau");
        session.warmup.clear();

        let mut shaky = 0;
        for _ in 0..400 {
            if session.pick_glyph().char == "ㄱ" {
                shaky += 1;
            }
        }

        assert!(shaky > 240, "le signe fragile devrait dominer, obtenu {shaky}/400");
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
