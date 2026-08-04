//! Le catalogue : toutes les langues chargées, et les règles de cohérence
//! qu'elles doivent respecter.
//!
//! La validation est volontairement stricte et se fait au démarrage. Un fichier
//! TOML incohérent doit produire un message clair tout de suite, pas un chemin
//! d'apprentissage silencieusement cassé trois écrans plus loin.

use std::collections::HashSet;
use std::fmt;

use super::model::{Language, Level};

#[derive(Debug, Default)]
pub struct Catalog {
    pub languages: Vec<Language>,
}

impl Catalog {
    pub fn language(&self, id: &str) -> Option<&Language> {
        self.languages.iter().find(|language| language.id == id)
    }

    /// Vérifie toutes les invariants du catalogue.
    pub fn validate(&self) -> Result<(), DataError> {
        let mut seen_languages = HashSet::new();
        let mut seen_levels: HashSet<&str> = HashSet::new();

        for language in &self.languages {
            if !seen_languages.insert(language.id.as_str()) {
                return Err(DataError::DuplicateLanguage(language.id.clone()));
            }
            if language.levels.is_empty() {
                return Err(DataError::EmptyLanguage(language.id.clone()));
            }

            for level in &language.levels {
                // Unicité globale et non par langue : la sauvegarde de
                // progression indexe les niveaux par leur seul identifiant.
                if !seen_levels.insert(level.id.as_str()) {
                    return Err(DataError::DuplicateLevel(level.id.clone()));
                }
                validate_level(level)?;

                for required in &level.requires {
                    if language.level(required).is_none() {
                        return Err(DataError::UnknownRequirement {
                            level: level.id.clone(),
                            requires: required.clone(),
                        });
                    }
                }
            }

            detect_cycle(language)?;
        }

        Ok(())
    }
}

fn validate_level(level: &Level) -> Result<(), DataError> {
    let invalid = |reason: &str| {
        Err(DataError::InvalidLevel { level: level.id.clone(), reason: reason.to_string() })
    };

    if level.glyphs.is_empty() {
        return invalid("aucun glyphe : il n'y aurait rien à apprendre");
    }
    if let Some(glyph) = level.glyphs.iter().find(|glyph| glyph.answers.is_empty()) {
        return invalid(&format!("le glyphe « {} » n'a aucune réponse acceptée", glyph.char));
    }
    if level.requires.iter().any(|required| *required == level.id) {
        return invalid("le niveau se référence lui-même dans `requires`");
    }

    let stars = &level.stars;
    if !(0.0..=1.0).contains(&stars.one)
        || !(0.0..=1.0).contains(&stars.two)
        || !(0.0..=1.0).contains(&stars.three)
    {
        return invalid("les seuils d'étoiles sont des précisions, entre 0.0 et 1.0");
    }
    if !(stars.one <= stars.two && stars.two <= stars.three) {
        return invalid("les seuils d'étoiles doivent être croissants (one <= two <= three)");
    }

    let rules = &level.rules;
    if rules.lives == 0 {
        return invalid("`lives` doit valoir au moins 1");
    }
    if rules.columns < 1 {
        return invalid("`columns` doit valoir au moins 1");
    }
    if rules.spawn_interval <= 0.0 {
        return invalid("`spawn_interval` doit être strictement positif");
    }
    if rules.duration < 0.0 {
        return invalid("`duration` ne peut pas être négatif (0 = sans limite)");
    }
    if !(0.0..=1.0).contains(&rules.review_ratio) {
        return invalid("`review_ratio` est une proportion, entre 0.0 et 1.0");
    }
    if rules.speed.start <= 0.0 || rules.speed.max < rules.speed.start || rules.speed.ramp < 0.0 {
        return invalid("`speed` doit vérifier 0 < start <= max et ramp >= 0");
    }
    // Un niveau qui ne révise que d'anciens glyphes sans prérequis n'aurait
    // rien à réviser : la tuile ne pourrait pas être tirée.
    if rules.review_ratio > 0.0 && level.requires.is_empty() {
        return invalid("`review_ratio` > 0 sans aucun prérequis à réviser");
    }

    Ok(())
}

/// Détecte un cycle dans le graphe de prérequis, qui rendrait des niveaux
/// définitivement inaccessibles.
fn detect_cycle(language: &Language) -> Result<(), DataError> {
    // Parcours en profondeur classique : `visiting` = sur la pile courante,
    // `visited` = sous-arbre déjà prouvé sain.
    let mut visited: HashSet<&str> = HashSet::new();
    let mut visiting: HashSet<&str> = HashSet::new();

    fn visit<'a>(
        language: &'a Language,
        level: &'a Level,
        visited: &mut HashSet<&'a str>,
        visiting: &mut HashSet<&'a str>,
    ) -> Result<(), DataError> {
        if visited.contains(level.id.as_str()) {
            return Ok(());
        }
        if !visiting.insert(level.id.as_str()) {
            return Err(DataError::CyclicRequirements {
                language: language.id.clone(),
                level: level.id.clone(),
            });
        }

        for required in &level.requires {
            if let Some(parent) = language.level(required) {
                visit(language, parent, visited, visiting)?;
            }
        }

        visiting.remove(level.id.as_str());
        visited.insert(level.id.as_str());
        Ok(())
    }

    for level in &language.levels {
        visit(language, level, &mut visited, &mut visiting)?;
    }

    Ok(())
}

