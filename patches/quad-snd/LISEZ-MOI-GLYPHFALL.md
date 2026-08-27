# Copie corrigée de quad-snd 0.2.8

Ce dossier n'est pas de nous. C'est `quad-snd` 0.2.8 tel que publié par Fedor
Logachev, sous **licence MIT ou Apache-2.0 au choix**, avec une seule
modification — dans `src/alsa_snd.rs`.

Le reste du jeu est sous GPL-3.0 ; cette bibliothèque garde sa licence
d'origine, que ces deux-là autorisent à combiner.

## Ce qui a changé, et pourquoi

Sur Linux, le jeu se lançait muet en imprimant sans fin :

    thread '<unnamed>' panicked at src/alsa_snd.rs:62:
    Can't set harware parameters.
    Audio thread died
    Audio thread died
    ...

Deux défauts, l'un derrière l'autre.

**La mise en place ALSA imposait une taille de tampon exacte, trop tôt.**
L'ordre d'origine était : accès, format, *taille de tampon*, voies, fréquence.
Chaque réglage passait isolément, mais leur écriture groupée était refusée :
une fois la stéréo et le 44,1 kHz arrêtés, plus aucune configuration ne tenait
dans un tampon de 4096 images exactement. La taille de tampon vient désormais
en dernier, et se négocie — `set_buffer_size_near` laisse ALSA choisir la
valeur possible la plus proche.

**L'échec était une panique.** Une machine sans son n'est pas une erreur de
programmation. La panique tuait le fil audio, refermait le canal, et chaque son
demandé imprimait alors « Audio thread died » — une ligne par bruitage, par
voix et par morceau, jusqu'à noyer la console. La mise en place rend maintenant
`None`, et le fil bascule sur `silence()` : il continue de vider la file des
messages sans rien jouer. Le jeu tourne muet, et se tait aussi dans la console.

## Pourquoi ce dossier est versionné

Un `vendor/` — le miroir de toutes les dépendances que produit `cargo vendor` —
n'a rien à faire dans un dépôt : il pèse des dizaines de mégaoctets et
n'apporte rien qu'un `cargo fetch` ne retrouve. Ceci est autre chose : **un
seul correctif que nous portons**, sans lequel la compilation ne se fait pas.
D'où le nom du dossier, qui dit ce qu'il contient.

Les exemples et les fichiers annexes de la publication d'origine ont été
retirés : il ne reste que ce que la compilation lit, plus le README de l'auteur.

## À supprimer le jour où le correctif remonte en amont

`Cargo.toml` branche cette copie par `[patch.crates-io]`. Si une version
ultérieure de `quad-snd` corrige ces deux points, retirez la section et ce
dossier.
