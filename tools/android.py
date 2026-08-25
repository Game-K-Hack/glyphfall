# -*- coding: utf-8 -*-
"""Construit l'APK de Glyphfall, sans Gradle.

Le projet `android/` reste ouvrable dans Android Studio ; ce script sert à
fabriquer un APK signé de test en une commande, avec les seuls outils du SDK.
Il compile la bibliothèque native pour chaque architecture, puis assemble et
signe le paquet.

    python tools/android.py            # arm64 seulement, le plus courant
    python tools/android.py --toutes   # les quatre architectures

Un APK signé avec une clé de débogage s'installe par sideload mais ne se publie
pas : le magasin exige une clé que vous seul détenez. Le projet Gradle est là
pour ça.
"""
import argparse
import os
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent
PROJET = RACINE / "android"
SORTIE = RACINE / "target" / "android"

PAQUET = "fr.harlock.glyphfall"
NOM_LIB = "libglyphfall_core.so"

# Le niveau d'API visé par la bibliothèque native, et le plancher
# d'installation. À garder en accord avec android/app/build.gradle, qui les
# redit pour Gradle — `aapt2`, lui, ne sait pas lire un fichier Gradle.
API = 26
CIBLE_SDK = 35
VERSION_CODE = 1
VERSION_NOM = "0.1.0"

# Les quatre architectures d'Android, et le triplet Rust de chacune.
ARCHITECTURES = {
    "arm64-v8a": ("aarch64-linux-android", "aarch64-linux-android"),
    "armeabi-v7a": ("armv7-linux-androideabi", "armv7a-linux-androideabi"),
    "x86_64": ("x86_64-linux-android", "x86_64-linux-android"),
    "x86": ("i686-linux-android", "i686-linux-android"),
}


def sdk():
    for variable in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        if chemin := os.environ.get(variable):
            return Path(chemin)
    defaut = Path.home() / "AppData/Local/Android/Sdk"
    if defaut.exists():
        return defaut
    sys.exit("SDK Android introuvable : posez ANDROID_HOME.")


def plus_recent(dossier, filtre=lambda _: True):
    """Le sous-dossier de version la plus élevée, façon `30.0.3` < `37.0.0`."""
    def cle(chemin):
        return [int(part) if part.isdigit() else 0 for part in chemin.name.split(".")[:3]]

    candidats = [c for c in dossier.iterdir() if c.is_dir() and filtre(c)]
    if not candidats:
        sys.exit(f"rien d'utilisable dans {dossier}")
    return max(candidats, key=cle)


def outils():
    """Les chemins dont l'assemblage a besoin."""
    racine = sdk()
    build_tools = plus_recent(racine / "build-tools")
    plateforme = plus_recent(
        racine / "platforms", lambda c: (c / "android.jar").exists()
    )
    ndk = plus_recent(racine / "ndk")
    clang = ndk / "toolchains/llvm/prebuilt/windows-x86_64/bin"
    if not clang.exists():
        # Le NDK range ses compilateurs par système hôte.
        clang = plus_recent(ndk / "toolchains/llvm/prebuilt", lambda _: True)
        clang = clang / "bin"

    return {
        "aapt2": build_tools / "aapt2.exe",
        "d8": build_tools / "d8.bat",
        "zipalign": build_tools / "zipalign.exe",
        "apksigner": build_tools / "apksigner.bat",
        "android_jar": plateforme / "android.jar",
        "clang": clang,
    }


def executer(commande, **kwargs):
    resultat = subprocess.run([str(part) for part in commande], **kwargs)
    if resultat.returncode != 0:
        sys.exit(f"échec : {' '.join(str(p) for p in commande[:3])}…")
    return resultat


def compiler(abi, clang):
    """Compile la bibliothèque native pour une architecture."""
    triplet, prefixe = ARCHITECTURES[abi]
    lieur = clang / f"{prefixe}{API}-clang.cmd"
    if not lieur.exists():
        lieur = clang / f"{prefixe}{API}-clang"

    environnement = dict(os.environ)
    # Cargo cherche le lieur dans une variable nommée d'après la cible.
    variable = "CARGO_TARGET_" + triplet.upper().replace("-", "_") + "_LINKER"
    environnement[variable] = str(lieur)

    print(f"  compilation {abi} ({triplet})")
    executer(
        ["cargo", "build", "--release", "--target", triplet, "--lib"],
        cwd=RACINE,
        env=environnement,
    )

    produite = RACINE / "target" / triplet / "release" / NOM_LIB
    destination = PROJET / "app/src/main/jniLibs" / abi
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copy2(produite, destination / NOM_LIB)
    return destination / NOM_LIB


def cle_de_debogage(outil_keytool):
    """La clé de débogage, créée au premier appel."""
    magasin = SORTIE / "debug.keystore"
    if magasin.exists():
        return magasin

    SORTIE.mkdir(parents=True, exist_ok=True)
    print("  création d'une clé de débogage")
    executer([
        outil_keytool, "-genkeypair", "-v",
        "-keystore", magasin, "-storepass", "android", "-keypass", "android",
        "-alias", "androiddebugkey", "-keyalg", "RSA", "-keysize", "2048",
        "-validity", "10000", "-dname", "CN=Glyphfall Debug,O=Glyphfall,C=FR",
    ], stdout=subprocess.DEVNULL)
    return magasin


