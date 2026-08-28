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

from version import code_android, version as numero_de_version

RACINE = Path(__file__).resolve().parent.parent
PROJET = RACINE / "android"
SORTIE = RACINE / "target" / "android"

PAQUET = "fr.harlock.glyphfall"
NOM_LIB = "libglyphfall_core.so"

# La balise que les deux empaqueteurs completent avant de la donner a aapt2.
MANIFESTE_OUVRANT = '<manifest xmlns:android="http://schemas.android.com/apk/res/android">'

# Le niveau d'API visé par la bibliothèque native, et le plancher
# d'installation. À garder en accord avec android/app/build.gradle, qui les
# redit pour Gradle — `aapt2`, lui, ne sait pas lire un fichier Gradle.
API = 26
CIBLE_SDK = 35

# Les quatre architectures d'Android, et le triplet Rust de chacune.
ARCHITECTURES = {
    "arm64-v8a": ("aarch64-linux-android", "aarch64-linux-android"),
    "armeabi-v7a": ("armv7-linux-androideabi", "armv7a-linux-androideabi"),
    "x86_64": ("x86_64-linux-android", "x86_64-linux-android"),
    "x86": ("i686-linux-android", "i686-linux-android"),
}


def outil(nom, script=False):
    """Le nom d'un outil du SDK selon le système.

    Le SDK livre ses outils en deux exemplaires : un exécutable et un script
    d'enrobage. Sous Windows le premier prend `.exe` et le second `.bat` ;
    ailleurs, tous deux sont sans extension.
    """
    if sys.platform != "win32":
        return nom
    return nom + (".bat" if script else ".exe")


def hote():
    """Le nom que le NDK donne au système qui compile."""
    return {
        "win32": "windows-x86_64",
        "darwin": "darwin-x86_64",
    }.get(sys.platform, "linux-x86_64")


