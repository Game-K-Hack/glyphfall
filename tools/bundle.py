# -*- coding: utf-8 -*-
"""Construit le bundle Android de Glyphfall, signé, prêt pour Google Play.

Google Play n'accepte plus d'APK pour une nouvelle application : il veut un
*Android App Bundle*, signé d'une clé que vous seul détenez. `android.py`
reste le chemin court — un APK de débogage qu'on pose sur un téléphone en une
commande ; celui-ci est le chemin du magasin.

    python tools/bundle.py

Comme `android.py`, il se passe de Gradle : les mêmes outils du SDK produisent
les ressources et le code, `bundletool` se contente de les ranger au format
que Play attend. Le projet `android/` reste ouvrable dans Android Studio, mais
rien ici n'en dépend.

La clé de signature vient de l'environnement, jamais du dépôt :

    GLYPHFALL_KEYSTORE           le magasin de clés
    GLYPHFALL_KEYSTORE_PASSWORD  son mot de passe
    GLYPHFALL_KEY_ALIAS          l'alias de la clé (« glyphfall » par défaut)
    GLYPHFALL_KEY_PASSWORD       le mot de passe de la clé (celui du magasin
                                 par défaut, ce que fait `keytool` si on ne
                                 lui en demande pas d'autre)

Plutôt que de les retaper, on peut les ranger dans un `signature.env` à la
racine, une variable par ligne ; git l'ignore, comme la clé elle-même. Ce que
l'environnement porte déjà l'emporte sur ce fichier.

Pour créer cette clé, une fois pour toutes :

    keytool -genkeypair -v -keystore glyphfall.jks -alias glyphfall
            -keyalg RSA -keysize 4096 -validity 10000

Gardez ce fichier et son mot de passe hors du dépôt et sauvegardés ailleurs :
Play associe l'application à cette clé pour toujours, et la perdre revient à
ne plus jamais pouvoir la mettre à jour.
"""
import argparse
import hashlib
import os
import shutil
import subprocess
import sys
import urllib.request
import zipfile
from pathlib import Path

import android
from android import (ARCHITECTURES, NOM_LIB, PROJET, SORTIE, charger_env,
                     executer)
from version import version as numero_de_version


# La version de `bundletool` est figée, et son empreinte vérifiée : c'est un
# outil téléchargé qui signe indirectement ce que reçoivent des téléphones.
BUNDLETOOL = "1.18.3"
BUNDLETOOL_URL = (
    "https://github.com/google/bundletool/releases/download/"
    f"{BUNDLETOOL}/bundletool-all-{BUNDLETOOL}.jar"
)
BUNDLETOOL_SHA256 = "a099cfa1543f55593bc2ed16a70a7c67fe54b1747bb7301f37fdfd6d91028e29"


def bundletool():
    """Le chemin du jar, téléchargé au premier appel."""
    jar = SORTIE / f"bundletool-{BUNDLETOOL}.jar"
    if jar.exists() and empreinte(jar) == BUNDLETOOL_SHA256:
        return jar

    SORTIE.mkdir(parents=True, exist_ok=True)
    print(f"  téléchargement de bundletool {BUNDLETOOL}")
    urllib.request.urlretrieve(BUNDLETOOL_URL, jar)

    obtenue = empreinte(jar)
    if obtenue != BUNDLETOOL_SHA256:
        jar.unlink()
        sys.exit(
            f"bundletool : empreinte {obtenue}, attendue {BUNDLETOOL_SHA256}"
        )
    return jar


def empreinte(fichier):
    return hashlib.sha256(fichier.read_bytes()).hexdigest()


def signature():
    """Les paramètres de signature, ou l'explication de ce qui manque."""
    magasin = os.environ.get("GLYPHFALL_KEYSTORE")
    mot_de_passe = os.environ.get("GLYPHFALL_KEYSTORE_PASSWORD")
    if not magasin or not mot_de_passe:
        sys.exit(
            "GLYPHFALL_KEYSTORE et GLYPHFALL_KEYSTORE_PASSWORD sont requis.\n"
            "Pour créer la clé, une fois pour toutes :\n"
            "  keytool -genkeypair -v -keystore glyphfall.jks "
            "-alias glyphfall -keyalg RSA -keysize 4096 -validity 10000"
        )
    if not Path(magasin).exists():
        sys.exit(f"magasin de clés introuvable : {magasin}")

    return {
        "magasin": magasin,
        "mot_de_passe": mot_de_passe,
        "alias": os.environ.get("GLYPHFALL_KEY_ALIAS") or "glyphfall",
        "cle": os.environ.get("GLYPHFALL_KEY_PASSWORD") or mot_de_passe,
    }


