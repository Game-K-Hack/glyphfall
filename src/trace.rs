//! Ce que le jeu fait, écrit au fil de l'eau, pour scripter une prise.
//!
//! Filmer une bande-annonce demande de répondre juste, donc de savoir quel
//! signe tombe et à quel instant. On peut le deviner en regardant les images
//! une à une ; il est plus court de laisser le jeu le dire.
//!
//! `GLYPHFALL_TRACE=fichier.txt` note chaque apparition, horodatée sur la même
//! horloge que les images du film. Une prise à vide suffit alors à écrire le
//! scénario de la suivante — la graine garantissant que la manche se rejoue à
//! l'identique.
//!
//! Sans la variable, rien n'est ouvert et rien n'est écrit.

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;

fn fichier() -> Option<&'static String> {
    static CHEMIN: OnceLock<Option<String>> = OnceLock::new();
    CHEMIN
        .get_or_init(|| {
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let chemin = std::env::var("GLYPHFALL_TRACE").ok()?;
                let _ = std::fs::write(&chemin, "");
                Some(chemin)
            }
        })
        .as_ref()
}

/// Note un événement, précédé de l'instant absolu où il se produit.
pub fn note(evenement: &str) {
    let Some(chemin) = fichier() else { return };
    let instant = macroquad::miniquad::date::now();

    if let Ok(mut sortie) = OpenOptions::new().append(true).open(chemin) {
        let _ = writeln!(sortie, "{instant:.6} {evenement}");
    }
}
