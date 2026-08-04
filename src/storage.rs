//! Lecture et écriture de petits fichiers de sauvegarde, quelle que soit la
//! plateforme.
//!
//! Sur le bureau ce sont des fichiers dans le dossier de données de
//! l'utilisateur ; en WebAssembly il n'y a pas de système de fichiers, on passe
//! par le stockage local du navigateur. Le reste du programme ne voit que `read`
//! et `write`, avec un nom de fichier.
//!
//! Aucune de ces opérations n'échoue bruyamment : perdre une sauvegarde est
//! ennuyeux, empêcher de jouer l'est davantage.

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use std::path::PathBuf;

    use directories::ProjectDirs;

    fn save_path(name: &str) -> Option<PathBuf> {
        let dirs = ProjectDirs::from("", "", "AlphaTiles")?;
        Some(dirs.data_dir().join(name))
    }

    pub fn read(name: &str) -> Option<String> {
        std::fs::read_to_string(save_path(name)?).ok()
    }

    pub fn write(name: &str, content: &str) {
        let Some(path) = save_path(name) else { return };

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
    pub fn read(name: &str) -> Option<String> {
        quad_storage::STORAGE.lock().ok()?.get(name)
    }

    pub fn write(name: &str, content: &str) {
        if let Ok(mut storage) = quad_storage::STORAGE.lock() {
            storage.set(name, content);
        }
    }
}

pub use backend::{read, write};
