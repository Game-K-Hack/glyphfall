# -*- coding: utf-8 -*-
"""Empaquette Glyphfall pour Debian et Ubuntu, depuis n'importe quel système.

    python tools/linux.py     # d'abord le binaire
    python tools/deb.py       # puis son paquet

Un `.deb` n'est qu'une archive `ar` de trois membres, dans un ordre imposé :
la version du format, les métadonnées, les fichiers. Aucun outil Debian n'est
donc nécessaire pour en produire un — ce que ce script fait à la main, faute de
`dpkg-deb` sur une machine Windows.

Le paquet s'installe avec `sudo apt install ./glyphfall_<version>_amd64.deb`, ce
qui tire au passage les bibliothèques manquantes. Pour un vrai `apt install
glyphfall`, sans chemin de fichier, il faudrait un dépôt signé et hébergé :
voir le README.
"""
import argparse
import gzip
import hashlib
import io
import shutil
import subprocess
import sys
import tarfile
import time
from pathlib import Path

from version import version as numero_de_version

RACINE = Path(__file__).resolve().parent.parent
LINUX = RACINE / "target" / "linux"
SORTIE = RACINE / "target" / "deb"

PAQUET = "glyphfall"

# Le nom que Debian donne aux architectures, et le dossier où `tools/linux.py`
# dépose le binaire correspondant.
ARCHITECTURES = {"amd64": "x86_64", "arm64": "arm64"}

# Les bibliothèques que le jeu ouvre lui-même, à l'exécution : `dpkg-shlibdeps`
# ne saurait pas les deviner, elles ne figurent pas dans l'exécutable. Sans
# elles, le jeu s'installe puis refuse de démarrer.
# Ce qui fournit reellement un peripherique « default » a ALSA.
#
# `libasound2` n'est que la bibliotheque : sur une machine de bureau moderne,
# le peripherique « default » est un pont vers PipeWire ou PulseAudio, apporte
# par un paquet separe. Sans lui, la bibliotheque se charge mais n'ouvre rien,
# et le fil audio du moteur meurt en annoncant « Audio thread died ».
#
# En Recommends plutot qu'en Depends : un bureau complet les a deja, et le jeu
# se lance sans son plutot que de refuser de s'installer.
RECOMMANDATIONS = [
    "pipewire-alsa | libasound2-plugins",
]

DEPENDANCES = [
    "libc6 (>= 2.31)",
    # Renommée dans les versions récentes, où le nom d'origine n'existe plus.
    "libasound2t64 | libasound2",
    "libx11-6",
    "libxi6",
    "libgl1",
    "libxkbcommon0",
]

RACCOURCI = """[Desktop Entry]
Type=Application
Name=Glyphfall
GenericName=Apprentissage des écritures
Comment=Apprendre le hangeul, les kana et les kanji en jouant
Exec=/usr/games/glyphfall
Icon=glyphfall
Terminal=false
Categories=Game;LogicGame;Education;
Keywords=japonais;coréen;kana;kanji;hangeul;
"""

LICENCE = """Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: Glyphfall

Files: *
Copyright: Glyphfall
License: à préciser

Files: assets/fonts/*
Copyright: Google et leurs auteurs respectifs
License: OFL-1.1
 Polices Noto, Nanum, Gaegu, Jua, Do Hyeon, Klee One, Yusei Magic,
 Zen Maru Gothic, RocknRoll One et Shippori Mincho, sous SIL Open Font
 License 1.1, réduites aux signes du catalogue.

Files: assets/*
Copyright: palette « Sweetie 16 » de GrafxKid
License: public-domain
"""


def icones():
    """Les tailles à installer, toutes multiples de la source.

    Agrandir au plus proche et par un facteur entier : c'est du pixel, une
    interpolation le rendrait flou là où tout le jeu tient à sa netteté.
    """
    from PIL import Image

    base = Image.open(RACINE / "src/assets/icons/icon64.png")
    rendus = {}
    for taille in (16, 32, 64, 128, 256):
        source = RACINE / f"src/assets/icons/icon{taille}.png"
        image = Image.open(source) if source.exists() else base.resize(
            (taille, taille), Image.NEAREST
        )
        tampon = io.BytesIO()
        image.save(tampon, "PNG")
        rendus[taille] = tampon.getvalue()
    return rendus


def dossiers(fichiers):
    """Les dossiers à déclarer avant les fichiers qu'ils contiennent.

    dpkg ne crée pas les dossiers manquants : il les attend dans l'archive, et
    échoue sur le premier fichier dont le dossier n'existe pas encore sur la
    machine. `/usr/games` existe partout, `/usr/share/applications` non.
    """
    connus = set()
    for chemin, _, _ in fichiers:
        parts = chemin.split("/")[1:-1]
        for profondeur in range(1, len(parts) + 1):
            connus.add("./" + "/".join(parts[:profondeur]))

    return [(chemin, None, 0o755) for chemin in sorted(connus)]


