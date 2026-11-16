# HISTORIQUE — sketchcraft

Ce qui est fait. Une entrée par changement livré, ajoutée à la fin ; les entrées passées ne sont pas modifiées.
L'historique de Sawhorse avant la copie est dans git (`git log a7a637d`) et dans CHANGELOG.md.

## H-20261116-1 · évolution · documentation · Copie de départ de Sawhorse

- Origine : JC, 16 nov. 2026, 10 h 49 : « copie dans un dossier de ta machine. »
- Fichiers : dépôt Sawhorse entier (https://github.com/sawhorse3d/sawhorse), historique complet (340 commits), dernier commit a7a637d du 15 nov. 2026 ; branche `sketchcraft` créée pour nos changements ; source distante renommée `upstream`.
- Évolution : le projet sketchcraft part de Sawhorse 0.1.0-alpha.6 (licence MIT ou Apache-2.0, au choix).
- Écarté : un fork sur GitHub (JC a choisi la copie locale).
- Effet sur les résultats : aucun.
- Effet sur les projets existants : aucun.
- Confidentialité : non.
- Vérification : copie de travail de Claude : `npx vitest run` 881 tests passés (253 ignorés par le projet), `cargo test --workspace` 692 tests passés (3 ignorés). Machine jc-ms-7c37 : paquet `sketchcraft.bundle` déposé dans `~/sketchcraft`, relu et comparé par empreinte (SHA-256 a253889095d8e671…) ; cloné par JC dans `~/sketchcraft/code` ; noyau compilé par JC (`npm run wasm`, Rust installé par rustup) ; application ouverte dans Firefox, vue 3D affichée.
- Limites connues : dans le navigateur intégré de l'app Claude, la vue 3D reste vide (pas de WebGL2) ; dans la copie de travail de Claude, le noyau WebAssembly ne se compile pas (cible wasm32 non téléchargeable). Node 22.22.1 sur jc-ms-7c37 (Sawhorse demande Node 24) : avertissements seulement.

## H-20261116-2 · évolution · interface · Rapporteur (E-01)

- Origine : JC, 16 nov. 2026, 10 h 41 : « il faut juste les outils de base comme le crayon, le rapporteur d'angle et quelques outils typique qu'il manque pour faire le même travail que  sketchup pour les volumétries externes des batiments » ; E-01.
- Fichiers :
  - `src/tools/protractor.ts` (nouveau) : l'outil Rapporteur et la lecture des angles tapés (`parseProtractorAngle`, `fmtSlope`).
  - `src/tools/protractor.test.ts` (nouveau) : 7 tests.
  - `src/tools/rotate.ts` : `AXIS_KEYS`, `axisWords` et `rotate` exportés (aucun changement de comportement).
  - `src/lib/tools.ts` : l'outil `protractor` sur la barre d'outils, groupe Mètre et Cote, sans touche par défaut (comme SketchUp 2017).
  - `src/components/glyphs.tsx` : son icône (un demi-rapporteur et son aiguille).
  - `src/App.tsx` : l'outil créé, et Outils > Protractor dans le menu.
  - `src/App.test.tsx` : la barre compte 19 outils ; le rapporteur n'a pas de touche.
- Évolution : clic sur le sommet, clic le long de la ligne de référence (une arête, un axe), puis clic où va la ligne de construction (l'angle s'accroche aux graduations de 15°), ou angle tapé : degrés (`35`, `-22.5`), pente en pour cent (`70%`), rapport hauteur:longueur (`1:2`). La ligne de construction passe par le sommet, du côté de la référence où est le curseur. Le plan du rapporteur suit la face sous le curseur ; les flèches le verrouillent (droite : vertical face au rouge ; gauche : vertical face au vert ; haut : à plat), Maj garde le plan. Le message dit l'angle et sa pente (« Guide at 26.565° (50%) »). Une annulation par ligne posée.
- Écarté : passer par le noyau (une nouvelle opération) : inutile, l'opération `add_guide` accepte déjà une direction quelconque ; une touche par défaut : SketchUp 2017 n'en avait pas, à décider par JC.
- Effet sur les résultats : aucun sur la géométrie ; ajoute des lignes de construction (déjà prévues par le format `.shz`).
- Effet sur les projets existants : aucun ; pas de changement de format.
- Confidentialité : non.
- Vérification : tests de fonctionnement : 7 scénarios neufs (angles tapés ; 45° au sol ; pente 70 % du côté du curseur ; pente de toit 1:2 sur un plan vertical verrouillé ; refus en clair d'un angle tapé trop tôt ; présence sur la barre), qui échouent sans le changement (outil absent) ; suite complète : 888 passés (881 de la référence + 7), 253 ignorés, aucun échec ; vérification des types (`tsc -b`) sans erreur. Niveau : testé (tests unitaires) ; non essayé dans le navigateur (copie de travail sans noyau WebAssembly) : essai par JC à faire.
- Limites connues : intitulés en anglais comme le reste de l'application ; libellé court « Protr. » sous l'icône ; pas de verrou de l'angle sur une arête parallèle (seulement les graduations de 15°).

## H-20261116-3 · correction · module · Accrochage : origine, axes du modèle, croisements, tolérance d'axe (E-14)

- Origine : JC, 16 nov. 2026, 13 h 00 : « l'accrochage a un comportement assez différents de sketchup , il n'y a pas un menu pour l'activer . Je cherche un comportement du style sketchup ( le but ici n'est pas de cloner sketchup ) uniquement avoir suffisament de fonctions pour les volumétrie des batiments » ; précisé : « ici je n'ai pas l'accochage à l'origine j'ai tracé la ligne du bas , la ligne verticale, j'essaye de tracer la ligne en // pour dessiner le rectangle mais ca ne marche pas il ne suis pas l'axe et ne détecte pas le point de fin qui est alligné sur le premier tracé » ; E-14.
- Constat : dans `crates/inference/src/infer.rs`, (1) l'origine n'était pas un point d'accrochage ; (2) les axes du modèle n'étaient pas des lignes d'accrochage : le point au-dessus de l'origine n'était trouvé que par « From Point », après avoir survolé l'origine ; (3) une ligne d'axe partant du départ ne s'arrêtait que sur une arête, pas sur une ligne de construction ; (4) la tolérance d'une ligne partant du départ était de 7 pixels quelle que soit la longueur du trait.
- Fichiers :
  - `crates/inference/src/infer.rs` : l'origine comme extrémité ; les trois axes du modèle comme lignes (rang des arêtes et des lignes de construction ; longueur arrondie à la grille comme le long d'un axe ; pas proposés quand le départ est déjà sur cet axe) ; les lignes partant du départ s'arrêtent aussi sur une ligne de construction ou un axe du modèle (« Intersection ») ; tolérance des lignes partant du départ : 7 px près du départ, puis environ 3,4° vus du départ, au plus 30 px (`start_line_px`).
  - `crates/inference/tests/infer.rs` : 5 tests neufs (dont le cas de JC) ; un test existant ajusté (`the_grid_snaps_only_when_nothing_else_is_near` : un point à 3 px de l'axe rouge accroche désormais l'axe, à la même longueur arrondie ; un point hors des axes garde la grille).
