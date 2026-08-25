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

    #[cfg(not(target_os = "android"))]
    use directories::ProjectDirs;

    /// Le dossier où le jeu a le droit d'écrire.
    ///
    /// Sur Android, `ProjectDirs` répond un chemin de bureau — `~/.local/share`
    /// et consorts — auquel une application n'a pas accès : la sauvegarde
    /// échouait en silence, et la progression repartait de zéro à chaque
    /// ouverture. Chaque application y dispose en revanche d'un dossier privé,
    /// à un emplacement dérivé de son nom de paquet.
    #[cfg(target_os = "android")]
    fn save_dir() -> Option<PathBuf> {
        // Doit correspondre à `applicationId` dans android/app/build.gradle.
        const PAQUET: &str = "fr.harlock.glyphfall";

        // `/data/user/0` est la forme moderne, `/data/data` son ancien alias.
        // Les deux mènent au même endroit sur un téléphone à un seul profil,
        // mais seule la première est juste sur un téléphone qui en a plusieurs.
        let candidats =
            [format!("/data/user/0/{PAQUET}/files"), format!("/data/data/{PAQUET}/files")];

        candidats
            .into_iter()
            .map(PathBuf::from)
            .find(|chemin| {
                chemin.parent().is_some_and(|parent| parent.exists())
                    && std::fs::create_dir_all(chemin).is_ok()
            })
    }

    #[cfg(not(target_os = "android"))]
    fn save_dir() -> Option<PathBuf> {
        Some(ProjectDirs::from("", "", "Glyphfall")?.data_dir().to_path_buf())
    }

    fn save_path(name: &str) -> Option<PathBuf> {
        Some(save_dir()?.join(name))
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