def archive_tar(fichiers):
    """Un `.tar.gz` aux droits de root, comme l'exige un paquet."""
    brut = io.BytesIO()
    with tarfile.open(fileobj=brut, mode="w") as tar:
        for chemin, contenu, mode in dossiers(fichiers) + list(fichiers):
            info = tarfile.TarInfo(chemin)
            info.type = tarfile.DIRTYPE if contenu is None else tarfile.REGTYPE
            info.size = 0 if contenu is None else len(contenu)
            info.mode = mode
            info.mtime = int(time.time())
            # Les fichiers d'un paquet appartiennent à root, quel que soit
            # l'utilisateur qui l'a fabriqué.
            info.uid = info.gid = 0
            info.uname = info.gname = "root"
            tar.addfile(info, None if contenu is None else io.BytesIO(contenu))

    return gzip.compress(brut.getvalue())


def archive_ar(membres, destination):
    """L'enveloppe d'un `.deb` : une archive `ar` en clair."""
    with destination.open("wb") as sortie:
        sortie.write(b"!<arch>\n")
        for nom, contenu in membres:
            entete = (
                f"{nom:<16}{int(time.time()):<12}{0:<6}{0:<6}{100644:<8}{len(contenu):<10}"
            ).encode() + b"`\n"
            sortie.write(entete)
            sortie.write(contenu)
            # Chaque membre commence sur un octet pair.
            if len(contenu) % 2:
                sortie.write(b"\n")


def main():
    arguments = argparse.ArgumentParser(description="Empaquette Glyphfall pour Debian.")
    arguments.add_argument(
        "--arch",
        choices=sorted(ARCHITECTURES),
        default="amd64",
        help="architecture du paquet (amd64 par défaut)",
    )
    arguments.add_argument("--version", help="numéro à graver ; celui de Cargo.toml sinon")
    choix = arguments.parse_args()
    architecture = choix.arch
    version = numero_de_version(choix.version)
    dossier = ARCHITECTURES[architecture]

    binaire = LINUX / dossier / "glyphfall"
    if not binaire.exists():
        sys.exit(f"binaire absent : lancez `python tools/linux.py --arch {dossier}`.")

    print(f"Glyphfall pour Debian ({architecture})")
    SORTIE.mkdir(parents=True, exist_ok=True)

    executable = binaire.read_bytes()
    fichiers = [
        # Debian range les jeux à part, et `/usr/games` est sur le chemin.
        (f"./usr/games/{PAQUET}", executable, 0o755),
        (f"./usr/share/applications/{PAQUET}.desktop", RACCOURCI.encode(), 0o644),
        (f"./usr/share/doc/{PAQUET}/copyright", LICENCE.encode(), 0o644),
    ]
    for taille, contenu in icones().items():
        fichiers.append((
            f"./usr/share/icons/hicolor/{taille}x{taille}/apps/{PAQUET}.png",
            contenu,
            0o644,
        ))

    donnees = archive_tar(fichiers)

    poids = sum(len(contenu) for _, contenu, _ in fichiers) // 1024
    controle = (
        f"Package: {PAQUET}\n"
        f"Version: {version}\n"
        f"Architecture: {architecture}\n"
        "Maintainer: Harlock <harlock7@laposte.net>\n"
        f"Installed-Size: {poids}\n"
        f"Depends: {', '.join(DEPENDANCES)}\n"
        f"Recommends: {', '.join(RECOMMANDATIONS)}\n"
        "Section: games\n"
        "Priority: optional\n"
        "Description: Apprendre les ecritures non latines en jouant\n"
        " Les signes tombent, le joueur tape leur lecture avant qu'ils ne\n"
        " franchissent la ligne. Hangeul, hiragana, katakana et kanji, avec un\n"
        " chemin d'apprentissage progressif et une interface 8-bit.\n"
    )
    sommes = "".join(
        f"{hashlib.md5(contenu).hexdigest()}  {chemin[2:]}\n"
        for chemin, contenu, _ in fichiers
    )
    metadonnees = archive_tar([
        ("./control", controle.encode(), 0o644),
        ("./md5sums", sommes.encode(), 0o644),
    ])

    paquet = SORTIE / f"{PAQUET}_{version}_{architecture}.deb"
    # L'ordre des trois membres est impose par le format.
    archive_ar([
        ("debian-binary", b"2.0\n"),
        ("control.tar.gz", metadonnees),
        ("data.tar.gz", donnees),
    ], paquet)

    print(f"\n{paquet}  ({paquet.stat().st_size / 1048576:.0f} Mo)")
    print(f"  installation :  sudo apt install ./{paquet.name}")


if __name__ == "__main__":
    main()
