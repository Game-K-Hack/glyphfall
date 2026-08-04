//! Lecture et écriture d'un petit fichier de sauvegarde, quelle que soit la
//! plateforme.
//!
//! Sur le bureau c'est un fichier dans le dossier de données de l'utilisateur ;
//! en WebAssembly il n'y a pas de système de fichiers, on passe par le stockage
//! local du navigateur. Le reste du programme ne voit que `read` et `write`.
//!
//! Aucune de ces opérations n'échoue bruyamment : perdre une progression est
//! ennuyeux, empêcher de jouer l'est davantage.

/// Nom du fichier, et clé de stockage en navigateur.
const SAVE_NAME: &str = "progress.toml";

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::PathBuf;

    use directories::ProjectDirs;

    use super::SAVE_NAME;

    fn save_path() -> Option<PathBuf> {
        let dirs = ProjectDirs::from("", "", "AlphaTiles")?;
        Some(dirs.data_dir().join(SAVE_NAME))
    }

    pub fn read() -> Option<String> {
        std::fs::read_to_string(save_path()?).ok()
    }

    pub fn write(content: &str) {
        let Some(path) = save_path() else { return };

        if let Some(parent) = path.parent() {
            // Au premier lancement, le dossier de données n'existe pas encore.
            if std::fs::create_dir_all(parent).is_err() {
                return;
            }
        }

        let _ = std::fs::write(path, content);
    }
}

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::SAVE_NAME;

    pub fn read() -> Option<String> {
        quad_storage::STORAGE.lock().ok()?.get(SAVE_NAME)
    }

    pub fn write(content: &str) {
        if let Ok(mut storage) = quad_storage::STORAGE.lock() {
            storage.set(SAVE_NAME, content);
        }
    }
}

pub use backend::{read, write};
