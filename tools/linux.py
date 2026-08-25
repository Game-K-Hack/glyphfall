# -*- coding: utf-8 -*-
"""Construit l'exécutable Linux depuis n'importe quel système, grâce à Zig.

Rien du jeu n'est propre à Windows : c'est la chaîne de compilation qui manque.
Zig en fournit une, complète et croisée, sans machine virtuelle ni conteneur.

    pip install ziglang
    cargo install cargo-zigbuild
    python tools/linux.py

Le binaire produit embarque tout — langues, polices, musique : il n'y a rien à
installer à côté. Il demande une glibc 2.31 ou plus récente, soit Ubuntu 20.04
et au-delà.

Ce dont il a besoin sur la machine du joueur, et qu'aucun bureau Linux n'a
besoin d'installer : `libasound2`, seule dépendance inscrite dans le fichier,
puis `libX11`, `libXi`, `libGL` et `libxkbcommon`, que miniquad ouvre à
l'exécution — elles n'apparaissent donc pas dans la liste des dépendances, et
leur absence ne se voit qu'au lancement.
"""
import argparse
import glob
import os
import re
import shutil
import struct
import subprocess
import sys
import tarfile
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent
SORTIE = RACINE / "target" / "linux"

# Les architectures visées : le triplet Rust, celui de Zig, et le nom que
# Debian leur donne.
ARCHITECTURES = {
    "x86_64": ("x86_64-unknown-linux-gnu", "x86_64-linux-gnu", "amd64"),
    "arm64": ("aarch64-unknown-linux-gnu", "aarch64-linux-gnu", "arm64"),
}

# La glibc la plus ancienne acceptée. Zig fabrique les symboles de cette
# version-là, ce qui fait tourner le binaire sur tout ce qui est plus récent.
GLIBC = "2.31"


def executer(commande, **kwargs):
    resultat = subprocess.run([str(part) for part in commande], **kwargs)
    if resultat.returncode != 0:
        sys.exit(f"échec : {' '.join(str(p) for p in commande[:3])}…")
    return resultat


def zig():
    """Zig, sur le chemin ou installé comme module Python."""
    if trouve := shutil.which("zig"):
        return Path(trouve).parent

    try:
        import ziglang
    except ImportError:
        sys.exit("Zig introuvable : `pip install ziglang`, ou posez-le sur le PATH.")

    return Path(ziglang.__file__).parent


def bouchon_alsa(cible_zig):
    """Fabrique une fausse `libasound.so`, pour l'édition de liens seulement.

    `quad-alsa-sys` réclame ALSA au lieur, alors que le son n'est ouvert qu'à
    l'exécution. Fournir la vraie bibliothèque obligerait à récupérer un paquet
    Debian ; il suffit d'un fichier qui déclare les mêmes symboles, sans rien en
    faire. Le nom interne annoncé — `libasound.so.2` — est celui que le binaire
    ira chercher chez le joueur, où la vraie répondra.
    """
    sources = glob.glob(os.path.expanduser("~/.cargo/registry/src/*/quad-alsa-sys-*/src"))
    if not sources:
        sys.exit("quad-alsa-sys introuvable : lancez d'abord `cargo fetch`.")

    noms = set()
    for fichier in glob.glob(sources[0] + "/*.rs"):
        contenu = Path(fichier).read_text(encoding="utf-8")
        noms |= set(re.findall(r"pub fn (snd_[A-Za-z_0-9]+)", contenu))

    dossier = SORTIE / "alsa" / cible_zig
    dossier.mkdir(parents=True, exist_ok=True)
    source = dossier / "bouchon.c"
    source.write_text("\n".join(f"void {nom}(void) {{}}" for nom in sorted(noms)) + "\n")

    print(f"  bouchon ALSA ({len(noms)} symboles)")
    executer([
        zig() / "zig", "cc", "-target", cible_zig,
        "-shared", "-fPIC", "-Wl,-soname,libasound.so.2",
        "-o", dossier / "libasound.so", source,
    ])
    return dossier


def dependances(binaire):
    """Les bibliothèques que le chargeur exigera, lues dans l'ELF."""
    octets = binaire.read_bytes()
    if octets[:4] != b"\x7fELF":
        sys.exit("le fichier produit n'est pas un ELF")

    debut, = struct.unpack_from("<Q", octets, 0x28)
    taille, nombre, _ = struct.unpack_from("<HHH", octets, 0x3A)
    sections = [
        struct.unpack_from("<IIQQQQIIQQ", octets, debut + index * taille)
        for index in range(nombre)
    ]

    dynamique = next(s for s in sections if s[1] == 6)      # SHT_DYNAMIC
    textes = sections[dynamique[6]]                          # la table de chaînes liée

    def chaine(decalage):
        depart = textes[4] + decalage
        return octets[depart:octets.index(b"\0", depart)].decode()

    besoins = []
    for index in range(dynamique[5] // 16):
        etiquette, valeur = struct.unpack_from("<qQ", octets, dynamique[4] + index * 16)
        if etiquette == 0:
            break
        if etiquette == 1:                                   # DT_NEEDED
            besoins.append(chaine(valeur))
    return besoins


def main():
    arguments = argparse.ArgumentParser(description="Construit l'exécutable Linux.")
    arguments.add_argument(
        "--arch",
        choices=sorted(ARCHITECTURES),
        default="x86_64",
        help="architecture visée (x86_64 par défaut)",
    )
    arch = arguments.parse_args().arch
    cible, cible_zig, _ = ARCHITECTURES[arch]

    print(f"Glyphfall pour Linux ({arch})")
    SORTIE.mkdir(parents=True, exist_ok=True)
    dossier = bouchon_alsa(cible_zig)

    environnement = dict(os.environ)
    environnement["PATH"] = f"{zig()}{os.pathsep}{environnement['PATH']}"
    environnement["RUSTFLAGS"] = f"{environnement.get('RUSTFLAGS', '')} -L {dossier}".strip()

    print(f"  compilation {cible} (glibc {GLIBC})")
    executer(
        ["cargo", "zigbuild", "--release", "--target", f"{cible}.{GLIBC}"],
        cwd=RACINE,
        env=environnement,
    )

    produit = RACINE / "target" / cible / "release" / "glyphfall"
    binaire = SORTIE / arch / "glyphfall"
    binaire.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(produit, binaire)

    archive = SORTIE / f"glyphfall-linux-{arch}.tar.gz"
    with tarfile.open(archive, "w:gz") as tar:
        info = tar.gettarinfo(binaire, arcname="glyphfall")
        info.mode = 0o755                                    # exécutable après extraction
        with binaire.open("rb") as flux:
            tar.addfile(info, flux)

    print(f"\n{binaire}  ({binaire.stat().st_size / 1048576:.0f} Mo)")
    print(f"{archive}  ({archive.stat().st_size / 1048576:.0f} Mo)")
    print("  dépendances :", ", ".join(dependances(binaire)))
    print("  lancement   :  chmod +x glyphfall && ./glyphfall")


if __name__ == "__main__":
    main()
