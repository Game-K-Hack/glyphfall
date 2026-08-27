//! La voix d'un signe : un vrai locuteur, pas une synthèse.
//!
//! Un moyen mnémotechnique dit à quoi ressemble un signe, la fiche de
//! prononciation dit comment le former. Ni l'un ni l'autre ne remplace de
//! l'avoir entendu — surtout en coréen, où l'écart entre le [k] doux du début
//! de mot et le [g] entre deux voyelles ne se décrit qu'imparfaitement.
//!
//! Les enregistrements sont embarqués dans le binaire comme le reste du
//! contenu, dans `assets/languages/<langue>/voices/`, et un `index.toml` dit
//! quel fichier prononce quel signe. Deux signes peuvent partager un
//! enregistrement : une voyelle prononcée seule est exactement la syllabe que
//! forme le cercle muet.
//!
//! # Pourquoi les kanji se taisent
//!
//! Les deux syllabaires et le coréen ont leurs enregistrements ; les kanji
//! n'en ont pas, et n'en auront pas par ce chemin. On a essayé de fabriquer
//! leurs lectures en recollant des mores — NICHI vaut `に` puis `ち`, dont les
//! sons existent. Le résultat s'entend comme deux mots : chaque more a été
//! captée seule, donc avec l'intonation de fin d'énoncé, et sa fondamentale
//! retombe de cinquante à quatre-vingts hertz. Deux chutes de suite ne font
//! pas un mot, surtout dans une langue où la mélodie distingue les mots.
//!
//! Leurs fiches donnent donc les lectures ON et KUN par écrit, avec un mot où
//! les entendre. Pour leur donner une voix un jour, il faudra de vrais
//! enregistrements de mots, ou une synthèse qui gère l'accent de hauteur.
//!
//! # Pourquoi une file d'attente
//!
//! Charger un son est asynchrone chez macroquad, or les écrans sont dessinés
//! par des fonctions ordinaires. Un écran ne joue donc pas : il **demande**, et
//! la boucle principale, qui a le droit d'attendre, sert la demande à la frame
//! suivante. C'est la même mécanique que la musique, à une différence près —
//! une voix ne dure qu'une seconde, et deux demandes rapprochées doivent se
//! remplacer plutôt que s'empiler.

use std::cell::RefCell;
use std::collections::HashMap;

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound, stop_sound};
use serde::Deserialize;

use crate::data::voice_bytes;
use crate::music::decode_bytes;

/// Le fichier qui dit quel enregistrement prononce quel signe.
#[derive(Deserialize)]
struct Index {
    fichiers: HashMap<String, String>,
}

pub struct Voices {
    /// Langue, puis signe, vers nom de fichier.
    ///
    /// Toutes les langues d'un coup : l'application est unique et le joueur
    /// change d'écriture sans que rien ne se recharge. Seuls les index sont
    /// gardés en mémoire — quelques centaines de noms de fichiers — et les
    /// enregistrements eux-mêmes restent dans le binaire jusqu'à ce qu'on les
    /// demande.
    index: HashMap<String, HashMap<String, String>>,
    /// Ce qu'un écran a demandé pendant la frame, pas encore joué : la langue
    /// et le signe.
    ///
    /// Un `Cell` parce que les écrans reçoivent l'application en partage :
    /// demander à entendre un signe n'est pas une raison de rendre tout le
    /// reste modifiable.
    pending: RefCell<Option<(String, String)>>,
    /// Le dernier son joué, gardé en vie le temps qu'il s'entende — le moteur
    /// audio ne copie pas les échantillons, il pointe dessus.
    playing: RefCell<Option<Sound>>,
    volume: f32,
}

impl Voices {
    /// Charge l'index de chaque langue qui en a un. Une langue sans dossier
    /// `voices/` reste simplement muette, ce qui est le cas des trois
    /// écritures japonaises pour l'instant.
    pub fn load(languages: impl Iterator<Item = String>, volume: f32) -> Self {
        let mut index = HashMap::new();

        for language in languages {
            let Some(fichiers) = voice_bytes(&language, "index.toml")
                .and_then(|octets| std::str::from_utf8(octets).ok())
                .and_then(|texte| toml::from_str::<Index>(texte).ok())
                .map(|index| index.fichiers)
            else {
                continue;
            };
            index.insert(language, fichiers);
        }

        Self { index, pending: RefCell::new(None), playing: RefCell::new(None), volume }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume;
    }

