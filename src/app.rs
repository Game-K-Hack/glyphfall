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

use crate::data::Catalog;
use crate::gfx::Fonts;
use crate::progress::Progress;
use crate::session::{Outcome, Session};

/// Ce qui vit pour toute la durée du programme.
pub struct App {
    pub catalog: Catalog,
    pub fonts: Fonts,
    pub progress: Progress,
}

pub enum Screen {
    Title,
    /// `selected` survit d'une frame à l'autre : c'est la carte mise en avant.
    LanguageSelect {
        selected: usize,
    },
    LearningPath {
        language: String,
        selected: usize,
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
    /// Revenir à la racine, quel que soit le nombre d'écrans empilés.
    ToRoot,
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
            Transition::ToRoot => self.stack.truncate(1),
            Transition::Quit => return false,
        }
        true
    }
}