#[derive(Debug)]
pub enum DataError {
    /// Le manifeste `language.toml` d'un dossier de langue est absent.
    MissingManifest(String),
    /// Un fichier embarqué n'est pas de l'UTF-8 valide.
    NotUtf8(String),
    /// Erreur de syntaxe ou de schéma dans un TOML.
    Parse { file: String, source: toml::de::Error },
    DuplicateLanguage(String),
    DuplicateLevel(String),
    EmptyLanguage(String),
    UnknownRequirement { level: String, requires: String },
    CyclicRequirements { language: String, level: String },
    InvalidLevel { level: String, reason: String },
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingManifest(dir) => {
                write!(f, "le dossier de langue « {dir} » n'a pas de fichier language.toml")
            }
            Self::NotUtf8(file) => write!(f, "« {file} » n'est pas encodé en UTF-8"),
            Self::Parse { file, source } => write!(f, "« {file} » est invalide : {source}"),
            Self::DuplicateLanguage(id) => write!(f, "deux langues portent l'identifiant « {id} »"),
            Self::DuplicateLevel(id) => write!(f, "deux niveaux portent l'identifiant « {id} »"),
            Self::EmptyLanguage(id) => write!(f, "la langue « {id} » n'a aucun niveau"),
            Self::UnknownRequirement { level, requires } => write!(
                f,
                "le niveau « {level} » exige « {requires} », qui n'existe pas dans cette langue"
            ),
            Self::CyclicRequirements { language, level } => write!(
                f,
                "les prérequis de « {language} » forment un cycle passant par « {level} »"
            ),
            Self::InvalidLevel { level, reason } => write!(f, "niveau « {level} » : {reason}"),
        }
    }
}

impl std::error::Error for DataError {}

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
                hint: String::new(),
            }],
        }
    }

    fn language(id: &str, levels: Vec<Level>) -> Language {
        Language {
            id: id.to_string(),
            name: id.to_string(),
            native_name: id.to_string(),
            description: String::new(),
            font: None,
            levels,
        }
    }

    fn catalog(languages: Vec<Language>) -> Catalog {
        Catalog { languages }
    }

    #[test]
    fn a_coherent_catalog_validates() {
        let catalog = catalog(vec![language(
            "ko",
            vec![level("ko-01", &[]), level("ko-02", &["ko-01"]), level("ko-03", &["ko-02"])],
        )]);

        assert!(catalog.validate().is_ok());
    }

    #[test]
    fn level_ids_are_unique_across_languages() {
        let catalog = catalog(vec![
            language("ko", vec![level("shared", &[])]),
            language("ja", vec![level("shared", &[])]),
        ]);

        assert!(matches!(catalog.validate(), Err(DataError::DuplicateLevel(_))));
    }

    #[test]
    fn a_requirement_must_exist_in_the_same_language() {
        let catalog = catalog(vec![language("ko", vec![level("ko-01", &["ja-01"])])]);

        assert!(matches!(catalog.validate(), Err(DataError::UnknownRequirement { .. })));
    }

    #[test]
    fn cyclic_requirements_are_rejected() {
        let catalog = catalog(vec![language(
            "ko",
            vec![level("a", &["c"]), level("b", &["a"]), level("c", &["b"])],
        )]);

        assert!(matches!(catalog.validate(), Err(DataError::CyclicRequirements { .. })));
    }

    #[test]
    fn a_diamond_shaped_path_is_not_a_cycle() {
        // a → b, a → c, puis b et c → d : le noeud `a` est visité deux fois
        // sans qu'il y ait de cycle. Un simple « déjà vu » naïf se tromperait.
        let catalog = catalog(vec![language(
            "ko",
            vec![
                level("a", &[]),
                level("b", &["a"]),
                level("c", &["a"]),
                level("d", &["b", "c"]),
            ],
        )]);

        assert!(catalog.validate().is_ok());
    }

    #[test]
    fn star_thresholds_must_increase() {
        let mut broken = level("ko-01", &[]);
        broken.stars = Stars { one: 0.9, two: 0.5, three: 0.95 };

        let catalog = catalog(vec![language("ko", vec![broken])]);

        assert!(matches!(catalog.validate(), Err(DataError::InvalidLevel { .. })));
    }

    #[test]
    fn reviewing_without_prerequisites_is_rejected() {
        let mut broken = level("ko-01", &[]);
        broken.rules.review_ratio = 0.3;

        let catalog = catalog(vec![language("ko", vec![broken])]);

        assert!(matches!(catalog.validate(), Err(DataError::InvalidLevel { .. })));
    }

}
