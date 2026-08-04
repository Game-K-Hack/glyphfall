//! Chargement du contenu depuis `assets/`.
//!
//! Les fichiers sont embarqués dans le binaire à la compilation via
//! `include_dir!`. C'est ce qui permet de garder une arborescence de fichiers
//! éditable à la main tout en fonctionnant partout, y compris en WebAssembly
//! où il n'y a pas de système de fichiers, et sans dépendre du répertoire
//! courant au lancement.
//!
//! Ajouter une langue = créer un dossier ici puis recompiler. Rien à déclarer
//! dans le code.

use include_dir::{Dir, include_dir};

use super::catalog::{Catalog, DataError};
use super::model::{Language, Level};

static LANGUAGES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/languages");
static FONTS: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/fonts");

/// Charge et valide toutes les langues disponibles.
pub fn load_catalog() -> Result<Catalog, DataError> {
    let mut languages = Vec::new();

    for language_dir in LANGUAGES.dirs() {
        languages.push(load_language(language_dir)?);
    }

    // Ordre d'affichage stable : `dirs()` ne garantit rien d'utile pour l'oeil.
    languages.sort_by(|a, b| a.name.cmp(&b.name));

    let catalog = Catalog { languages };
    catalog.validate()?;
    Ok(catalog)
}

fn load_language(dir: &Dir<'static>) -> Result<Language, DataError> {
    let dir_name = dir.path().display().to_string();

    let manifest_path = dir.path().join("language.toml");
    let manifest = LANGUAGES
        .get_file(&manifest_path)
        .ok_or_else(|| DataError::MissingManifest(dir_name.clone()))?;

    let mut language: Language = parse(manifest.path().display().to_string(), manifest.contents())?;
    language.levels = load_levels(dir)?;

    // Tri par `order`, puis par identifiant pour rester déterministe si deux
    // niveaux partagent le même rang.
    language.levels.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));

    Ok(language)
}

fn load_levels(dir: &Dir<'static>) -> Result<Vec<Level>, DataError> {
    let Some(levels_dir) = LANGUAGES.get_dir(dir.path().join("levels")) else {
        // Pas de dossier `levels/` : `Catalog::validate` signalera la langue
        // vide avec un message plus parlant que « dossier absent ».
        return Ok(Vec::new());
    };

    let mut levels = Vec::new();
    for file in levels_dir.files() {
        if file.path().extension().is_none_or(|ext| ext != "toml") {
            continue;
        }
        levels.push(parse(file.path().display().to_string(), file.contents())?);
    }

    Ok(levels)
}

fn parse<T: serde::de::DeserializeOwned>(file: String, contents: &[u8]) -> Result<T, DataError> {
    let text = std::str::from_utf8(contents).map_err(|_| DataError::NotUtf8(file.clone()))?;
    toml::from_str(text).map_err(|source| DataError::Parse { file, source })
}

/// Les octets d'une police de `assets/fonts/`, par nom de fichier.
///
/// Renvoie `None` si le manifeste d'une langue référence une police absente ;
/// l'appelant retombe alors sur la police par défaut plutôt que de planter.
pub fn font_bytes(file_name: &str) -> Option<&'static [u8]> {
    FONTS.get_file(file_name).map(|file| file.contents())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_catalog_loads_and_validates() {
        // Ce test est le garde-fou du contenu : toute faute de frappe dans un
        // TOML de `assets/languages/` fait échouer `cargo test`.
        let catalog = match load_catalog() {
            Ok(catalog) => catalog,
            Err(error) => panic!("le catalogue embarqué est invalide : {error}"),
        };

        assert!(!catalog.languages.is_empty(), "aucune langue embarquée");
    }

    #[test]
    fn every_glyph_is_drawable_by_its_language_font() {
        // Un caractère absent de la police s'afficherait en tofu (□) sans que
        // rien ne plante : ce test attrape le problème au moment du contenu.
        let catalog = load_catalog().expect("catalogue valide");

        for language in &catalog.languages {
            let Some(font_name) = &language.font else { continue };
            let bytes = font_bytes(font_name).expect("police déclarée présente");
            let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
                .unwrap_or_else(|error| panic!("« {font_name} » illisible : {error}"));

            for level in &language.levels {
                for glyph in &level.glyphs {
                    for character in glyph.char.chars() {
                        assert_ne!(
                            font.lookup_glyph_index(character),
                            0,
                            "« {character} » (niveau {}) est absent de {font_name}",
                            level.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_ui_font_covers_every_text_it_will_display() {
        // Les noms et descriptions de langues sont écrits en français avec la
        // police pixel : un accent manquant passerait inaperçu jusqu'à ce que
        // « Coréen » s'affiche « Cor en ».
        let catalog = load_catalog().expect("catalogue valide");
        let bytes = font_bytes(crate::gfx::fonts::UI_FONT_FILE).expect("police d'interface");
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .expect("police d'interface lisible");

        let mut texts = Vec::new();
        for language in &catalog.languages {
            texts.push(language.name.clone());
            texts.push(language.description.clone());
            for level in &language.levels {
                texts.push(level.title.clone());
                texts.push(level.subtitle.clone());
                for glyph in &level.glyphs {
                    texts.push(glyph.hint.clone());
                    texts.extend(glyph.answers.iter().cloned());
                }
            }
        }

        for text in texts {
            for character in text.chars() {
                // Les glyphes cités dans les aides sont dessinés avec la police
                // de la langue, pas celle de l'interface.
                if (character as u32) > 0x2000 {
                    continue;
                }
                assert_ne!(
                    font.lookup_glyph_index(character),
                    0,
                    "« {character} » (dans « {text} ») est absent de la police d'interface"
                );
            }
        }
    }

    #[test]
    fn every_language_declares_a_font_that_exists() {
        let catalog = load_catalog().expect("catalogue valide");

        for language in &catalog.languages {
            if let Some(font) = &language.font {
                assert!(
                    font_bytes(font).is_some(),
                    "la langue « {} » référence la police « {font} », absente de assets/fonts/",
                    language.id
                );
            }
        }
    }
}
