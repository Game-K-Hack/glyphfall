# -*- coding: utf-8 -*-
"""Fabrique le programme d'installation Windows.

    python tools/setup.py [--version 0.3.0] [--sans-compiler]

Le jeu est d'abord compilé en publication, puis Inno Setup assemble
`installer/glyphfall.iss` autour de lui.

Inno Setup plutôt que NSIS : celui-ci affiche « Nullsoft Install System » au
bas de chaque page. Inno n'appose aucune marque, et sa traduction française
est livrée avec l'outil.

S'il n'est pas installé sur la machine, on le récupère et on l'extrait en mode
portable dans `target/inno`, sans rien poser dans le registre ni dans le menu
Démarrer. C'est ce qui permet à ce script de tourner aussi bien sur une machine
de développement que sur une machine d'intégration continue.
"""
import argparse
import os
import shutil
import subprocess
import sys
import urllib.request
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from version import version as numero_de_version  # noqa: E402

RACINE = Path(__file__).resolve().parent.parent
SCRIPT = RACINE / "installer" / "glyphfall.iss"
PORTABLE = RACINE / "target" / "inno"

# L'archive officielle, servie par GitHub. La version est figée : une
# construction doit donner le même résultat dans six mois.
TELECHARGEMENT = (
    "https://github.com/jrsoftware/issrc/releases/download/"
    "is-6_7_3/innosetup-6.7.3.exe"
)


def executer(commande, **kwargs):
    print("  " + " ".join(str(morceau) for morceau in commande))
    resultat = subprocess.run(commande, **kwargs)
    if resultat.returncode != 0:
        sys.exit(resultat.returncode)


def compilateur():
    """Le chemin d'ISCC : celui de la machine, ou une copie portable."""
    if trouve := shutil.which("ISCC"):
        return Path(trouve)

    for base in (os.environ.get("ProgramFiles(x86)"), os.environ.get("ProgramFiles")):
        if not base:
            continue
        for majeure in (6, 7):
            candidat = Path(base) / f"Inno Setup {majeure}" / "ISCC.exe"
            if candidat.exists():
                return candidat

    portable = PORTABLE / "ISCC.exe"
    if portable.exists():
        return portable

    print("Inno Setup absent : extraction d'une copie portable")
    PORTABLE.mkdir(parents=True, exist_ok=True)
    archive = PORTABLE / "innosetup.exe"
    urllib.request.urlretrieve(TELECHARGEMENT, archive)

    # `/PORTABLE=1` extrait sans toucher au registre : rien à désinstaller
    # ensuite, et la machine reste telle qu'on l'a trouvée.
    executer([
        archive, "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART",
        "/PORTABLE=1", f"/DIR={PORTABLE}",
    ])
    if not portable.exists():
        sys.exit("l'extraction d'Inno Setup a échoué")
    return portable


def main():
    arguments = argparse.ArgumentParser(description=__doc__)
    arguments.add_argument("--version", help="numéro à graver ; sinon Cargo.toml")
    arguments.add_argument("--sans-compiler", action="store_true",
                           help="réutilise le binaire déjà présent")
    choix = arguments.parse_args()

    if sys.platform != "win32":
        sys.exit("le programme d'installation ne se fabrique que sous Windows")

    numero = numero_de_version(choix.version)

    if not choix.sans_compiler:
        print("Compilation du jeu")
        executer(["cargo", "build", "--release"], cwd=RACINE)

    binaire = RACINE / "target" / "release" / "glyphfall.exe"
    if not binaire.exists():
        sys.exit(f"{binaire} est absent : compilez d'abord")

    print(f"Assemblage, version {numero}")
    executer([compilateur(), f"/DVERSION={numero}", SCRIPT], cwd=RACINE)

    produit = RACINE / "installer" / "glyphfall-windows-x86_64-setup.exe"
    poids = produit.stat().st_size / 1048576
    print(f"\n{produit}  ({poids:.1f} Mo)")


if __name__ == "__main__":
    main()
