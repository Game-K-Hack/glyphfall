// Glyphfall — apprendre le hangeul, les kana et les kanji en jouant.
// Copyright (C) 2026 Game K
//
// Ce programme est un logiciel libre : vous pouvez le redistribuer et le
// modifier selon les termes de la GNU General Public License, version 3,
// publiée par la Free Software Foundation.
//
// Il est distribué dans l'espoir d'être utile, mais SANS AUCUNE GARANTIE,
// pas même la garantie implicite de VALEUR MARCHANDE ou d'ADÉQUATION À UN
// USAGE PARTICULIER. Voir la GNU General Public License pour les détails.
//
// Vous devriez avoir reçu une copie de la licence avec ce programme, dans
// le fichier LICENSE. Sinon, voir <https://www.gnu.org/licenses/>.

//! Le lanceur du bureau et du navigateur.
//!
//! Tout le programme vit dans la bibliothèque : Android ne démarre pas un
//! processus mais charge une `.so`, et une bibliothèque sert les deux.

fn main() {
    glyphfall_core::start();
}
