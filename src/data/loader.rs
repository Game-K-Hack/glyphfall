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
    fn every_glyph_is_drawable_by_every_font_of_its_language() {
        // Toutes les polices, pas seulement la première : une manche en tire une
        // au hasard, et un signe manquant dans la deuxième s'afficherait en tofu
        // au beau milieu d'une partie. C'est aussi ce qui protège du découpage
        // des polices, qui pourrait retirer un signe sans prévenir.
        let catalog = load_catalog().expect("catalogue valide");

        for language in &catalog.languages {
            for font_name in &language.fonts {
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

        // Les moyens mnémotechniques en font partie : ils sont écrits sans
        // caractère de l'écriture enseignée, précisément pour rester nets sur
        // la fiche d'un signe.
        let mut texts = Vec::new();
        for language in &catalog.languages {
            texts.push(language.name.clone());
            texts.push(language.description.clone());
            for level in &language.levels {
                texts.push(level.title.clone());
                texts.push(level.subtitle.clone());
                texts.extend(level.glyphs.iter().flat_map(|glyph| glyph.answers.iter().cloned()));
                texts.extend(
                    level.glyphs.iter().flat_map(|glyph| glyph.mnemonics.iter().cloned()),
                );
                // Le nom et la prononciation sont écrits avec la même police,
                // et la prononciation cite volontiers des mots français : elle
                // est le texte le plus exposé aux accents de tout le jeu.
                texts.extend(level.glyphs.iter().map(|glyph| glyph.name.clone()));
                texts.extend(level.glyphs.iter().map(|glyph| glyph.pronunciation.clone()));
            }
        }

        for text in texts {
            for character in text.chars() {
                // La prononciation est écrite en blocs : ses retours à la ligne
                // et ses retraits mettent le texte en page, ils ne sont jamais
                // dessinés.
                if character.is_whitespace() {
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
    fn every_pronunciation_fits_on_its_screen() {
        // La prononciation est le seul texte du jeu qui garde ses retours à la
        // ligne et ses retraits : c'est son auteur qui la met en page, et rien
        // à l'exécution ne l'avertirait qu'elle est coupée — `ui::block`
        // s'arrête sans bruit à la dernière ligne visible.
        //
        // Les deux budgets reprennent ceux de `screens::pronunciation`, qui
        // sont figés à la compilation : les vérifier tous les deux ici est le
        // seul moyen de couvrir l'orientation que ce binaire n'a pas.
        // Largeur et hauteur de la zone de texte, dans les deux orientations,
        // telles que `screens::pronunciation` les calcule.
        const PORTRAIT: (f32, f32) = (196.0, 254.0);
        const LANDSCAPE: (f32, f32) = (298.0, 160.0);

        let catalog = load_catalog().expect("catalogue valide");
        let bytes = font_bytes(crate::gfx::fonts::UI_FONT_FILE).expect("police d'interface");
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .expect("police d'interface lisible");

        // La police d'interface est à chasse fixe : une seule largeur suffit
        // donc à rejouer le découpage de `ui::wrap` sans contexte graphique.
        let advance = font.metrics('M', 8.0).advance_width;

        for language in &catalog.languages {
            for level in &language.levels {
                for glyph in &level.glyphs {
                    if glyph.pronunciation.is_empty() {
                        continue;
                    }
                    for (width, height) in [PORTRAIT, LANDSCAPE] {
                        let used = height_used(&glyph.pronunciation, advance, width);
                        assert!(
                            used <= height,
                            "la prononciation de « {} » prend {used} pixels de haut sur les                              {height} disponibles en {width} de large",
                            glyph.char
                        );
                    }
                }
            }
        }
    }

    /// Rejoue la mise en page de `ui::block` : une ligne source vide vaut un
    /// demi-interligne, et les lignes de continuation gardent le retrait de
    /// leur source.
    fn height_used(content: &str, advance: f32, max_width: f32) -> f32 {
        const STEP: f32 = 8.0 + 2.0;
        let mut height = 0.0;

        for source in content.lines() {
            if source.trim().is_empty() {
                height += STEP / 2.0;
                continue;
            }
            let retrait = source.len() - source.trim_start().len();
            let available = max_width - retrait as f32 * advance;
            let per_line = (available / advance).floor().max(1.0) as usize;

            let mut current = 0;
            for word in source.split_whitespace() {
                let candidate = if current == 0 {
                    word.chars().count()
                } else {
                    current + 1 + word.chars().count()
                };
                if candidate <= per_line || current == 0 {
                    current = candidate;
                } else {
                    height += STEP;
                    current = word.chars().count();
                }
            }
            if current > 0 {
                height += STEP;
            }
        }

        height
    }

    #[test]
    fn every_sign_says_how_to_remember_it() {
        // Sans moyen mnémotechnique, un signe n'est qu'une forme arbitraire à
        // recopier : c'est la seule aide dont dispose le joueur devant une
        // écriture qu'il ne connaît pas.
        let catalog = load_catalog().expect("catalogue valide");

        for language in &catalog.languages {
            for level in &language.levels {
                for glyph in &level.glyphs {
                    assert!(
                        !glyph.mnemonics.is_empty(),
                        "« {} » (niveau {}) n'a aucun moyen de le retenir",
                        glyph.char,
                        level.id
                    );
                    assert!(
                        glyph.mnemonics.iter().all(|mnemonic| mnemonic.chars().count() > 20),
                        "« {} » a un moyen trop court pour aider vraiment",
                        glyph.char
                    );
                }
            }
        }
    }

    #[test]
    fn every_language_declares_fonts_that_exist() {
        let catalog = load_catalog().expect("catalogue valide");

        for language in &catalog.languages {
            for font in &language.fonts {
                assert!(
                    font_bytes(font).is_some(),
                    "la langue « {} » référence la police « {font} », absente de assets/fonts/",
                    language.id
                );
            }
        }
    }

    #[test]
    fn a_non_latin_script_offers_several_tracings() {
        // Un seul tracé ne permet pas d'apprendre à reconnaître un signe
        // ailleurs que dans la police du jeu.
        let catalog = load_catalog().expect("catalogue valide");

        for language in &catalog.languages {
            assert!(
                language.fonts.len() >= 2,
                "« {} » n'a qu'un tracé : le tirage aléatoire n'aurait rien à tirer",
                language.id
            );
        }
    }
}