- Correction : en traçant à la ligne, l'origine s'accroche ; les axes rouge, vert et bleu s'accrochent (« On Blue Axis ») ; la ligne parallèle au premier tracé s'arrête d'elle-même au-dessus de l'origine, et sur une ligne de construction (par ex. celle du rapporteur) ; l'axe tient sur un trait long.
- Écarté : élargir toutes les tolérances (les extrémités et arêtes gardent 10 et 7 px) ; un menu d'accrochage dans cette correction (→ E-15, proposée).
- Effet sur les résultats : les points accrochés changent près de l'origine et des axes (voir le test ajusté) ; aucun effet sur la géométrie existante.
- Effet sur les projets existants : aucun ; pas de changement de format. Il faut recompiler le noyau (`npm run wasm`).
- Confidentialité : non.
- Vérification : 5 scénarios neufs, qui échouaient tous avant la correction et passent après ; 4 tests existants cassés par la première version (départ sur un axe) réparés par la règle « pas proposé quand le départ est sur cet axe », sans les modifier ; un test ajusté (ci-dessus). `cargo test --workspace` : 697 passés (692 de la référence + 5), 3 ignorés, aucun échec. Niveau : testé (tests du noyau d'accrochage) ; non essayé dans le navigateur : essai par JC à faire.
- Limites connues : l'accrochage à la grille reste toujours actif sur le sol (E-15) ; couleurs d'accrochage propres à Sawhorse, pas celles de SketchUp.
