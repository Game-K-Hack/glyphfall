//! L'état global et la navigation entre écrans.
//!
//! Les écrans forment une **pile** plutôt qu'une simple variable : revenir en
//! arrière depuis le briefing doit ramener au chemin d'apprentissage, qui doit
//! lui-même ramener au choix de la langue. Avec un seul champ « écran courant »,
//! chaque écran devrait savoir d'où on vient — la pile s'en souvient à sa place.
//!
//! Un écran ne modifie jamais la pile directement : il renvoie une `Transition`
//! que la boucle principale applique. Cela évite qu'un écran se retire du
//! dessous de ses propres pieds au milieu de son rendu.

use crate::audio::Sfx;
use crate::data::Catalog;
use crate::music::Music;
use crate::gfx::Fonts;
use crate::progress::Progress;
use crate::settings::Settings;
use crate::session::{Outcome, Session};

/// Ce qui vit pour toute la durée du programme.
pub struct App {
    pub catalog: Catalog,
    pub fonts: Fonts,
    pub sfx: Sfx,
    pub music: Music,
    pub progress: Progress,
    pub settings: Settings,
}

pub enum Screen {
    Title,
    /// `selected` survit d'une frame à l'autre : c'est la carte mise en avant.
    LanguageSelect {
        selected: usize,
    },
    /// `selected` retient la ligne de réglage mise en avant.
    Options {
        selected: usize,
    },
    LearningPath {
        language: String,
        /// `None` tant que l'écran n'a pas choisi où se placer : il se cale
        /// alors sur l'étape en cours plutôt que sur la première.
        selected: Option<usize>,
    },
    Briefing {
        language: String,
        level: String,
    },
    /// Les manches et leurs bilans sont volumineux : les mettre en boite
    /// garde l'enumeration compacte, elle qui est copiee a chaque transition.
    Playing(Box<Session>),
    /// `elapsed` fait apparaitre les etoiles une a une.
    Results {
        outcome: Box<Outcome>,
        elapsed: f32,
    },
}

/// Ce qu'un écran demande à la boucle principale de faire de la pile.
#[must_use]
pub enum Transition {
    /// Rester sur cet écran.
    Stay,
    /// Empiler un écran, en gardant celui-ci en dessous.
    Push(Screen),
    /// Revenir à l'écran précédent.
    Pop,
    /// Remplacer cet écran : la partie relancée ne doit pas s'empiler sur la
    /// précédente, sinon « retour » traverserait toutes les tentatives.
    Replace(Screen),
    /// Remonter de plusieurs crans d'un coup, quand l'écran visé n'est pas
    /// juste en dessous. Ne dépile jamais la racine.
    PopMany(usize),
    Quit,
}

pub struct Navigator {
    stack: Vec<Screen>,
}

impl Navigator {
    pub fn new(root: Screen) -> Self {
        Self { stack: vec![root] }
    }

    /// L'écran affiché. La pile n'est jamais vide : la racine y reste toujours.
    pub fn top_mut(&mut self) -> &mut Screen {
        self.stack.last_mut().expect("la pile d'écrans garde toujours sa racine")
    }

    /// Y a-t-il un écran en dessous vers lequel revenir ?
    pub fn can_go_back(&self) -> bool {
        self.stack.len() > 1
    }

    /// Applique une transition. Renvoie `false` quand le jeu doit s'arrêter.
    pub fn apply(&mut self, transition: Transition) -> bool {
        match transition {
            Transition::Stay => {}
            Transition::Push(screen) => self.stack.push(screen),
            Transition::Pop => {
                // On ne dépile jamais la racine : sans elle, plus rien à afficher.
                if self.can_go_back() {
                    self.stack.pop();
                }
            }
            Transition::Replace(screen) => {
                self.stack.pop();
                self.stack.push(screen);
            }
            Transition::PopMany(count) => {
                let keep = self.stack.len().saturating_sub(count).max(1);
                self.stack.truncate(keep);
            }
            Transition::Quit => return false,
        }
        true
    }

    #[cfg(test)]
    fn depth(&self) -> usize {
        self.stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn navigator() -> Navigator {
        let mut navigator = Navigator::new(Screen::Title);
        navigator.apply(Transition::Push(Screen::LanguageSelect { selected: 0 }));
        navigator.apply(Transition::Push(Screen::LearningPath {
            language: "ko".into(),
            selected: None,
        }));
        navigator
    }

    #[test]
    fn the_root_is_never_popped() {
        // Depiler la racine ne laisserait plus rien a afficher.
        let mut navigator = Navigator::new(Screen::Title);

        assert!(!navigator.can_go_back());
        navigator.apply(Transition::Pop);

        assert_eq!(navigator.depth(), 1);
    }

    #[test]
    fn popping_many_stops_at_the_root() {
        let mut navigator = navigator();

        navigator.apply(Transition::PopMany(99));

        assert_eq!(navigator.depth(), 1);
    }

    #[test]
    fn popping_many_goes_back_several_screens_at_once() {
        // Le bouton « chemin » de l'ecran de resultats saute par-dessus le
        // briefing : sans cela il ne tiendrait pas ce que son nom promet.
        let mut navigator = navigator();
        navigator.apply(Transition::Push(Screen::Briefing {
            language: "ko".into(),
            level: "ko-01".into(),
        }));
        let before = navigator.depth();

        navigator.apply(Transition::PopMany(2));

        assert_eq!(navigator.depth(), before - 2);
        assert!(matches!(navigator.top_mut(), Screen::LanguageSelect { .. }));
    }

    #[test]
    fn replacing_keeps_the_stack_depth() {
        // Rejouer ne doit pas empiler les tentatives les unes sur les autres.
        let mut navigator = navigator();
        let before = navigator.depth();

        navigator.apply(Transition::Replace(Screen::Title));

        assert_eq!(navigator.depth(), before);
    }

    #[test]
    fn quitting_stops_the_loop() {
        let mut navigator = navigator();

        assert!(!navigator.apply(Transition::Quit));
    }
}
