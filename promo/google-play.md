# Glyphfall sur Google Play — tous les champs

Tout ce que la Play Console demande, prêt à coller. Les longueurs sont
vérifiées : chaque champ annonce le nombre de caractères utilisés sur le
maximum autorisé.

Les chiffres cités viennent du dépôt, pas d'une estimation : **4 parcours,
58 niveaux, 208 signes, 719 enregistrements de voix**.

---

## Fiche Play Store

### Langue par défaut

**Français (France)** — et elle seule.

Le jeu est écrit en français : ses consignes, ses moyens mnémotechniques, ses
équivalents de prononciation. Une fiche en anglais attirerait des joueurs qui
ne pourraient pas lire l'écran. À rouvrir le jour où le jeu sera traduit, pas
avant.

### Nom de l'application — 30 caractères

```
Glyphfall : hangeul et kana
```

Le nom seul, `Glyphfall`, ne dit rien à qui ne le connaît pas et ne se
rattache à aucune recherche. Les deux mots ajoutés sont ceux que les gens
tapent.

### Description courte — 80 caractères

```
Un jeu d'arcade où l'on apprend à lire le coréen et le japonais sans le voir.
```

### Description complète — 4000 caractères

```
Les signes tombent. Vous tapez leur lecture avant qu'ils touchent le sol.
C'est tout, et c'est un jeu d'arcade avant d'être autre chose.

Au bout de quelques parties, vous lisez le hangeul. Personne ne vous a
demandé de réviser.

QUATRE PARCOURS, 208 SIGNES

- Coréen — les 14 consonnes et les 10 voyelles du hangeul, puis les syllabes
- Hiragana — les 71 signes, des cinq voyelles aux sons composés
- Katakana — les 71 mêmes sons, dans l'autre écriture
- Kanji — 20 caractères pour commencer : les nombres, les jours de la semaine

58 niveaux, dans un ordre qui a été pensé : trois traits d'abord, puis trois
de plus, puis une révision. Jamais plus de quelques signes nouveaux à la
fois, et toujours de quoi les rattacher à ce qui précède.

QUATRE MODES

- NORMAL — noté en étoiles, du passable au parfait
- RAPIDE — sans faute, ou à refaire
- ULTRA — un signe toutes les 1,2 seconde dès le premier niveau
- INFINI — pas de chronomètre : la chute accélère jusqu'à ce que vous cédiez

CE QU'IL Y A DERRIÈRE CHAQUE SIGNE

Touchez un signe et sa fiche s'ouvre : son tracé, trait par trait, dans
l'ordre où il s'écrit. Un moyen de le retenir, écrit à la main pour ce
signe-là. Sa prononciation, avec l'équivalent français le plus proche — et
l'enregistrement d'une vraie voix pour le coréen, les hiragana et les
katakana.

719 enregistrements. Les kanji sont muets : leur lecture dépend du mot qui
les entoure, et un enregistrement isolé aurait enseigné quelque chose de
faux.

CE QU'IL N'Y A PAS

- Pas de publicité
- Pas d'achat
- Pas de compte
- Pas de connexion : tout fonctionne hors ligne, dans l'avion comme au sous-sol
- Aucune permission demandée. Le jeu ne peut pas lire vos fichiers, vos
  contacts ni votre position, parce qu'il ne les demande jamais.

Votre progression reste sur votre téléphone et n'en sort pas.

LIBRE

Glyphfall est un logiciel libre, sous licence GPL-3.0. Le code est public :
n'importe qui peut le lire, le modifier, en faire sa propre version — à
condition qu'elle reste libre elle aussi.

https://github.com/Game-K-Hack/glyphfall
```

### Éléments graphiques

| champ | fichier | format |
|---|---|---|
| Icône | `promo/icone-512.png` | 512×512 |
| Image de présentation | `promo/banniere-1024x500.png` | 1024×500 |
| Captures téléphone | `promo/captures/` — 5 fichiers | 1080×1920 |
| Vidéo | lien YouTube vers `promo/glyphfall-16-9.mp4` | URL, pas un fichier |

Ordre conseillé pour les captures : `1-jeu`, `2-fiche`, `4-chemin`,
`5-prononciation`, `3-titre`. Le jeu d'abord, ce qu'il enseigne ensuite,
l'écran-titre en dernier — il ne montre rien.

### Catégorie et coordonnées

- Type d'application : **Jeu**
- Catégorie : **Éducatif**
- Tags (5 maximum) : apprentissage des langues, jeu de mots, arcade, rétro,
  éducatif
