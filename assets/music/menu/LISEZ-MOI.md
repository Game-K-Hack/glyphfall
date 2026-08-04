# Musiques de menu

Déposez vos fichiers ici : ils sont joués **en boucle et dans un ordre
aléatoire** sur l'écran-titre, le choix de la langue, le chemin
d'apprentissage, le briefing et les résultats. La manche, elle, reste
silencieuse pour ne pas concurrencer les bruitages.

## Formats acceptés

`.mp3`, `.ogg`, `.wav`. Rien à déclarer nulle part : tout fichier portant une
de ces extensions est repéré tout seul.

**Il faut recompiler après avoir ajouté un fichier** (`cargo build`) : le
contenu est embarqué dans l'exécutable, comme les langues et les polices, pour
qu'il n'y ait rien à distribuer à côté du binaire et que la version navigateur
fonctionne.

## À savoir

- Le poids des fichiers s'ajoute directement à celui de l'exécutable. En OGG à
  128 kbit/s, comptez environ 1 Mo par minute.
- Une seule piste est décodée à la fois, au moment où elle démarre. Un MP3 doit
  être décodé entièrement en mémoire : une piste de trois minutes en occupe une
  soixantaine de mégaoctets le temps qu'elle joue. Des morceaux de une à deux
  minutes qui bouclent sont plus économes qu'un long morceau.
- L'OGG est le meilleur choix : à qualité égale il pèse moins lourd que le MP3
  et bien moins qu'un WAV.
- Ce dossier peut rester vide : le jeu se lance alors simplement sans musique.

Ce fichier est ignoré par le jeu, seules les extensions audio sont lues.