def sdk():
    for variable in ("ANDROID_HOME", "ANDROID_SDK_ROOT"):
        if chemin := os.environ.get(variable):
            return Path(chemin)
    for defaut in (
        Path.home() / "AppData/Local/Android/Sdk",
        Path.home() / "Android/Sdk",
        Path.home() / "Library/Android/sdk",
    ):
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
    # Le NDK vit d'ordinaire dans le SDK, mais une machine d'intégration
    # continue l'installe souvent à côté et l'annonce par une variable.
    ndk = None
    for variable in ("ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "ANDROID_NDK"):
        if chemin := os.environ.get(variable):
            ndk = Path(chemin)
            break
    if ndk is None or not ndk.exists():
        ndk = plus_recent(racine / "ndk")
    # Le NDK range ses compilateurs par système hôte.
    clang = ndk / "toolchains/llvm/prebuilt" / hote() / "bin"
    if not clang.exists():
        clang = plus_recent(ndk / "toolchains/llvm/prebuilt", lambda _: True) / "bin"

    return {
        "aapt2": build_tools / outil("aapt2"),
        "d8": build_tools / outil("d8", script=True),
        "zipalign": build_tools / outil("zipalign"),
        "apksigner": build_tools / outil("apksigner", script=True),
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


# Les fichiers où l'on peut ranger les mots de passe de signature plutôt que
# de les retaper. Tous deux sont ignorés par git.
FICHIERS_ENV = ("signature.env", ".env")


def charger_env():
    """Lit les mots de passe d'un fichier local, sans dépendance extérieure.

    `python-dotenv` ferait la même chose en une ligne, mais rien n'installe de
    bibliothèque Python dans le workflow : une dépendance de plus ici, et la
    construction Android échouerait sur l'intégration continue sans qu'on le
    voie avant la publication — c'est déjà arrivé avec Pillow.

    Ce que l'environnement porte déjà l'emporte : sur GitHub, les secrets du
    dépôt doivent primer, quoi qu'un fichier oublié raconte.
    """
    for nom in FICHIERS_ENV:
        chemin = RACINE / nom
        if not chemin.exists():
            continue
        for ligne in chemin.read_text(encoding="utf-8").splitlines():
            ligne = ligne.strip()
            if not ligne or ligne.startswith("#") or "=" not in ligne:
                continue
            cle, _, valeur = ligne.partition("=")
            os.environ.setdefault(cle.strip(), valeur.strip().strip("\"'"))


def cle_de_signature(outil_keytool):
    """La clé qui signe l'APK : la vôtre si vous en fournissez une.

    Sans elle, on retombe sur une clé de débogage — commode pour poser le jeu
    sur son propre téléphone, inutilisable pour une distribution : sur
    l'intégration continue elle est recréée à chaque construction, si bien que
    deux releases successives ne se ressemblent pas. Android refuse alors la
    mise à jour de l'une par l'autre, et le joueur doit désinstaller.
    """
    charger_env()
    magasin = os.environ.get("GLYPHFALL_KEYSTORE")
    mot_de_passe = os.environ.get("GLYPHFALL_KEYSTORE_PASSWORD")

    if magasin and mot_de_passe and Path(magasin).exists():
        return {
            "magasin": magasin,
            "magasin_pass": mot_de_passe,
            "cle_pass": os.environ.get("GLYPHFALL_KEY_PASSWORD") or mot_de_passe,
            "alias": os.environ.get("GLYPHFALL_KEY_ALIAS") or "glyphfall",
            "debogage": False,
        }

    return {
        "magasin": cle_de_debogage(outil_keytool),
        "magasin_pass": "android",
        "cle_pass": "android",
        "alias": "androiddebugkey",
        "debogage": True,
    }


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


def dex(travail, o):
    """Compile le Java du dépôt et celui qu'`aapt2` a engendré, puis le traduit.

    Le Java se limite à l'activité de miniquad et au `R` des ressources : rien
    ici ne dépend de la forme du paquet, si bien que l'APK et l'AAB partagent
    exactement le même `classes.dex`.
    """
    print("  java")
    source = PROJET / "app/src/main"
    (travail / "gen").mkdir(exist_ok=True)
    sources = list(source.rglob("*.java")) + list((travail / "gen").rglob("*.java"))
    classes = travail / "classes"
    classes.mkdir(exist_ok=True)

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

    return travail / "classes.dex"


def manifeste_complet(dossier, version):
    """Le manifeste tel qu'`aapt2` le veut, écrit dans le dossier de travail.

    Gradle tire le paquet, les versions et les niveaux d'API de `build.gradle`
    et refuse de les voir dans le manifeste ; `aapt2`, lui, ne lit que le
    manifeste, et prend un minSdk de 1 s'il n'y trouve rien — ce qui ferait
    tourner le jeu en mode compatibilité, à l'écran rétréci. On complète donc
    une copie, le manifeste du dépôt restant celui de Gradle.

    L'APK et l'AAB partagent ce manifeste : c'est le même jeu, et laisser
    diverger leurs versions serait le plus discret des défauts.
    """
    source = PROJET / "app/src/main/AndroidManifest.xml"
    entete = "\n".join([
        MANIFESTE_OUVRANT[:-1],
        f'    package="{PAQUET}"',
        f'    android:versionCode="{code_android(version)}"',
        f'    android:versionName="{version}">',
        f'    <uses-sdk android:minSdkVersion="{API}"'
        f' android:targetSdkVersion="{CIBLE_SDK}" />',
    ])
    destination = dossier / "AndroidManifest.xml"
    destination.write_text(
        source.read_text(encoding="utf-8").replace(MANIFESTE_OUVRANT, entete, 1),
        encoding="utf-8",
    )
    return destination


def assembler(abis, o, version):
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

    manifeste = manifeste_complet(travail, version)

    base = travail / "base.apk"
    executer([
        o["aapt2"], "link",
        "-I", o["android_jar"],
        "--manifest", manifeste,
        "--java", travail / "gen",
        "-o", base,
        travail / "res.zip",
    ])

    dex(travail, o)

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

    keytool = Path(shutil.which("keytool") or outil("keytool"))
    cles = cle_de_signature(keytool)
    if cles["debogage"]:
        print("  signature de débogage : cet APK ne se distribue pas")
    # `apksigner` dit pourquoi il refuse — « keystore password was incorrect »
    # par exemple —, mais sur sa sortie d'erreur, que le rapport d'échec
    # ordinaire ne reprend pas. On la retient donc pour la montrer.
    resultat = subprocess.run([str(part) for part in [
        o["apksigner"], "sign",
        "--ks", cles["magasin"],
        "--ks-pass", f"pass:{cles['magasin_pass']}",
        "--key-pass", f"pass:{cles['cle_pass']}",
        "--ks-key-alias", cles["alias"],
        aligne,
    ]], capture_output=True, text=True)
    if resultat.returncode != 0:
        premiere = (resultat.stderr + resultat.stdout).strip().splitlines()
        sys.exit("échec de la signature :\n"
                 + "\n".join(premiere[:3]))
    return aligne


def main():
    arguments = argparse.ArgumentParser(description=__doc__)
    arguments.add_argument("--version", help="numéro à graver ; celui de Cargo.toml sinon")
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

    apk = assembler(abis, o, numero_de_version(choix.version))
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
