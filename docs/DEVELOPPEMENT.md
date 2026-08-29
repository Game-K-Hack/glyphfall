# Guide du développeur

Comment Glyphfall est bâti, comment le compiler pour chaque plateforme, et quoi
toucher pour l'étendre. Pour jouer, voir le [guide du joueur](JOUER.md).

- [Prendre en main](#prendre-en-main)
- [Le principe : une toile de 384 × 216](#le-principe--une-toile-de-384--216)
- [Organisation du code](#organisation-du-code)
- [Ajouter une langue](#ajouter-une-langue)
- [Ajouter une police](#ajouter-une-police)
- [Ajouter une musique](#ajouter-une-musique)
- [Ajouter des voix](#ajouter-des-voix)
- [Ce que le moteur fait de son côté](#ce-que-le-moteur-fait-de-son-côté)
- [Ce que le jeu vérifie au démarrage](#ce-que-le-jeu-vérifie-au-démarrage)
- [Compiler pour chaque plateforme](#compiler-pour-chaque-plateforme)
- [Publier](#publier)
- [Raccourcis de développement](#raccourcis-de-développement)
- [Pièges connus](#pièges-connus)

## Prendre en main

```sh
cargo run --release      # lancer le jeu
cargo test --release     # 129 tests, dont la validation du catalogue
```

Aucune dépendance système sur Windows et macOS. Sur Linux, il faut
`libasound2-dev` pour compiler.

Les recettes d'empaquetage vivent dans **`tools/`**, en Python sans dépendance
extérieure, et tournent aussi bien sur une machine que sur un coureur
d'intégration : le workflow ne fait que les appeler. Une chaîne de compilation
qui n'existerait que dans l'intégration continue finirait par diverger de celle
qu'on utilise pour de vrai.

## Le principe : une toile de 384 × 216

Tout est dessiné sur une toile virtuelle, agrandie d'un facteur **entier** au
plus proche voisin. C'est ce qui donne de vrais pixels carrés. Les écrans
raisonnent donc en pixels virtuels et n'appellent jamais `screen_width()`.

L'écran suit la machine : **couché** sur un bureau (384 × 216), **debout** sur
un téléphone (216 × 384). C'est une constante de compilation :

```rust
pub const PORTRAIT: bool = cfg!(target_os = "android") || cfg!(feature = "portrait");
```

Les écrans posent leurs coordonnées en paires — `canvas::pick(debout, couché)`.
Le compilateur efface la branche inutile : les deux mises en page ne coûtent
rien à l'exécution.

Pour regarder la mise en page téléphone depuis un bureau :

```sh
cargo run --features portrait
```

Cinq fois 384 × 216 donne exactement 1920 × 1080, et cinq fois 216 × 384 donne
1080 × 1920 : c'est ce qui permet aux captures et aux bandes-annonces d'être au
format des magasins sans aucune interpolation.

## Organisation du code

| Fichier | Rôle |
|---|---|
| `src/app.rs` | État global et pile de navigation |
| `src/session.rs` | Une manche : règles, tirage des tuiles, score, bilan |
| `src/progress.rs` | Étoiles gagnées, maîtrise par signe, déverrouillage |
| `src/settings.rs` | Réglages : volumes, objectif quotidien, tracés variés |
| `src/daily.rs` | Temps d'apprentissage du jour |
| `src/storage.rs` | Sauvegardes : fichiers, ou stockage du navigateur |
| `src/audio.rs` | Bruitages, synthétisés au démarrage |
| `src/music.rs` | Playlists, décodage des morceaux |
| `src/voices.rs` | Enregistrements de prononciation |
| `src/compose.rs` | Le générateur de la musique « Claude » |
| `src/trace.rs` | Journal des apparitions, pour la mise au point |
| `src/data/` | Lecture et validation des fichiers de langue |
| `src/gfx/` | Toile, palette, polices, briques d'interface |
| `src/screens/` | Un fichier par écran |
| `src/screens/keyboard.rs` | Le clavier dessiné, pour jouer sans clavier |

Les deux plus gros fichiers sont `session.rs` (la manche) et `gfx/ui.rs` (les
briques d'interface). C'est là que se passe l'essentiel.

## Ajouter une langue

Créez un dossier dans `assets/languages/`, puis recompilez. **Rien à déclarer
dans le code** : le dossier est découvert tout seul.

```
assets/languages/el-grec/
├── language.toml
└── levels/
    ├── 01-voyelles.toml
    └── 02-consonnes.toml
```

`language.toml` :

```toml
id = "el-grec"                     # identifiant stable, utilisé par la sauvegarde
name = "Grec"                      # nom affiché, en français
native_name = "Ελληνικά"           # nom dans l'écriture elle-même
description = "L'alphabet grec, 24 lettres."
fonts = [                          # fichiers de assets/fonts/, au moins deux
  "NotoSansGreek-Regular.ttf",     # le tracé de référence
  "GreekHandwriting-Regular.ttf",  # les autres, tirés au sort en jeu
]
```

Un fichier par étape dans `levels/` :

```toml
id = "el-01"                       # unique dans TOUT le catalogue
title = "Les voyelles"
subtitle = "Sept lettres pour commencer"
order = 1                          # position sur le chemin
requires = []                      # étapes à finir avant celle-ci
mode = "tile_fall"

[rules]
lives = 3
duration = 90                      # secondes ; 0 = sans limite
columns = 4
spawn_interval = 1.4               # secondes entre deux signes
speed = { start = 55.0, ramp = 1.5, max = 170.0 }
review_ratio = 0.25                # part des signes puisée dans `requires`

[stars]                            # précision minimale pour chaque étoile
one = 0.50
two = 0.75
three = 0.90

[[glyphs]]
char = "α"
answers = ["a"]                    # toutes les lectures acceptées
mnemonics = [                      # au moins un, obligatoire
  "Le premier signe de l'alphabet, l'ancetre de notre A.",
  "Un a minuscule dont la boucle s'est ouverte.",
]
```

La fiche du signe montre tous les moyens mnémotechniques, mais n'affiche que
**sept lignes** en tout, bande des tracés oblige — le signe le plus bavard du
catalogue les occupe déjà toutes.

### Comment découper un chemin

Un niveau qui présente quatorze signes d'un coup n'en apprend aucun. Trois
règles :

- **Trois à cinq signes nouveaux par étape**, pas plus. Une étape doit se
  gagner du premier coup ou presque.
- **Une révision toutes les deux ou trois étapes**, qui ne présente aucun signe
  nouveau et rebrasse ce qui précède.
- **`review_ratio` monte avec le chemin.** Il puise dans **toute** la chaîne de
  `requires`, pas seulement dans l'étape précédente : à la quinzième étape, un
  tirage sur deux revient sur ce qui a été appris depuis le début.

Les étapes de découverte sont volontairement lentes et courtes — `duration`
autour de 50 s, `speed.start` vers 42 — et se durcissent au fil du chemin. Un
signe qu'on découvre demande le temps de le reconnaître ; un signe qu'on révise
doit venir tout seul.

## Ajouter une police

Déposez le `.ttf` dans `assets/fonts/` et nommez-le dans la liste `fonts` de
`language.toml`. **Une écriture en déclare au moins deux** — un test le
vérifie — parce que les tuiles y puisent au hasard.

Prenez des tracés **franchement** différents. Une sans empattement et une avec
ne suffit pas : sur du hangeul comme sur du kana, sans et serif se ressemblent
trop à 24 pixels. Ce qui marche, ce sont des familles de mains différentes.
Chaque écriture en compte sept :

| Rôle | Coréen | Japonais |
|---|---|---|
| Imprimé, sans empattement | Noto Sans KR | Noto Sans JP |
| Imprimé, à empattements | Nanum Myeongjo | Shippori Mincho |
| Manuscrit, stylo | Nanum Pen Script | Klee One |
| Manuscrit, crayon | Gaegu | — |
| Manuscrit, pinceau | Nanum Brush Script | Yuji Syuku |
| Manuscrit, feutre | — | Yusei Magic |
| Affiche, ronde | Jua | Zen Maru Gothic |
| Affiche, grasse | Do Hyeon | RocknRoll One |

La première de la liste est le **tracé de référence** : c'est elle qui écrit le
briefing, les aides et le grand signe de la fiche.

Une police CJK complète pèse plusieurs mégaoctets et alourdit d'autant le
binaire. Réduisez-la aux signes du catalogue avant de la déposer — `fonttools`
fait passer les quatre polices CJK de 50 Mo à moins d'un :

```sh
pyftsubset NotoSerifJP-Regular.ttf --text-file=signes.txt --output-file=...
```

## Ajouter une musique

Déposez un `.mp3`, `.ogg` ou `.wav` dans l'un des deux dossiers et recompilez.
Rien à déclarer : les fichiers sont repérés tout seuls et enchaînés dans un
ordre aléatoire.

| Dossier | Quand |
|---|---|
| `assets/music/menu/` | Partout sauf en partie |
| `assets/music/game/` | Pendant les manches |

Le moteur audio ne sait pas lire le MP3 : le jeu décode lui-même les morceaux
puis les lui confie en WAV brut. C'est ce décodage qui donne aussi leur durée
exacte, dont la playlist a besoin pour savoir quand enchaîner.

Une seule piste est décodée à la fois, au moment où elle démarre : un morceau
de cinq minutes prend environ trois dixièmes de seconde à décoder et occupe une
cinquantaine de mégaoctets pendant qu'il joue. Des morceaux courts sont plus
économes, et leur poids s'ajoute directement à celui de l'exécutable.

Le dossier peut rester vide : le jeu se lance alors sans musique.

## Ajouter des voix

Les enregistrements vivent dans `assets/voices/<langue>/`, et
`assets/languages/<langue>/voices.toml` dit quel fichier correspond à quel
signe. Deux signes peuvent pointer sur le même fichier.

Les noms de fichiers sont ceux du fournisseur, gardés tels quels pour pouvoir
recouper avec la source d'origine.

Les kanji n'en ont pas, et c'est délibéré : leur lecture dépend du mot qui les
entoure. Le détail est dans l'en-tête de `src/voices.rs`.

## Ce que le moteur fait de son côté

Le découpage des fichiers ne fait que la moitié du travail. Pendant la manche,
le tirage des tuiles n'est pas uniforme :

- **Les signes neufs passent d'abord**, un par un, dans l'ordre du fichier. Un
  tirage purement aléatoire pourrait montrer le même trois fois de suite et en
  oublier un autre jusqu'à la fin.
- **Un signe raté revient** trois tuiles plus tard. C'est le seul moment où la
  correction porte encore.
- **Le tirage favorise ce qui est mal su.** Chaque signe porte une note de
  maîtrise, gardée d'une partie à l'autre : une réussite la monte d'un point,
  une erreur la descend de deux. Cette asymétrie est volontaire — sans elle, un
  signe raté une fois sur trois finirait par passer pour acquis.

Ces notes suivent le signe, pas son tracé : changer de police ne remet pas la
maîtrise à zéro. Elles sont rangées **par écriture** : un signe n'existe pas en
dehors de la sienne.

Le briefing marque en orange les signes dont la note est négative — il dit donc
quoi travailler, au lieu d'afficher une liste uniforme où les faiblesses se
noient.

## Ce que le jeu vérifie au démarrage

Un contenu incohérent affiche un écran d'erreur explicite plutôt que de casser
silencieusement le chemin. Sont refusés : deux identifiants identiques, un
`requires` qui ne résout pas, un cycle de prérequis, des seuils d'étoiles
décroissants, un `review_ratio` sans prérequis à réviser, un niveau sans signe,
une clé mal orthographiée.

`cargo test` va plus loin : chaque signe du catalogue doit être dessinable par
la police de sa langue, la police pixel doit couvrir tous les textes français —
un accent manquant afficherait « Cor en » sans rien casser —, et chaque fichier
cité par un `voices.toml` doit exister.

## Compiler pour chaque plateforme

### Linux, depuis n'importe quel système

Zig fournit la chaîne croisée : ni machine virtuelle ni conteneur.

```sh
pip install ziglang && cargo install cargo-zigbuild
python tools/linux.py      # binaire et archive dans target/linux/
python tools/deb.py        # paquet Debian dans target/deb/
```

Le jeu demande une glibc 2.31 ou plus récente — Ubuntu 20.04 et au-delà — et,
sur la machine du joueur, `libasound2`, `libX11`, `libXi`, `libGL` et
`libxkbcommon`. Seule la première est inscrite dans le paquet : miniquad ouvre
les autres à l'exécution, si bien que leur absence ne se voit qu'au lancement.

Un `.deb` n'étant qu'une archive `ar` de trois membres, il est fabriqué à la
main : aucun outil Debian n'est nécessaire, et il se construit donc depuis
Windows.

### Windows

```sh
cargo build --release
python tools/setup.py      # programme d'installation
```

### Android

SDK, NDK et Java 17 requis. **Ni Gradle ni Docker.**

```sh
python tools/android.py              # arm64, le plus courant
python tools/android.py --toutes     # les quatre architectures
adb install -r target/android/glyphfall.apk
```

Les niveaux d'API vivent dans **`android/sdk.properties`**, que `tools/` et
Gradle lisent tous deux — les écrire deux fois, c'était les laisser diverger.
La construction prévient quand une plateforme plus récente est installée, sans
relever `targetSdk` toute seule : chaque niveau change des comportements.

`android/` reste un projet Gradle ouvrable dans Android Studio. Les deux
chemins partagent le manifeste, le script y ajoutant à la volée ce que Gradle
refuse d'y voir — le paquet, les versions, les niveaux d'API.

**Pour Google Play**, il faut un bundle signé, pas un APK :

```sh
python tools/bundle.py     # target/android/glyphfall.aab
```

La clé vient de l'environnement, jamais du dépôt — `GLYPHFALL_KEYSTORE` et
`GLYPHFALL_KEYSTORE_PASSWORD`, ou un `signature.env` à la racine, ignoré par
git. Sans clé, `android.py` retombe sur une clé de débogage **et le dit** : un
APK ainsi signé s'installe par sideload mais ne se publie pas, et sa signature
change à chaque construction.

### Navigateur

```sh
python tools/web.py
python -m http.server --directory target/web 8080
```

Ouvrir `index.html` depuis le disque ne marche pas : le navigateur refuse de
charger un `.wasm` en `file://`. Il faut passer par HTTP, même en local.

**Ce que le binaire embarque, et ce qu'il ne porte plus.** Les polices et les
leçons voyagent dans le `.wasm` — trois mégaoctets, disponibles dès la première
image. Les musiques et les voix, soixante-huit de plus, sont recopiées à côté
de la page et récupérées à la demande : les embarquer ferait attendre, avant le
premier écran, un contenu qui ne sera peut-être jamais joué. Le `.wasm` tombe
ainsi de 72 Mo à 5.

C'est la seule différence entre le navigateur et les autres plateformes.
`data::asset_bytes` cache cet écart derrière une seule fonction, et les chemins
sont identiques des deux côtés — d'où le `assets/` recopié tel quel.

Le site publié comprend aussi une page d'accueil (`web/index.html`), le jeu
(`web/jouer.html`) et les deux pages légales. « Quitter » y ramène à l'accueil,
par une fonction que la page fournit au moteur : dans un onglet il n'y a pas de
fenêtre à fermer, et sortir de la boucle laisserait une toile figée.

## Publier

On ne pose plus d'étiquette à la main. **release-please** lit les messages de
commit de `master`, tient à jour une pull request de publication — version
relevée dans `Cargo.toml`, journal des changements écrit — et c'est en la
fusionnant qu'on publie.

| message | effet sur `0.3.0` |
|---|---|
| `fix: ...` | `0.3.1` |
| `feat: ...` | `0.4.0` |
| `feat!: ...` | `1.0.0` |

`docs:`, `build:`, `refactor:` ne font pas bouger le numéro mais figurent au
journal.

`.github/workflows/build.yml` fabrique tout : Windows et son programme
d'installation, Linux x86_64 et ARM avec leurs paquets Debian, macOS Intel et
Apple Silicon, l'APK, le bundle Play et la version navigateur — publiée sur
GitHub Pages à chaque release.

Il est **appelé** par le workflow de publication plutôt que déclenché par lui :
ce qu'un workflow fait avec le jeton par défaut n'en déclenche pas un autre. Une
release créée par release-please n'émettrait aucun événement, et rien ne se
construirait.

Le bundle Play n'est pas attaché à la release : il part dans l'artefact
`play-bundle`. Un AAB ne s'installe pas, et l'attacher ajouterait deux cent
soixante-dix mégaoctets qu'aucun joueur ne peut ouvrir.

## Raccourcis de développement

```sh
GLYPHFALL_START=languages               cargo run
GLYPHFALL_START=options                 cargo run
GLYPHFALL_START=path:ja-hiragana        cargo run
GLYPHFALL_START=briefing:ko/ko-01       cargo run
GLYPHFALL_START=briefing:ko/ko-04/ultra cargo run   # sur un mode donné
GLYPHFALL_START=play:ko/ko-03           cargo run
GLYPHFALL_START=play:ko/ko-03/endless   cargo run   # dans un mode donné
GLYPHFALL_START=sign:ko/ko-01           cargo run   # la fiche du premier signe
GLYPHFALL_START=fonts                   cargo run   # la question des tracés

# Rejouer une manche à l'identique, et journaliser chaque apparition.
GLYPHFALL_SEED=7 GLYPHFALL_TRACE=trace.txt cargo run

# Capturer une image après N frames puis quitter, pour vérifier un écran.
GLYPHFALL_SCREENSHOT=ecran.png GLYPHFALL_SCREENSHOT_AFTER=120 cargo run

# Écrire une image par frame, horodatée : ce dont sont faites les
# bandes-annonces.
GLYPHFALL_FILM=images/ GLYPHFALL_FILM_FRAMES=900 cargo run --release

# Régénérer la musique d'ambiance après avoir retouché src/compose.rs.
GLYPHFALL_COMPOSE=assets/music/menu/Claude.wav cargo run --release
```

Aucune de ces variables ne coûte quoi que ce soit quand elle n'est pas posée.

## Pièges connus

**Tous les textes sont écrits avec la police pixel**, qui ne connaît que le
latin : titres, sous-titres, noms, descriptions et moyens mnémotechniques. N'y
mettez pas de caractères de l'écriture enseignée, ils s'afficheraient en tofu —
un test le vérifie. Nommez plutôt le signe (« le giyeok », « le ha »). Seuls les
`char` passent par la police de la langue.

**`speed` s'exprime en pixels virtuels par seconde**, sur une toile de
384 × 216. Un signe parcourt 200 pixels avant d'atteindre la ligne : à 55, il
laisse un peu moins de quatre secondes pour répondre.

**`patches/quad-snd` est une copie corrigée** de la bibliothèque audio de
macroquad, montée par `[patch.crates-io]`. Sa mise en place ALSA échouait sur
des machines Linux saines ; le motif est expliqué dans son
[LISEZ-MOI](../patches/quad-snd/LISEZ-MOI-GLYPHFALL.md). Ne la remplacez pas par
la version amont sans vérifier le son sur Linux.

**`.cargo/config.toml` porte deux réglages non négociables.** Le navigateur a
besoin de `--allow-undefined`, les fonctions de `gl.js` n'existant qu'à
l'exécution. Android a besoin de `-Wl,-z,max-page-size=16384` : Google Play
refuse un bundle dont une bibliothèque n'est pas alignée sur des pages de 16 Ko,
et `lld` ne le fait de lui-même ni sur les cibles 32 bits ni avec un NDK ancien.

**Les scripts de `promo/` ne sont pas versionnés** — seules les images le sont.
Ils fabriquent bannières, captures et bandes-annonces à partir des variables
`GLYPHFALL_FILM`, `GLYPHFALL_SEED` et `GLYPHFALL_TRACE`.
