# AlphaTiles

Un jeu pour apprendre les alphabets qui ne s'écrivent pas en lettres latines :
hangeul, hiragana, katakana, kanji. Les signes tombent, vous tapez leur lecture
avant qu'ils ne franchissent la ligne rouge. Interface entièrement en 8-bit.

Écrit en Rust avec [macroquad](https://macroquad.rs), donc le même code tourne
sur Windows, macOS, Linux et dans un navigateur.

## Le parcours

1. **Choix de l'alphabet** — chaque écriture s'annonce dans ses propres signes.
2. **Chemin d'apprentissage** — les étapes s'enchaînent, une étoile suffit à
   ouvrir la suivante. Chaque écriture en compte une quinzaine : trois à cinq
   signes nouveaux par étape, et une révision toutes les deux ou trois.
3. **Briefing** — tous les signes de l'étape avec leur lecture, les règles et
   les seuils à viser. Survolez un signe pour son aide mnémotechnique.
4. **La manche** — trois vies et un chronomètre.
5. **Résultats** — de zéro à trois étoiles, et surtout la liste des signes
   ratés avec la lecture qu'il fallait taper.

La progression est enregistrée automatiquement, tout comme les volumes réglés
depuis l'écran d'options.

## Commandes

| Touche | Effet |
|---|---|
| Lettres et chiffres | Composer la lecture |
| `Entrée` / `Espace` | Valider la saisie |
| `Retour arrière` | Corriger |
| `↑` `↓` | Naviguer dans les menus |
| `←` `→` | Régler un volume, dans les options |
| `Échap` | Revenir en arrière |

La souris fonctionne partout où le clavier fonctionne.

## Lancer et construire

```sh
cargo run --release
```

Pour le navigateur :

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/alphatiles.wasm web/
```

Puis servez le dossier `web/` par HTTP — ouvrir `index.html` depuis le disque
ne marche pas, le navigateur refuse de charger un `.wasm` en `file://` :

```sh
python -m http.server --directory web 8080
```

Tout le contenu (langues, polices) est embarqué dans le binaire à la
compilation : il n'y a rien à distribuer à côté de l'exécutable. En
contrepartie le `.wasm` pèse environ 17 Mo, dont l'essentiel est constitué des
deux polices CJK. Servez-le avec la compression `gzip` activée, cela le ramène
à quelques mégaoctets.

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
font = "NotoSansGreek-Regular.ttf" # fichier de assets/fonts/, facultatif
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
hint = "alpha — l'ancêtre du A"    # facultatif, affiché au briefing
```

### Comment découper un chemin

Un niveau qui présente quatorze signes d'un coup n'en apprend aucun. Le
découpage suit trois règles simples :

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

### Ce que le moteur fait de son côté

Le découpage des fichiers ne fait que la moitié du travail. Pendant la manche,
le tirage des tuiles n'est pas uniforme :

- **Les signes neufs passent d'abord**, un par un, dans l'ordre du fichier. Un
  tirage purement aléatoire pourrait montrer le même trois fois de suite et en
  oublier un autre jusqu'à la fin.
- **Un signe raté revient** trois tuiles plus tard. C'est le seul moment où la
  correction porte encore ; le laisser filer, c'est le laisser mal appris.
- **Le tirage favorise ce qui est mal su.** Chaque signe porte une note de
  maîtrise, gardée d'une partie à l'autre : une réussite la monte d'un point,
  une erreur la descend de deux. Plus elle est basse, plus le signe revient.

Cette asymétrie entre réussite et erreur est volontaire : sans elle, un signe
raté une fois sur trois finirait par passer pour acquis.

L'écran de briefing marque en orange les signes dont la note est négative — il
dit donc quoi travailler, au lieu d'afficher une liste uniforme où les
faiblesses se noient.

### Ce que le jeu vérifie au démarrage

Un contenu incohérent affiche un écran d'erreur explicite plutôt que de casser
silencieusement le chemin d'apprentissage. Sont refusés : deux identifiants
identiques, un `requires` qui ne résout pas, un cycle de prérequis, des seuils
d'étoiles décroissants, un `review_ratio` sans prérequis à réviser, un niveau
sans signe, une clé mal orthographiée.

`cargo test` va plus loin et vérifie que chaque signe du catalogue est bien
dessinable par la police de sa langue, et que la police pixel couvre tous les
textes français — un accent manquant afficherait « Cor en » sans rien casser.

### Deux pièges

- **Les titres, sous-titres, noms et descriptions sont écrits avec la police
  pixel**, qui ne connaît que le latin. N'y mettez pas de caractères de
  l'écriture enseignée : ils s'afficheraient en tofu. Les `hint`, eux, sont
  écrits avec la police de la langue et peuvent citer les signes.
- **`speed` s'exprime en pixels virtuels par seconde**, sur une toile de
  384 × 216. Un signe parcourt 200 pixels avant d'atteindre la ligne : à 55, il
  laisse un peu moins de quatre secondes pour répondre.

## Ajouter une musique

Déposez un `.mp3`, `.ogg` ou `.wav` dans `assets/music/menu/` et recompilez.
Rien à déclarer : les fichiers sont repérés tout seuls et enchaînés dans un
ordre aléatoire sur tous les écrans **sauf la manche**, qui reste silencieuse
pour ne pas couvrir les bruitages.

Le moteur audio ne sait pas lire le MP3 : le jeu décode lui-même les morceaux
puis les lui confie en WAV brut. C'est ce décodage qui donne aussi leur durée
exacte, dont la playlist a besoin pour savoir quand enchaîner.

Une seule piste est décodée à la fois, au moment où elle démarre : un morceau
de cinq minutes prend environ trois dixièmes de seconde à décoder et occupe
une cinquantaine de mégaoctets pendant qu'il joue. Des morceaux courts sont
plus économes. Leur poids s'ajoute par ailleurs directement à celui de
l'exécutable, puisque tout est embarqué.

Le dossier peut rester vide : le jeu se lance alors sans musique.

### Ajouter une police

Déposez le `.ttf` dans `assets/fonts/` et nommez-le dans `language.toml`. Une
police CJK complète pèse plusieurs mégaoctets et alourdit d'autant le binaire :
n'ajoutez que celles qui servent, et une seule graisse.

## Organisation du code

| Dossier | Rôle |
|---|---|
| `src/data/` | Lecture et validation des fichiers de langue |
| `src/gfx/` | Toile virtuelle, palette, polices, briques d'interface |
| `src/screens/` | Un fichier par écran |
| `src/session.rs` | Une manche : règles, tuiles, score, bilan |
| `src/progress.rs` | Étoiles gagnées, déverrouillage |
| `src/settings.rs` | Réglages du joueur, volumes |
| `src/storage.rs` | Sauvegardes, fichiers ou stockage navigateur |
| `src/audio.rs` | Bruitages synthétisés au démarrage |
| `src/music.rs` | Playlist des menus, décodage des morceaux |
| `src/compose.rs` | Le générateur de la musique « Claude » |
| `src/app.rs` | État global et pile de navigation |

Tout est dessiné sur une toile de 384 × 216 agrandie d'un facteur **entier** en
filtrage au plus proche. C'est ce qui donne de vrais pixels carrés ; les écrans
raisonnent donc en pixels virtuels et n'appellent jamais `screen_width()`.

### Raccourcis de développement

```sh
ALPHATILES_START=languages           cargo run
ALPHATILES_START=options             cargo run
ALPHATILES_START=path:ja-hiragana    cargo run
ALPHATILES_START=briefing:ko/ko-01   cargo run
ALPHATILES_START=play:ko/ko-03       cargo run

# Capture une image après N frames puis quitte, pour vérifier un écran.
ALPHATILES_SCREENSHOT=ecran.png ALPHATILES_SCREENSHOT_AFTER=120 cargo run

# Régénère la musique d'ambiance après avoir retouché src/compose.rs.
ALPHATILES_COMPOSE=assets/music/menu/Claude.wav cargo run --release
```

## Crédits

- Polices : [Press Start 2P](https://fonts.google.com/specimen/Press+Start+2P),
  [Noto Sans KR](https://fonts.google.com/noto/specimen/Noto+Sans+KR) et
  [Noto Sans JP](https://fonts.google.com/noto/specimen/Noto+Sans+JP), toutes
  sous SIL Open Font License.
- Palette : « Sweetie 16 » de GrafxKid, domaine public.
- Bruitages et musique « Claude » : synthétisés par le jeu lui-même, voir
  `src/audio.rs` et `src/compose.rs`.