def assembler(abis, o):
    """Compile les ressources et le Java, puis fabrique l'APK signé."""
    travail = SORTIE / "assemblage"
    if travail.exists():
        shutil.rmtree(travail)
    (travail / "res").mkdir(parents=True)

    source = PROJET / "app/src/main"

    print("  ressources")
    executer([
        o["aapt2"], "compile", "--dir", source / "res",
        "-o", travail / "res.zip",
    ])

    # Gradle tire le paquet, les versions et les niveaux d'API de
    # `build.gradle` et refuse de les voir dans le manifeste ; `aapt2`, lui, ne
    # lit que le manifeste, et prend un minSdk de 1 s'il n'y trouve rien — ce
    # qui ferait tourner le jeu en mode compatibilité, à l'écran rétréci. On
    # complète donc une copie, le manifeste du dépôt restant celui de Gradle.
    manifeste = travail / "AndroidManifest.xml"
    contenu = (source / "AndroidManifest.xml").read_text(encoding="utf-8")
    entete = (
        '<manifest xmlns:android="http://schemas.android.com/apk/res/android"\n'
        f'    package="{PAQUET}"\n'
        f'    android:versionCode="{VERSION_CODE}"\n'
        f'    android:versionName="{VERSION_NOM}">\n'
        f'    <uses-sdk android:minSdkVersion="{API}"'
        f' android:targetSdkVersion="{CIBLE_SDK}" />'
    )
    manifeste.write_text(
        contenu.replace(
            '<manifest xmlns:android="http://schemas.android.com/apk/res/android">',
            entete,
            1,
        ),
        encoding="utf-8",
    )

    base = travail / "base.apk"
    executer([
        o["aapt2"], "link",
        "-I", o["android_jar"],
        "--manifest", manifeste,
        "--java", travail / "gen",
        "-o", base,
        travail / "res.zip",
    ])

    print("  java")
    (travail / "gen").mkdir(exist_ok=True)
    sources = list(source.rglob("*.java")) + list((travail / "gen").rglob("*.java"))
    classes = travail / "classes"
    classes.mkdir()
    # Pas de `-bootclasspath` : depuis Java 9 il est refusé au-delà de la
    # cible 8, et `android.jar` en simple classpath suffit — d8 se charge
    # ensuite de traduire vers le format d'Android.
    executer([
        "javac", "-source", "17", "-target", "17", "-nowarn",
        "-classpath", o["android_jar"],
        "-d", classes, *sources,
    ])

    executer([
        o["d8"], "--lib", o["android_jar"], "--output", travail,
        *classes.rglob("*.class"),
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    print("  paquet")
    brut = travail / "brut.apk"
    shutil.copy2(base, brut)
    with zipfile.ZipFile(brut, "a", zipfile.ZIP_DEFLATED) as paquet:
        paquet.write(travail / "classes.dex", "classes.dex")
        for abi in abis:
            # Les bibliothèques natives sont stockées sans compression : Android
            # les charge alors directement, sans les recopier ailleurs.
            paquet.write(
                PROJET / "app/src/main/jniLibs" / abi / NOM_LIB,
                f"lib/{abi}/{NOM_LIB}",
                compress_type=zipfile.ZIP_STORED,
            )

    aligne = SORTIE / "glyphfall.apk"
    aligne.unlink(missing_ok=True)
    executer([o["zipalign"], "-p", "-f", "4", brut, aligne])

    keytool = Path(shutil.which("keytool") or "keytool")
    executer([
        o["apksigner"], "sign",
        "--ks", cle_de_debogage(keytool),
        "--ks-pass", "pass:android", "--key-pass", "pass:android",
        aligne,
    ])
    return aligne


def main():
    arguments = argparse.ArgumentParser(description=__doc__)
    arguments.add_argument("--toutes", action="store_true",
                           help="compiler les quatre architectures")
    arguments.add_argument("--abi", action="append", choices=list(ARCHITECTURES),
                           help="une architecture précise, répétable "
                                "(x86_64 pour l'émulateur)")
    choix = arguments.parse_args()

    abis = choix.abi or (list(ARCHITECTURES) if choix.toutes else ["arm64-v8a"])
    o = outils()

    print("Glyphfall pour Android")
    for abi in abis:
        compiler(abi, o["clang"])

    apk = assembler(abis, o)
    poids = apk.stat().st_size / 1048576
    print(f"\n{apk}  ({poids:.0f} Mo)")
    print("  architectures :", ", ".join(abis))

    # Un APK sans arm64 s'installe sur l'émulateur et sur rien d'autre : tous
    # les téléphones vendus depuis des années sont en arm64. L'oubli ne se voit
    # qu'à l'installation, par un message qui n'explique rien.
    if "arm64-v8a" not in abis:
        print("  ATTENTION : sans arm64-v8a, aucun téléphone n'acceptera cet APK.")

    print("  installation :  adb install -r", apk)


if __name__ == "__main__":
    main()