def module(travail, abis, o, version):
    """Le module « base », rangé comme `bundletool` l'attend.

    Un bundle n'est pas un APK : ses ressources y sont en protobuf plutôt qu'au
    format binaire d'Android, et chaque partie a son dossier. `aapt2` sait
    produire la première forme avec `--proto-format` ; le reste est un
    déplacement de fichiers.
    """
    source = PROJET / "app/src/main"

    print("  ressources")
    executer([
        o["aapt2"], "compile", "--dir", source / "res",
        "-o", travail / "res.zip",
    ])

    manifeste = android.manifeste_complet(travail, version)

    proto = travail / "proto.apk"
    executer([
        o["aapt2"], "link", "--proto-format",
        "-I", o["android_jar"],
        "--manifest", manifeste,
        "--java", travail / "gen",
        "-o", proto,
        travail / "res.zip",
    ])

    classes = android.dex(travail, o)

    print("  module")
    archive = travail / "base.zip"
    with zipfile.ZipFile(proto) as entree, \
            zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as sortie:
        for membre in entree.namelist():
            if membre == "AndroidManifest.xml":
                destination = "manifest/AndroidManifest.xml"
            elif membre == "resources.pb" or membre.startswith("res/"):
                destination = membre
            else:
                # Tout ce qui n'appartient à aucune catégorie connue se range
                # sous `root/`, d'où Android le retrouvera à la racine de l'APK
                # reconstitué. Le jeu n'y met rien, mais l'ignorer serait
                # perdre un fichier sans le dire.
                destination = f"root/{membre}"
            sortie.writestr(destination, entree.read(membre))

        sortie.write(classes, "dex/classes.dex")
        for abi in abis:
            # Stockées telles quelles : Android les charge alors depuis l'APK,
            # sans les recopier dans le stockage du téléphone.
            sortie.write(
                PROJET / "app/src/main/jniLibs" / abi / NOM_LIB,
                f"lib/{abi}/{NOM_LIB}",
                compress_type=zipfile.ZIP_STORED,
            )

    return archive


def signer(jar, aab, cles):
    """Signe le bundle, puis relit sa signature pour s'en assurer.

    `jarsigner -verify` avertira que le certificat est auto-signé, sans
    horodatage et d'une chaîne invérifiable : c'est normal et voulu. Une clé
    de téléversement Android est auto-signée par construction — personne ne la
    contresigne —, et c'est Play qui resignera l'application avec sa propre
    clé avant de la distribuer.
    """
    print("  signature")
    signe = SORTIE / "glyphfall.aab"
    signe.unlink(missing_ok=True)

    # `jarsigner` écrit ses erreurs sur la sortie standard, au milieu de la
    # liste des fichiers signés. La faire taire pour ne pas dérouler mille
    # lignes revenait à cacher aussi « keystore password was incorrect » : on
    # la retient donc, et on ne la montre qu'en cas d'échec.
    resultat = subprocess.run([
        "jarsigner", "-verbose:summary",
        "-keystore", cles["magasin"],
        "-storepass", cles["mot_de_passe"],
        "-keypass", cles["cle"],
        "-digestalg", "SHA-256", "-sigalg", "SHA256withRSA",
        "-signedjar", str(signe), str(aab), cles["alias"],
    ], capture_output=True, text=True)
    if resultat.returncode != 0:
        sys.exit("échec de la signature :\n"
                 + (resultat.stdout + resultat.stderr).strip())

    executer(["jarsigner", "-verify", signe], stdout=subprocess.DEVNULL)
    executer(["java", "-jar", jar, "validate", "--bundle", signe],
             stdout=subprocess.DEVNULL)
    return signe


def main():
    arguments = argparse.ArgumentParser(description=__doc__)
    arguments.add_argument("--version", help="numéro à graver ; celui de Cargo.toml sinon")
    arguments.add_argument("--abi", action="append", choices=list(ARCHITECTURES),
                           help="une architecture précise, répétable ; "
                                "les quatre par défaut")
    choix = arguments.parse_args()

    # Les quatre par défaut : le bundle les sépare, si bien qu'un téléphone ne
    # télécharge que la sienne. En couvrir moins n'allège rien et ferme des
    # appareils — les x86 sont ceux des Chromebooks et de l'émulateur.
    abis = choix.abi or list(ARCHITECTURES)
    version = numero_de_version(choix.version)
    charger_env()
    cles = signature()
    o = android.outils()
    jar = bundletool()

    print(f"Glyphfall {version} pour Google Play")
    for abi in abis:
        android.compiler(abi, o["clang"])

    travail = SORTIE / "bundle"
    shutil.rmtree(travail, ignore_errors=True)
    travail.mkdir(parents=True)

    archive = module(travail, abis, o, version)

    print("  bundle")
    brut = travail / "brut.aab"
    executer(["java", "-jar", jar, "build-bundle",
              f"--modules={archive}", f"--output={brut}", "--overwrite"],
             stdout=subprocess.DEVNULL)

    aab = signer(jar, brut, cles)

    poids = aab.stat().st_size / 1048576
    print(f"\n{aab}  ({poids:.0f} Mo)")
    print("  architectures :", ", ".join(abis))
    print("  à téléverser dans la Play Console, section « Versions »")


if __name__ == "__main__":
    main()