    /// Ce signe a-t-il un enregistrement ?
    ///
    /// Les écrans s'en servent pour ne pas proposer un bouton qui ne dirait
    /// rien. Le cercle coréen `ㅇ` est le cas type : en début de syllabe il ne
    /// se prononce pas, et aucun enregistrement ne peut exister.
    pub fn knows(&self, language: &str, glyph: &str) -> bool {
        self.index.get(language).is_some_and(|voix| voix.contains_key(glyph))
    }

    /// Demande à entendre un signe. Sans effet s'il n'a pas de voix.
    ///
    /// La dernière demande de la frame gagne : deux boutons pressés dans le
    /// même souffle ne doivent pas parler en même temps.
    pub fn request(&self, language: &str, glyph: &str) {
        if self.knows(language, glyph) {
            *self.pending.borrow_mut() = Some((language.to_string(), glyph.to_string()));
        }
    }

    /// Sert la demande en attente. À appeler une fois par frame depuis la
    /// boucle principale, seule à pouvoir attendre.
    pub async fn update(&self) {
        let Some((language, glyph)) = self.pending.borrow_mut().take() else { return };
        let Some(file_name) = self.index.get(&language).and_then(|voix| voix.get(&glyph)) else {
            return;
        };
        let Some(octets) = voice_bytes(&language, file_name) else { return };

        let extension = file_name.rsplit('.').next().unwrap_or("");
        let Some(decoded) = decode_bytes(octets, extension) else { return };
        let Ok(sound) = load_sound_from_bytes(&decoded.wav).await else { return };

        // La voix précédente s'arrête : deux signes prononcés en même temps ne
        // s'entendent ni l'un ni l'autre.
        if let Some(ancienne) = self.playing.borrow_mut().take() {
            stop_sound(&ancienne);
        }

        play_sound(&sound, PlaySoundParams { looped: false, volume: self.volume });
        *self.playing.borrow_mut() = Some(sound);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les voix de toutes les langues qui en déclarent, comme au démarrage.
    fn toutes(catalog: &crate::data::Catalog) -> Voices {
        Voices::load(catalog.languages.iter().map(|langue| langue.id.clone()), 1.0)
    }

    #[test]
    fn chaque_langue_sait_prononcer_ce_qu_elle_enseigne() {
        // Un signe qui tombe sans qu'on puisse l'entendre est un signe qu'on
        // apprend à moitié. Une langue entièrement muette est un choix — le
        // japonais l'a été longtemps — mais une langue qui a des voix doit les
        // avoir toutes, sinon le bouton apparaît et disparaît sans raison
        // visible.
        //
        // Seul `ㅇ` fait exception, et pour une bonne raison : en tête de
        // syllabe, il ne se prononce pas.
        let catalog = crate::data::load_catalog().expect("catalogue valide");
        let voices = toutes(&catalog);

        for langue in &catalog.languages {
            if !voices.index.contains_key(&langue.id) {
                continue;
            }
            for level in &langue.levels {
                for glyph in &level.glyphs {
                    if glyph.char == "ㅇ" {
                        continue;
                    }
                    assert!(
                        voices.knows(&langue.id, &glyph.char),
                        "« {} » (niveau {}) n'a pas d'enregistrement",
                        glyph.char,
                        level.id
                    );
                }
            }
        }
    }

    #[test]
    fn chaque_entree_de_l_index_designe_un_fichier_present() {
        // Les index sont écrits à partir de listes fournies : une faute de
        // frappe y donnerait un bouton muet, sans le moindre message.
        let catalog = crate::data::load_catalog().expect("catalogue valide");
        let voices = toutes(&catalog);
        assert!(!voices.index.is_empty(), "aucune langue n'a de voix");

        for (langue, fichiers) in &voices.index {
            assert!(!fichiers.is_empty(), "l'index de « {langue} » est vide");
            for (glyph, file_name) in fichiers {
                assert!(
                    voice_bytes(langue, file_name).is_some(),
                    "« {glyph} » ({langue}) renvoie à « {file_name} », qui n'est pas embarqué"
                );
            }
        }
    }
}