- Adresse e-mail : la vôtre
- Site web : `https://game-k-hack.github.io/glyphfall/`

---

## Contenu de l'application

### Règles de confidentialité

```
https://game-k-hack.github.io/glyphfall/confidentialite.html
```

**La page existe dans le dépôt mais n'est pas encore en ligne** : GitHub Pages
doit être activé (Settings → Pages → Source : GitHub Actions). Sans cela
l'URL renvoie 404 et Play refuse la fiche.

### Accès à l'application

> Toutes les fonctionnalités sont disponibles sans restriction d'accès.

Pas de compte, pas de code, rien à fournir aux évaluateurs.

### Annonces

> Non, mon application ne contient pas d'annonces.

### Classification du contenu (questionnaire IARC)

Catégorie déclarée : **Jeu**. Toutes les réponses sont **non** :

| question | réponse |
|---|---|
| Violence, réaliste ou stylisée | Non |
| Sang, blessures | Non |
| Contenu sexuel, nudité | Non |
| Langage grossier | Non |
| Drogue, alcool, tabac | Non |
| Jeux d'argent, simulation de jeux d'argent | Non |
| Contenu effrayant | Non |
| Interaction entre utilisateurs | Non |
| Partage de position | Non |
| Partage d'informations personnelles | Non |
| Achats numériques | Non |

Résultat attendu : **PEGI 3 / ESRB Everyone**.

### Public cible et contenu

- Tranches d'âge : **13-15, 16-17, 18 et plus**

Ne cochez pas « moins de 13 ans ». Le jeu conviendrait à un enfant, mais cette
case déclenche le programme Families : conception validée séparément, règles
publicitaires supplémentaires, examen plus long. Vous pourrez l'ajouter plus
tard si vous le voulez ; l'enlever après coup est bien plus pénible.

- Attire-t-elle involontairement les enfants ? **Non**

### Sécurité des données

- Collecte ou partage de données utilisateur : **Non**
- Données traitées de manière éphémère : **Non**
- Mécanisme de suppression des données : **sans objet**

C'est vérifiable ligne à ligne : le manifeste Android ne demande **aucune
permission**, et l'application n'ouvre aucune connexion réseau. La progression
est écrite dans l'espace privé de l'application, que le système efface avec
elle.

### Déclarations restantes

| déclaration | réponse |
|---|---|
| Application financière | Non |
| Application de santé | Non |
| Application gouvernementale | Non |
| Contenu généré par les utilisateurs | Non |
| Application d'actualités | Non |
| COVID-19 / contact tracing | Non |

---

## Version

### Fichier

`target/android/glyphfall.aab` — produit par `python tools/bundle.py`, ou
récupéré dans l'artefact `play-bundle` du workflow.

### Nom de la version

```
0.4.0 (400)
```

### Notes de version — 500 caractères

Ce ne sont **pas** les notes de la release GitHub. Pour qui télécharge depuis
Play, il n'y a pas eu de 0.3.x : c'est la première fois qu'il voit le jeu.
Lui parler de la signature de l'APK ou du retour d'un artefact Windows n'a
aucun sens. On présente donc le jeu, pas le journal des changements.

Le changelog reprendra son rôle à la version suivante, quand les joueurs
auront quelque chose à comparer.

```
Première version publiée sur Google Play.

Quatre parcours : coréen, hiragana, katakana et premiers kanji. 58 niveaux,
208 signes, quatre modes de jeu du plus calme au sans répit.

Chaque signe a sa fiche : le tracé trait par trait, un moyen de le retenir,
sa prononciation — et la voix d'un locuteur pour le coréen et les kana.

Sans publicité, sans compte, sans connexion. Aucune permission demandée.
```

---

## L'ordre des opérations

1. Activer GitHub Pages, vérifier que l'URL de confidentialité répond
2. Créer la clé de signature et construire l'AAB
3. Créer l'application dans la console, remplir « Contenu de l'application »
4. Remplir la fiche Play Store avec ce qui précède
5. Téléverser la vidéo sur YouTube, coller le lien
6. Version en **test interne** — c'est là que Play valide le bundle
7. **Test fermé** : douze testeurs, quatorze jours (comptes personnels)
8. Demander l'accès à la production, puis publier

L'étape 7 est la seule qu'on ne peut pas raccourcir. Commencez à réunir les
douze personnes dès maintenant : tout le reste tient en une soirée.
