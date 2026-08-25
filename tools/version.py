# -*- coding: utf-8 -*-
"""D'où vient le numéro de version, et comment il se décline.

Il était écrit en dur dans trois fichiers et deux formats. Publier `0.1.2`
donnait donc un paquet `0.1.0` : rien ne reliait l'étiquette aux fichiers
qu'elle produisait.

L'ordre est désormais : ce que la ligne de commande impose, puis ce que
l'environnement annonce — l'intégration continue y met l'étiquette de la
release —, puis `Cargo.toml`, qui reste la référence quand on construit chez
soi.
"""
import os
import re
import sys
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent


def version(explicite=None):
    """Le numéro à graver dans les paquets, sans le `v` d'usage des étiquettes."""
    brut = explicite or os.environ.get("GLYPHFALL_VERSION") or depuis_cargo()
    return brut.strip().removeprefix("v")


def depuis_cargo():
    manifeste = (RACINE / "Cargo.toml").read_text(encoding="utf-8")
    trouve = re.search(r'^version = "(.*?)"', manifeste, re.M)
    if not trouve:
        sys.exit("version absente de Cargo.toml")
    return trouve[1]


def code_android(numero):
    """Le numéro de version tel qu'Android l'exige : un entier qui ne recule pas.

    `0.1.2` devient 102. Chaque partie tient sur deux chiffres, ce qui suffit
    largement et garde l'ordre : une version plus récente donne toujours un
    entier plus grand. Au-delà de 99, il faudrait élargir le pas.
    """
    parties = [int(part) for part in re.findall(r"\d+", numero)[:3]]
    parties += [0] * (3 - len(parties))
    majeure, mineure, correctif = parties

    if mineure > 99 or correctif > 99:
        sys.exit(f"version {numero} : au-delà de 99, le code Android reculerait")

    return majeure * 10_000 + mineure * 100 + correctif
