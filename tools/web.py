# -*- coding: utf-8 -*-
"""Assemble la version navigateur.

    python tools/web.py [--sans-compiler]

Le jeu est compilé pour `wasm32-unknown-unknown`, puis déposé dans
`target/web/` avec la page, les scripts du moteur et les ressources que le
binaire n'embarque plus.

Ce qui n'est pas embarqué et pourquoi : le wasm ne porte que ce qui sert au
premier écran — polices et leçons, trois mégaoctets. Musiques et voix
pèseraient soixante-huit de plus, à télécharger avant d'avoir rien vu ; elles
sont donc recopiées à côté de la page et récupérées à la demande, à l'adresse
exacte que `data::asset_bytes` réclame.

Les scripts du moteur sont pris dans les caisses elles-mêmes plutôt que dans
des copies : une copie oubliée dérive au premier changement de version.
"""
import argparse
import glob
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from version import version as numero_de_version  # noqa: E402

RACINE = Path(__file__).resolve().parent.parent
SOURCE = RACINE / "web"
SORTIE = RACINE / "target" / "web"
CIBLE = "wasm32-unknown-unknown"

# Les ressources laissées hors du binaire, recopiées telles quelles.
RESSOURCES = ["assets/music", "assets/voices"]


def executer(commande, **kwargs):
    print("  " + " ".join(str(morceau) for morceau in commande))
    resultat = subprocess.run(commande, **kwargs)
    if resultat.returncode != 0:
        sys.exit(resultat.returncode)


def caisse(nom):
    """Le dossier d'une caisse dans le registre local."""
    motif = str(Path.home() / ".cargo/registry/src/*" / f"{nom}-*")
    trouves = sorted(glob.glob(motif))
    if not trouves:
        sys.exit(f"caisse introuvable dans le registre : {nom}")
    return Path(trouves[-1])


def scripts():
    """Où trouver chaque script du moteur.

    `quad-storage` ne publie pas le sien dans sa caisse : celui du dépôt est la
    seule source, et il ne bouge pas.
    """
    return [
        (caisse("miniquad") / "js" / "gl.js", "gl.js"),
        (caisse("sapp-jsutils") / "js" / "sapp_jsutils.js", "sapp_jsutils.js"),
        (RACINE / "patches/quad-snd/js/audio.js", "audio.js"),
        (SOURCE / "quad-storage.js", "quad-storage.js"),
    ]


def icones():
    """L'icône de l'onglet, et celle des écrans d'accueil mobiles.

    L'`.ico` est celui du programme d'installation Windows : six tailles, du
    16 au 256, toutes agrandies au plus proche voisin depuis l'icône du jeu.
    Un seul fichier pour deux usages, plutôt qu'une copie qui dériverait.

    Le PNG de 192, réclamé par Android et iOS pour l'écran d'accueil, est
    versionné plutôt que fabriqué ici. Il ne changera qu'avec l'icône du jeu,
    et le produire demandait Pillow — une dépendance que le coureur
    d'intégration n'a pas, et qui faisait échouer la construction.
    """
    shutil.copy2(RACINE / "installer/glyphfall.ico", SORTIE / "favicon.ico")


def poids(chemin):
    if chemin.is_file():
        return chemin.stat().st_size
    return sum(p.stat().st_size for p in chemin.rglob("*") if p.is_file())


def main():
    arguments = argparse.ArgumentParser(description="Assemble la version navigateur.")
    arguments.add_argument("--version", help="numéro à graver ; sinon Cargo.toml")
    arguments.add_argument("--sans-compiler", action="store_true",
                           help="réutilise le wasm déjà présent")
    choix = arguments.parse_args()

    print(f"Glyphfall pour le navigateur, version {numero_de_version(choix.version)}")

    if not choix.sans_compiler:
        executer(["rustup", "target", "add", CIBLE], cwd=RACINE)
        # `--bin` et non la compilation complete : la caisse produit aussi une
        # bibliotheque partagee, qui n'existe que pour Android — l'activite
        # Java y charge `quad_main`. Pour le navigateur elle ne sert a rien, et
        # les versions recentes de Rust refusent de l'assembler : elle laisse
        # indefinis les symboles que `gl.js` ne fournit qu'a l'execution
        # (`sapp_set_cursor`, `fs_take_buffer`...). L'executable, lui, a le
        # droit de les attendre.
        executer(["cargo", "build", "--release", "--target", CIBLE, "--bin", "glyphfall"],
                 cwd=RACINE)

    wasm = RACINE / "target" / CIBLE / "release" / "glyphfall.wasm"
    if not wasm.exists():
        sys.exit(f"{wasm} est absent : compilez d'abord")

    if SORTIE.exists():
        shutil.rmtree(SORTIE)
    SORTIE.mkdir(parents=True)

    shutil.copy2(wasm, SORTIE / "glyphfall.wasm")
    for page in SOURCE.glob("*.html"):
        shutil.copy2(page, SORTIE / page.name)
    for image in SOURCE.glob("*.png"):
        shutil.copy2(image, SORTIE / image.name)
    for source, nom in scripts():
        if not source.exists():
            sys.exit(f"script introuvable : {source}")
        shutil.copy2(source, SORTIE / nom)

    icones()

    for ressource in RESSOURCES:
        depart = RACINE / ressource
        if not depart.exists():
            continue
        # Seuls les sons : les notices et les listes ne servent pas au
        # navigateur, qui lit le catalogue depuis le binaire.
        for fichier in depart.rglob("*"):
            if not fichier.is_file() or fichier.suffix.lower() not in (".mp3", ".ogg", ".wav"):
                continue
            arrivee = SORTIE / fichier.relative_to(RACINE)
            arrivee.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(fichier, arrivee)

    print(f"\n{SORTIE}")
    for nom in ["glyphfall.wasm", "assets/music", "assets/voices"]:
        chemin = SORTIE / nom
        if chemin.exists():
            print(f"  {nom:<22} {poids(chemin) / 1048576:6.1f} Mo")
    print(f"  {'total':<22} {poids(SORTIE) / 1048576:6.1f} Mo")
    print(f"\n  essai local :  python -m http.server --directory {SORTIE}")


if __name__ == "__main__":
    main()
