//! Le lanceur du bureau et du navigateur.
//!
//! Tout le programme vit dans la bibliothèque : Android ne démarre pas un
//! processus mais charge une `.so`, et une bibliothèque sert les deux.

fn main() {
    glyphfall_core::start();
}
