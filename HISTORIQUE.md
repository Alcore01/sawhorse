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

## H-20261116-4 · évolution · interface · Affichage > Snap to Grid (E-15)

- Origine : JC, 16 nov. 2026, 13 h 47 : « il s'accroche à la grille en permanence . pas moyen de décocher les accrochages » ; E-15.
- Fichiers :
  - `src/tools/driver.ts` : `kernelResolver` reçoit l'état de l'interrupteur ; arrêté, la requête d'inférence part avec `grid: false` pour tous les outils.
  - `src/App.tsx` : l'interrupteur (préférence du navigateur, clé `sketchcraft.snapToGrid`, arrêté par défaut), l'entrée Affichage > Snap to Grid et son message ; ajouté aux dépendances du menu.
  - `src/tools/driver-grid.test.ts` (nouveau) : 2 tests ; `src/App.test.tsx` : 1 test.
- Évolution : Affichage > Snap to Grid. Arrêté (par défaut) : plus d'accrochage aux points de la grille ni d'arrondi des longueurs tracées à la souris ; les extrémités, milieux, arêtes, faces, axes, lignes de construction et le parallèle et la perpendiculaire s'accrochent toujours ; les valeurs tapées restent exactes. Marche : comme Sawhorse d'origine.
- Écarté : garder le choix dans le modèle (c'est une préférence de travail, pas une donnée du dessin) ; un interrupteur qui couperait toutes les inférences (on ne pourrait plus tracer juste ; à décider par JC).
- Effet sur les résultats : arrêté, les points tracés à la souris sur le sol ne tombent plus sur la grille et les longueurs ne sont plus arrondies (signe « ~ » devant la longueur).
- Effet sur les projets existants : aucun ; pas de changement de format.
- Confidentialité : non (préférence gardée dans le navigateur).
- Vérification : 3 tests neufs (l'interrupteur arrêté envoie `grid: false` même à un outil qui demande la grille ; en marche, chaque outil garde son choix ; le menu est arrêté par défaut, bascule et se souvient après rechargement), qui échouent sans le changement ; un premier essai a montré que le menu, mis en cache, ne se rafraîchissait pas : dépendance ajoutée. Suite complète : 891 passés (888 + 3), 253 ignorés, aucun échec. Niveau : testé ; essai dans le navigateur par JC à faire.
- Limites connues : pas de raccourci clavier pour l'interrupteur.

## H-20261116-5 · évolution · module|interface · Accrochages activables dans la légende (E-16) ; orbite pendant un tracé (E-17)

- Origine : JC, 16 nov. 2026, 14 h 04 : « d'une manière générale dans cette partie de l'écran . des accrochages activables et désactivables. » ; « j'ai aussi un autre soucis , si je crée une ligne et qu'avant de l'avoir terminée je fait une rotation de la vue cette rotation utilise le point en train d'étre construit . dans sketchup ce n'est pas du tout le cas » ; E-16, E-17.
- Constat (E-17) : `src/viewport/controls.ts` prend pour pivot de l'orbite le point sous le curseur ; pendant un tracé, le curseur est sur le point en construction, qui devenait le pivot.
- Fichiers :
  - `crates/inference/src/infer.rs` : `InferQuery.disabled` (types coupés) ; leurs candidats (points, lignes, lignes partant du départ, croisements, grille, arrondi des longueurs) sont écartés avant le choix ; un verrou (flèches, Maj) garde sa ligne. Tests : `tests/infer.rs` (1 test neuf ; champ ajouté à 3 requêtes de test : `infer.rs`, `perf.rs`, `sections.rs`).
  - `src/lib/inferenceStyle.ts` : les interrupteurs (`SNAP_SWITCHES` : Endpoint, Midpoint, On Edge, Intersection, Centre, Axes, Parallel / Perp., From Point, Guides, Grid) et `disabledKinds`.
  - `src/kernel/types.ts` : le champ `disabled` de la requête ; `src/tools/driver.ts` : il est transmis.
  - `src/components/Viewport.tsx`, `src/styles/app.css` : la légende devient une rangée de boutons (barré et atténué quand coupé) ; On Face reste affiché, sans bouton.
  - `src/App.tsx` : l'état des interrupteurs (préférence du navigateur, clé `sketchcraft.snapOff`) ; Grid = Snap to Grid ; l'orbite pendant un geste.
  - `src/viewport/controls.ts`, `src/viewport/engine.ts` : `orbitAboutView` : pendant un geste d'outil, le pivot de l'orbite est le point visé de la vue.
  - Tests : `src/tools/driver-grid.test.ts` (1), `src/App.test.tsx` (1), `src/viewport/controls.test.ts` (1).
- Évolution : (E-16) un clic sur un type d'accrochage dans la légende le coupe ou le rétablit ; le message le dit ; le choix se garde d'une visite à l'autre. (E-17) pendant un tracé (une ligne commencée, un rectangle…), le bouton du milieu tourne la vue autour de son point visé, comme SketchUp ; hors geste, rien ne change (le point sous le curseur).
- Écarté : un interrupteur pour On Face (on ne pourrait plus dessiner sur une face) ; garder les choix dans le modèle (préférence de travail).
- Effet sur les résultats : aucun quand tout est actif (les requêtes sans `disabled` sont inchangées).
- Effet sur les projets existants : aucun ; pas de changement de format. Il faut recompiler le noyau (`npm run wasm`).
- Confidentialité : non.
- Vérification : 4 tests neufs, échouant sans le changement : noyau (point coupé → l'arête prend la main ; axes coupés mais verrou tenu ; grille coupée) ; requête (les types coupés partent au noyau) ; légende (bascule et mémoire) ; orbite (pivot = point visé pendant un geste, point sous le curseur sinon). Rust : 698 passés (697 + 1), 3 ignorés ; TypeScript : 894 passés (891 + 3), 253 ignorés ; aucun échec. Niveau : testé ; essai dans le navigateur par JC à faire.
- Limites connues : le pivot « point visé » est celui de la vue, pas le centre du modèle ; pas de raccourcis clavier pour les interrupteurs.

## H-20261116-6 · correction · interface · Ligne continue qui change de plan (E-18)

- Origine : JC, 16 nov. 2026, 14 h 20 : « quand on dessine dessigne les lignes ici je dessines sur l'axe vert, je veux sans interrompre continuer le trais sur l'axe bleu et il ne s'accroche plus sur l'axe, si j'interrompt le trais et que je continue avec une nouvelle ligne il reprend l'accrochage » ; E-18.
- Constat : `src/tools/line.ts` fixe le plan de la chaîne au premier segment (« Line 100 cm on the ground ») et le transmet à l'accrochage ; le noyau d'accrochage écarte alors l'axe bleu (hors du plan), et un point hors du plan était refusé (« off this line's plane »).
- Fichiers :
  - `src/tools/line.ts` : une chaîne « libre » (pas commencée dans une face, pas de plan choisi aux flèches) ne transmet plus son plan à l'accrochage ; un segment qui quitte le plan va sur un nouveau plan ; si le segment précédent était seul sur son plan et n'est pas aligné, il est annulé et retracé avec le nouveau sur le plan qu'ils partagent (une annulation) ; le segment précédent sert aussi de référence pour le parallèle et la perpendiculaire.
  - `src/tools/draw.test.ts` : 3 tests.
- Correction : comme dans SketchUp, on trace au sol puis, sans lâcher, on monte le long de l'axe bleu ; un rectangle vertical tracé d'un trait depuis le sol se ferme en face sur un seul plan.
- Écarté : garder des segments à cheval sur deux esquisses sans les retracer (le rectangle ne formerait pas de face) ; retracer des segments plus anciens (ils peuvent appartenir à une forme au sol).
- Effet sur les résultats : une chaîne libre n'est plus refusée hors de son plan ; l'annulation du segment retracé et du nouveau se fait en un pas.
- Effet sur les projets existants : aucun ; pas de changement de format.
- Confidentialité : non.
- Vérification : 3 tests neufs ; les 2 premiers (montée sans interruption avec retracé et fermeture du rectangle vertical ; changement de plan après deux segments au sol, sans annulation) échouent sans le changement ; le 3e (plan choisi aux flèches : la chaîne reste tenue) passe avant et après, il garde l'ancien comportement voulu. Suite complète : 897 passés (894 + 3), 253 ignorés, aucun échec. Niveau : testé (hôte de test, sans noyau) ; non vérifié avec le vrai noyau (la suite Playwright `e2e/draw.spec.ts` ne tourne pas ici) : essai par JC à faire.
- Limites connues : seul le segment immédiatement précédent est retracé ; une chaîne libre en l'air sans accrochage tombe sur le sol (comme SketchUp).

## H-20261116-7 · correction · module · Une forme se ferme contre l'arête d'une autre esquisse (E-19)

- Origine : JC, 16 nov. 2026, 14 h 28 : « j'ai réalisé une deuxième rectangle attenant mais sa surface ne se rempli pas » ; E-19.
- Constat : le noyau range chaque tracé dans l'esquisse de son plan (`Model::sketch_on`) et ne cherche les contours fermés que parmi les arêtes de cette esquisse. L'arête verticale commune aux deux rectangles appartient à l'esquisse du premier (plan y = 0) : dans le plan du second (x = 0), le contour restait ouvert. Reproduit par un test (`regions: []`).
- Fichiers :
  - `crates/kernel/src/model.rs` : `draw_in_sketch` : après un tracé ouvert, les arêtes des autres esquisses du même contexte qui sont dans ce plan et touchent une extrémité du tracé sont tracées aussi dans cette esquisse, quand leurs deux bouts y sont déjà des points (seul cas où elles peuvent fermer un contour) ; `coplanar_edges_at` les trouve.
  - `crates/kernel/tests/draw.rs` : 1 test (le cas de JC, et un tracé qui ne touche une arête que par un bout n'importe rien).
  - `crates/inference/src/infer.rs` : mise en forme seule (`cargo fmt`) d'une ligne de H-20261116-3.
- Correction : comme dans SketchUp, où les arêtes sont partagées : le second rectangle attenant se remplit.
- Écarté : fusionner les esquisses de plans différents (une esquisse est plane) ; copier toutes les arêtes coplanaires à chaque tracé (des doublons inutiles).
- Effet sur les résultats : l'arête commune existe désormais dans les deux esquisses (une par plan) ; les autres tracés ne changent pas.
- Effet sur les projets existants : aucun à l'ouverture ; pas de changement de format. Il faut recompiler le noyau (`npm run wasm`).
- Confidentialité : non.
- Vérification : 1 test neuf, qui échouait avant (`regions: []`) et passe après ; `cargo test --workspace` : 699 passés (698 + 1), 3 ignorés, aucun échec (dont les empreintes « golden » du noyau). Niveau : testé (noyau) ; essai dans le navigateur par JC à faire.
- Limites connues : les arêtes des faces des solides (volumes déjà poussés) ne sont pas encore partagées ; un tracé fermé d'un coup (`closed`) n'en importe pas (il n'en a pas besoin pour se fermer) ; effacer l'arête commune n'efface que celle de l'esquisse visée.

## H-20261116-8 · correction · module · Fermeture contre les arêtes d'autres esquisses, généralisée (E-19)

- Origine : JC, 16 nov. 2026, 14 h 58 : capture sans texte d'une boîte de quatre murs tracés à la ligne, le quatrième non rempli ; E-19 ; suite de H-20261116-7.
- Constat : la règle de H-20261116-7 n'importait une arête d'une autre esquisse que si ses deux bouts étaient déjà des points du tracé. Le 4e mur se ferme avec trois arêtes de trois esquisses (les deux montants des murs, le pied au sol) : le pied ne touchait pas le tracé. De plus, une ligne seule n'a pas de plan : l'outil la range sur un plan horizontal, où rien ne se ferme. Reproduit par un test, qui échoue avec le noyau de H-20261116-7.
- Fichiers :
  - `crates/kernel/src/model.rs` : `draw_in_sketch_closing` remplace la règle : après un tracé ouvert, les arêtes des autres esquisses du contexte situées dans son plan et reliées au tracé (de proche en proche) y sont ajoutées, seulement si elles ferment un contour, sinon rien ne change (`import_connected`, `edges_on_plane`) ; un tracé droit qui ne ferme rien sur son plan essaie les plans passant par lui et une arête qui le touche, et va sur le premier où il ferme un contour (esquisse existante de ce plan, ou nouvelle), l'esquisse vide laissée derrière étant retirée (`close_on_another_plane`, `straight`). Seulement pour les tracés sur un plan (cible `plane`, celle des outils de dessin) sans courbe ; `coplanar_edges_at` (H-20261116-7) est retiré.
  - `crates/kernel/tests/draw.rs` : 1 test (le 4e mur fermé d'une ligne en haut ; une ligne qui ne ferme rien reste où elle a été tracée).
- Correction : comme dans SketchUp, une ligne qui ferme un contour plan avec des arêtes existantes, quelles que soient leurs esquisses, crée la face.
- Écarté : changer de plan pour une ligne qui ne ferme rien (elle reste où l'outil la met) ; chercher les plans des arêtes qui ne touchent pas la ligne (coût, et ambiguïté).
- Effet sur les résultats : une ligne qui ferme une face peut être rangée dans une autre esquisse que celle du plan demandé ; l'issue de l'opération dit laquelle.
- Effet sur les projets existants : aucun à l'ouverture ; pas de changement de format. Il faut recompiler le noyau (`npm run wasm`).
- Confidentialité : non.
- Vérification : le test neuf échoue avec le noyau de H-20261116-7 et passe ; le test de H-20261116-7 passe toujours ; `cargo test --workspace` : 700 passés (699 + 1), 3 ignorés, aucun échec (dont le fuzzer de séquences et les empreintes « golden »). Niveau : testé (noyau) ; essai dans le navigateur par JC à faire.
- Limites connues : seules les esquisses sont partagées, pas les faces des volumes poussés ; si plusieurs plans conviennent pour une ligne, le premier trouvé l'emporte.

## H-20261116-9 · correction · module · Une ligne qui ferme deux contours crée les deux faces (E-19)

- Origine : JC, 16 nov. 2026, 15 h 21 : « la dernière face est complète mais elle  ne se ferme pas » ; E-19 ; suite de H-20261116-8.
- Constat : la dernière ligne de la boîte (le haut de la façade) fermait le toit sur son plan ; la recherche d'autres plans (H-20261116-8) ne se faisait que si rien ne se fermait sur le plan du tracé : la façade, fermée elle aussi par cette ligne, restait sans face. Reproduit par un test, qui échoue avec le noyau de H-20261116-8.
- Fichiers :
  - `crates/kernel/src/model.rs` : `close_on_other_planes` (ex-`close_on_another_plane`) essaie tous les plans passant par une ligne droite et une arête qui la touche, et la trace dans chacun où elle ferme un contour ; si elle ne ferme rien sur son propre plan, elle en est retirée (comme avant).
  - `crates/kernel/tests/draw.rs` : 1 test (le haut de la façade ferme le toit et la façade).
- Correction : comme dans SketchUp, une arête qui ferme deux contours crée les deux faces.
- Effet sur les résultats : une telle ligne existe désormais dans l'esquisse de chaque face qu'elle ferme.
- Effet sur les projets existants : aucun à l'ouverture ; une façade déjà tracée sans face se remplit en retraçant une de ses arêtes (la ligne retracée ne ferme rien sur son plan et va sur celui de la façade). Il faut recompiler le noyau (`npm run wasm`).
- Confidentialité : non.
- Vérification : le test neuf échoue avec le noyau de H-20261116-8 et passe ; `cargo test --workspace` : 701 passés (700 + 1), 3 ignorés, aucun échec. Niveau : testé (noyau) ; essai dans le navigateur par JC à faire.
- Limites connues : celles de H-20261116-8 (pas de partage avec les volumes poussés).

## H-20261116-10 · évolution · interface · Rapporteur : zéro sur la référence, angle de la référence affiché (E-20)

- Origine : JC, 16 nov. 2026, 15 h 51 : « en fait il y a plusieurs choses, quand on bouge la référence on ne sait pas ce que représente l'angle. quand on pose le compas on ne sait pas le faire tourner comme dans sketchup pour son repère d'angle , exemple ic j'ai mesuré 37 a partir du dessous de la toiture » ; E-20 (suite de E-01, H-20261116-2).
- Constat : avant le clic de la référence, le rapporteur était un cercle complet aux graduations fixes (zéro sur un axe du plan sans rapport avec le dessin), et rien ne disait l'angle de la référence en cours.
- Fichiers :
  - `src/tools/protractor.ts` : `view()` : un demi-rapporteur posé sur sa ligne de base (le diamètre par le sommet), graduations tous les 15° depuis son zéro ; ce zéro suit le curseur pendant le choix de la référence (comme SketchUp), puis reste sur la référence cliquée, marquée « 0° » ; pendant le choix, l'étiquette « ref 37° (75.4%) » donne l'angle de la référence depuis l'horizontale du plan (depuis l'axe rouge sur un plan horizontal) ; `levelIn` et `referenceAngle` (0 à 90°).
  - `src/tools/protractor.test.ts` : 2 tests.
- Évolution : on voit ce que mesure le rapporteur : la référence et son angle (ex. le dessous d'une toiture à 37°), puis l'angle de la ligne de construction depuis cette référence.
- Écarté : afficher l'angle de la référence dans la case des valeurs (elle sert à taper l'angle de la ligne de construction).
- Effet sur les résultats : aucun (affichage seul) ; les lignes de construction posées sont les mêmes.
- Effet sur les projets existants : aucun.
- Confidentialité : non.
- Vérification : 2 tests neufs (angle d'une référence, dans les deux sens, à plat ; affichage pendant le choix de la référence : base du demi-rapporteur dans la direction du curseur, étiquette « ref 37° (75.4%) », puis « 0° » sur la référence), qui échouent sans le changement ; suite complète : 899 passés (897 + 2), 253 ignorés, aucun échec. Niveau : testé ; essai dans le navigateur par JC à faire.
- Limites connues : l'angle de la référence est donné sans signe (0 à 90°) ; pas d'accrochage de la référence aux graduations (seulement aux arêtes, axes et lignes).

## H-20261116-11 · évolution · module · Accrochage à double référence (E-21)

- Origine : JC, 16 nov. 2026, 15 h 59 : « il manque aussi l’accrochage a la double références comme dans cet exemple », avec deux captures du rapporteur (alignement « From Point » horizontal sur le coin haut gauche, puis vertical sur le coin haut droit, jamais les deux) ; E-21.
- Constat : `crates/inference/src/infer.rs` ne croisait l'alignement d'un point survolé qu'avec les lignes partant du départ du geste, pas avec l'alignement d'un second point survolé.
- Fichiers :
  - `crates/inference/src/infer.rs` : pour chaque paire de points survolés (les deux derniers, `InferenceMemory.MAX_ANCHORS`), le croisement d'une ligne d'axe par l'un et d'une ligne d'axe par l'autre (dans le plan de dessin s'il y en a un) est un point « From Point », avec les deux pointillés.
  - `crates/inference/tests/infer.rs` : 1 test (sur le plan vertical du rapporteur, avec et sans point de départ).
- Évolution : comme dans SketchUp, survoler un coin puis un autre, puis aller vers la croisée de leurs alignements : le point s'y accroche.
- Effet sur les résultats : un point d'accrochage de plus près des croisées d'alignements ; coupé avec « From Point » dans la légende.
- Effet sur les projets existants : aucun ; pas de changement de format. Il faut recompiler le noyau (`npm run wasm`).
- Confidentialité : non.
- Vérification : le test neuf échoue sans le changement (point à un seul alignement) et passe ; `cargo test --workspace` : 702 passés (701 + 1), 3 ignorés, aucun échec. Niveau : testé (noyau) ; essai dans le navigateur par JC à faire.
- Limites connues : seulement les deux derniers points survolés, et seulement les axes (pas le parallèle à une arête).

## H-20261116-12 · correction · module · L'alignement d'un point survolé croise les lignes de construction et les arêtes (E-21)

- Origine : JC, 16 nov. 2026, 16 h 12 : « si j'essaye de l'accrocher le croisement disparait », avec deux captures : une ligne tracée le long d'une ligne de construction de pente (rapporteur), à l'aplomb d'un coin survolé ; le croisement s'affiche à l'approche (« From Point »), puis « On Guide » l'emporte sur la ligne ; E-21.
- Constat : `crates/inference/src/infer.rs` ne croisait l'alignement d'un point survolé qu'avec les lignes partant du départ et (H-20261116-11) avec celui d'un autre point survolé ; sur la ligne de construction, seul « On Guide » restait. Reproduit par un test.
- Fichiers :
  - `crates/inference/src/infer.rs` : le croisement d'une ligne d'axe par un point survolé avec une ligne de construction, ou avec une arête proche du curseur, est un point « From Point » (il l'emporte sur « On Guide » et « On Edge », comme un point).
  - `crates/inference/tests/infer.rs` : 1 test (la pente de toit à 37° et l'aplomb du coin ; une arête au sol et l'aplomb d'un coin).
- Correction : comme dans SketchUp, le point sur la ligne de construction à l'aplomb du coin s'accroche.
- Effet sur les résultats : des points d'accrochage de plus aux croisées des alignements avec les lignes de construction et les arêtes ; coupés avec « From Point » dans la légende.
- Effet sur les projets existants : aucun ; pas de changement de format. Il faut recompiler le noyau (`npm run wasm`).
- Confidentialité : non.
- Vérification : le test neuf échoue sans le changement (« On Guide ») et passe ; `cargo test --workspace` : 703 passés (702 + 1), 3 ignorés, aucun échec. Niveau : testé (noyau) ; essai dans le navigateur par JC à faire.
- Limites connues : seulement les deux derniers points survolés, et leurs alignements le long des axes.

## H-20261116-13 · correction · interface · Rapporteur : angle négatif tapé (E-22)

- Origine : JC, 16 nov. 2026, 16 h 43 : « il ne supporte pas les entrées manuelle avec un chiffre négatif », avec une capture : curseur à « -30° (57.7%) », « -37 » tapé ; E-22.
- Constat : `ProtractorTool.onValue` multipliait toujours l'angle tapé par le signe du côté du curseur : avec le curseur côté négatif, « -37 » devenait +37°, de l'autre côté de la référence. Reproduit par un test.
- Fichiers : `src/tools/protractor.ts` (un angle tapé négatif est pris tel quel, signé comme l'étiquette ; un angle positif va du côté du curseur, comme avant) ; `src/tools/protractor.test.ts` (1 test, curseur d'un côté puis de l'autre).
- Correction : « -37 » place la ligne de construction à -37°, comme l'étiquette le montre.
- Effet sur les résultats : seulement les angles tapés négatifs.
- Effet sur les projets existants : aucun.
- Confidentialité : non.
- Vérification : le test neuf échoue sans le changement et passe ; suite complète : 900 passés (899 + 1), 253 ignorés, aucun échec. Niveau : testé ; essai par JC à faire.
- Limites connues : une pente négative (« -70% », « -1:2 ») suit la même règle.

## H-20261116-14 · évolution · interface · Gomme : arêtes seulement (E-23)

- Origine : JC, 16 nov. 2026, 17 h 13 : « la gomme efface le contenu des faces , dans sketchup ce n'est pas le cas. on utilise la flèche de sélection et la touche del/suppr pour retirer les faces » ; E-23.
- Fichiers : `src/tools/erase.ts` (un clic sur une face ou une surface d'esquisse n'efface rien et affiche « The Eraser takes edges. To remove a face, select it (Space) and press Delete. » ; le glissé ne ramasse plus que les arêtes et courbes ; le survol ne surligne plus que les arêtes et courbes ; Sélection + Suppr inchangé) ; `src/tools/erase.test.ts` (1 test).
- Évolution : comme dans SketchUp, la gomme efface les arêtes (et la face qu'elles bordent disparaît avec elles, comme avant) ; une face se retire en la sélectionnant puis avec Suppr.
- Écarté : garder l'effacement des faces imprimées à la gomme (comportement de Sawhorse) : contraire à l'usage de SketchUp demandé.
- Effet sur les résultats : aucun sur la géométrie ; moins d'effacements involontaires.
- Effet sur les projets existants : aucun.
- Confidentialité : non.
- Vérification : 1 test neuf (clic sur une face et sur une surface d'esquisse, glissé sur une surface : rien n'est effacé, le message s'affiche ; une arête s'efface toujours), qui échoue sans le changement ; un premier essai a montré qu'un clic sur un sommet doit garder son refus d'origine (test existant) : le message est limité aux faces et surfaces. Suite complète : 901 passés (900 + 1), 253 ignorés, aucun échec. Niveau : testé ; essai par JC à faire.
- Limites connues : le guide utilisateur (docs/USER-GUIDE.md) décrit encore l'ancien comportement.

## H-20261116-15 · évolution · documentation|interface · Lanceur et installation Linux et Windows (E-24)

- Origine : JC, 16 nov. 2026, 20 h 15 : « creer un lien de démarrage classique et un script d'installation pour linux et windows » ; E-24.
- Fichiers (tous neufs, dans `scripts/sketchcraft/`) :
  - `serve.mjs` : serveur local sans dépendance (Node seul) de l'application construite `dist/`, sur 127.0.0.1 seulement, premier port libre à partir de 5180, types MIME du noyau (`application/wasm`) et des modules, retour à `index.html` pour une adresse inconnue, refus des chemins hors du dossier et des requêtes autres que GET/HEAD ; ouvre le navigateur par défaut (`--no-open` pour ne pas l'ouvrir).
  - `install-linux.sh` : vérifie Node ≥ 22, Rust (propose rustup), la cible wasm32, wasm-pack (propose `cargo install`), le compilateur C (dit d'installer `build-essential`) ; `npm ci`, `npm run wasm`, `npm run build` ; crée `~/.local/bin/sketchcraft` (reconstruit si la version git a changé depuis la dernière construction, notée dans `dist/.sketchcraft-build`, puis lance le serveur), l'entrée `sketchcraft.desktop` du menu (terminal visible) et sa copie sur le bureau (marquée de confiance pour GNOME), l'icône ; l'ancien lanceur est gardé sous un nom daté ; `--retirer` retire lanceur, entrées et icône.
  - `package-windows.sh` : construit si besoin, puis crée `sketchcraft-windows-<version>.zip` (dossier `sketchcraft\` : `dist\`, `serve.mjs`, `installer.cmd`, `install-windows.ps1`, `sketchcraft.ico`, `LISEZMOI.txt`, `VERSION.txt`, `EMPREINTES.txt` avec le SHA-256 de chaque fichier) ; un paquet de même nom est gardé sous un nom daté.
  - `installer.cmd` et `install-windows.ps1` : vérifie Node ≥ 22 (propose winget), copie dans `%LOCALAPPDATA%\sketchcraft` (version précédente déplacée dans `sauvegardes\<date>`), vérifie chaque fichier copié par son empreinte, crée `sketchcraft.cmd` (chemin complet de Node) et les raccourcis du Bureau et du menu Démarrer (fenêtre réduite, icône) ; `-Retirer` retire les raccourcis.
  - `LISEZMOI.md`, `LISEZMOI-windows.txt` : modes d'emploi.
- Évolution : sketchcraft se lance d'un clic, comme une application, sous Linux et sous Windows.
- Écarté : compiler le noyau sous Windows (Visual Studio Build Tools, plusieurs Go ; le noyau WebAssembly est identique) ; l'application de bureau native Tauri (.deb, .msi : dépendances système lourdes, à reprendre plus tard si voulu) ; un serveur en arrière-plan sans fenêtre (on ne saurait plus l'arrêter simplement).
- Effet sur les résultats : aucun.
- Effet sur les projets existants : aucun ; les modèles restent dans le navigateur et les fichiers .shz.
- Confidentialité : non ; le serveur n'écoute que sur 127.0.0.1.
- Vérification : dans la copie de travail de Claude, avec le noyau compilé sur jc-ms-7c37 : `npm run build` (contrôles du projet passés, 83 fichiers) ; serveur : page 200, noyau servi en `application/wasm` identique octet par octet, chemins `..` refusés, POST refusé, adresse inconnue → page ; `install-linux.sh` : syntaxe, puis passage complet dans un dossier personnel de test avec des leurres pour npm, Rust et wasm-pack (lanceur, entrée de menu, icône et bureau créés), lanceur : ouverture directe, puis reconstruction après changement de version, `--retirer` ; `package-windows.sh` : paquet de 90 fichiers, empreintes recontrôlées, `installer.cmd` en fins de ligne Windows, serveur lancé depuis le paquet décompressé (page 200, noyau 200). Non vérifié : `install-windows.ps1` (pas de PowerShell ni de Windows ici) ; l'installation réelle sur jc-ms-7c37 et sur le PC Windows : essais par JC à faire.
- Limites connues : icône de Sawhorse (pas encore d'icône sketchcraft) ; le lanceur Linux montre une fenêtre de terminal ; sous Windows, une mise à jour demande un nouveau paquet préparé sous Linux.

## H-20261117-1 · correction · module|documentation · Rebasage sur la version du créateur ; clippy

- Origine : consolidation, étape 1 (M-20261116-12, carnet de reprise du 16 nov. 2026) ; JC, 17 nov. 2026, 05 h 21 : « ok on peut reprendre sketchcraft ».
- Constat : sawhorse3d/sawhorse main avait 8 commits de plus que notre base a7a637d (dont #65 tracé d'une longue esquisse, #66 push/pull minimal, #69 rectangle, #70 bande minimale, #82 et #83 Échelle) ; un conflit, dans `Model::draw_on_plane` (`crates/kernel/src/model.rs`) : la garde de #65 contre un tracé sûr de dépasser la limite d'arêtes, à côté de notre appel `draw_in_sketch_closing` (H-20261116-8). Clippy (lancé pour la première fois) refusait deux fonctions de E-19 placées après le module de tests (`items_after_test_module`).
- Fichiers : les 14 commits de la branche rejoués sur sawhorse3d/sawhorse main `b497e3d` (branche `jc-volumetrie-rebase`) ; conflit résolu en gardant la garde de #65 puis notre appel ; `crates/kernel/src/model.rs` : `straight` et `import_connected` déplacées avant `mod tests` (aucun changement de code).
- Effet sur les résultats : aucun.
- Effet sur les projets existants : aucun.
- Confidentialité : non.
- Vérification : `cargo test --workspace` 743 passés, 4 ignorés, aucun échec ; `cargo clippy --workspace --all-targets -- -D warnings` sans erreur ; `cargo fmt --check` propre ; `tsc -b` propre ; `npx vitest run` 1 165 passés, 2 ignorés, aucun échec (avec le noyau WebAssembly compilé sur jc-ms-7c37 le 16 nov., donc d'avant le rebasage : les tests qui en dépendent tournent désormais) ; `npm run build` et ses contrôles passés. Non fait ici : recompiler le noyau WebAssembly (cible wasm32 non téléchargeable), golden:wasm, boolean:diff, corpus:skp, site, Playwright, fuzz long.
- Limites connues : la branche publiée `jc-volumetrie` (4e503d5) reste l'ancienne base jusqu'à l'envoi de celle-ci.

## H-20261117-2 · évolution · interface · Aire et volume de la sélection (E-10)

- Origine : JC, 17 nov. 2026, 06 h 05 : « ça marche assez bien, il manque par rapport à ce que j'utilise la possibilité d'avoir le volume d'un groupe sélectionné et aussi la surface des faces sélectionnées » ; E-10.
- Fichiers : `src/lib/measure.ts` (nouveau : `partAreas` calcule l'aire de chaque face affichée à partir de ses triangles ; `measureSelection` additionne l'aire des faces et surfaces d'esquisse choisies et le volume des solides choisis, directement ou dans un groupe ou une copie de composant, chaque solide une fois ; `formatSurface` en m² ou en pieds carrés ; `describeMeasure`) ; `src/lib/measure.test.ts` (3 tests) ; `src/App.tsx` (le badge « Selection » de la vue montre ces mesures).
- Évolution : avec une sélection, le badge en haut de la vue indique par exemple « Selection: 3 · 2 faces · 52.30 m² » ou « Selection: 1 · 1 solid · 466.183744 m³ ».
- Écarté : demander les mesures au noyau (il donne déjà le volume de chaque solide, et l'aire se lit sur le maillage affiché : aucun changement du noyau ni du format).
- Effet sur les résultats : affichage seul.
- Effet sur les projets existants : aucun.
- Confidentialité : non.
- Vérification : 3 tests neufs (aire d'un rectangle de 2 × 3 m et d'un triangle ; faces + surface + groupe + solide déjà compté ; formats m² et pieds carrés), qui échouent sans le module ; suite complète : 1 168 passés (1 165 + 3), 2 ignorés, aucun échec. Niveau : testé ; essai par JC à faire.
- Limites connues : le volume affiche les décimales du badge du modèle (jusqu'à 6) ; les maillages importés non solides n'ont pas de volume ; l'aire est celle de la face entière (ouvertures comprises si elles ne sont pas découpées).

## H-20261117-3 · évolution · module · Jonction d'un tracé avec un solide (E-25)

- Origine : JC (17 nov. 2026, 06 h 40) : « les jonctions nouvelle a la partie extrudée (le toit) ne se font pas » ; E-25.
- Constat : la fermeture de E-19 (`draw_in_sketch_closing`, `close_on_other_planes`) ne lisait que les arêtes des esquisses ; les arêtes d'un volume extrudé n'existaient pas pour elle.
- Fichiers : `crates/kernel/src/model.rs` (`solid_segments`, `solid_closers` ; appel dans les deux fermetures) ; `crates/kernel/tests/draw.rs` (3 scénarios).
- Évolution : un tracé ouvert dont les deux extrémités sont sur des arêtes d'un solide du même contexte se ferme en face : par le morceau de l'arête entre les deux extrémités, ou par deux morceaux qui passent par un coin commun.
- Écarté : prendre toutes les arêtes du solide dans le plan (premier essai) : le contour de la face du solide devenait une seconde face d'esquisse superposée (le test l'a montré : 2 régions au lieu d'une).
- Effet sur les résultats : aucun sur les solides existants (étanchéité et volume vérifiés).
- Effet sur les projets existants : aucun (pas de changement de format).
- Confidentialité : non.
- Vérification : 3 scénarios (pignon sur l'arête haute, fermeture par un coin, tracé loin du solide qui ne ferme rien) ; le premier échouait avant ; `cargo test --workspace` 746 réussis, clippy et fmt propres. Niveau : testé (banc : 3 scénarios). Non vérifié ici : le WASM (à construire chez JC) et l'essai à la souris.
- Limites connues : une extrémité au milieu d'une face du solide (pas sur une arête) ne ferme rien ; plus de deux arêtes à suivre ne sont pas cherchées.

## H-20261117-4 · évolution · interface · Grille adaptative et pivot d'orbite près du modèle (E-26, E-27)

- Origine : JC (17 nov. 2026, 06 h 40) : « quand je click sur un point de l'horizon, il l'utilise comme référence pour le déplacement de l'orbite (ce qui n'est pas très pratique). la grille n'est pas adaptive en fonction de l'échelle » ; E-26, E-27.
- Constat : la grille n'avait qu'un pas (celui des unités), masqué quand il devenait trop fin ; le sol sous le curseur était accepté comme pivot jusqu'à 50 fois la taille du modèle, donc presque jusqu'à l'horizon.
- Fichiers : `src/viewport/camera.ts` (`gridLevel`, `GRID_MIN_MAJORS`, `groundPivotOk`) ; `src/viewport/engine.ts` (niveau courant, reconstruction au changement de niveau, `gridLevel()`, `gridState().level`, `navPick`) ; `src/App.tsx` (l'accrochage à la grille suit le niveau affiché) ; `src/viewport/grid-level.test.ts` (6 tests).
- Évolution : la grille change de pas avec le zoom (×10 en métrique ; 1 in/1 ft puis 1 ft/10 ft… en impérial) et s'étend assez pour remplir la vue. L'orbite ne prend un point du sol comme pivot que près du modèle ou du point visé ; sinon elle tourne autour du point visé.
- Écarté : supprimer le pivot au sol (un clic près d'un bâtiment sur le terrain reste utile).
- Effet sur les résultats : aucun sur le modèle. `gridState().spec` reste la grille des unités (le test e2e qui lit son libellé est inchangé).
- Effet sur les projets existants : aucun.
- Confidentialité : non.
- Vérification : 6 tests unitaires (niveaux métriques et impériaux, maille ≥ 6 px, pivot accepté près, refusé vers l'horizon) ; vitest 1174 réussis, `tsc -b` et `npm run build` propres. Niveau : testé (banc : 6 scénarios). Non vérifié ici : le rendu (pas de WebGL2 ici) et les e2e Playwright ; à essayer par JC.
- Limites connues : changement de pas net, sans fondu entre niveaux. E-28 (supprimer une face d'un volume) n'est pas fait : le noyau garde les solides fermés ; voies à décider par JC : (a) « éclater » le volume en faces d'esquisse, puis supprimer la face (on perd le volume calculé) ; (b) supprimer la face et refermer aussitôt par les faces voisines prolongées (comme retirer un pan de toit) ; (c) remplacer la face par un tracé dessus (le toit sur le dessus de la boîte, E-25) et fusionner (E-04).

## H-20261117-5 · évolution · module|interface · Éclater un volume en faces ; refaire un volume de faces fermées (E-28)

- Origine : JC (17 nov. 2026, 06 h 40) : « on ne sais pas supprimer une face d'une extrusion » ; choix de JC, 07 h 11 : « a) Éclater le volume en faces simples, puis supprimer la face […] b) reconstruire un volume sur base d'une construction fermée » ; E-28.
- Constat : le noyau garde tout volume fermé ; retirer une face l'ouvrirait, d'où le refus « erasing that would open the solid ».
- Fichiers : `crates/kernel/src/model.rs` (ops `ExplodeSolid`, `SolidFromFaces`, issue `SolidExploded`, `explode_solid`, `solid_from_faces`, `newell`) ; `crates/kernel/src/error.rs` (`DocumentError::NotAVolume`) ; `crates/kernel/src/sketch.rs` (`inside_rings` visible dans le crate) ; `crates/kernel/tests/explode_solid.rs` (4 scénarios) ; `src/kernel/types.ts` ; `src/App.tsx` (menu Édition : « Explode Solid into Faces », « Make Solid from Faces »).
- Évolution : (a) un volume choisi devient des faces simples, chacune sur l'esquisse de son plan ; une face se retire ensuite par Sélection + Suppr. (b) des faces choisies (Maj+clic) qui ferment un volume redeviennent un solide : coins soudés, côtés coupés aux coins posés dessus, faces retournées pour regarder dehors ; sinon refus avec la raison (arête qui ne joint pas exactement deux faces, coque tordue, plusieurs morceaux, volume nul).
- Écarté : supprimer la face et refermer en prolongeant les voisines (voie b de la proposition du matin), non retenue par JC.
- Effet sur les résultats : le volume éclaté n'a plus de volume calculé (E-10 ne le compte plus) jusqu'à ce qu'il soit refait.
- Effet sur les projets existants : aucun ; deux ops nouvelles dans le journal (un fichier qui les contient ne s'ouvre pas dans un Sawhorse sans elles).
- Confidentialité : non.
- Vérification : 4 scénarios (boîte → 6 faces d'aire totale 10 m² ; 6 faces → volume 2 m³ ; 5 faces → refus, rien de changé ; dessus retiré + toit à deux pans → 9 faces → 2,5 m³ étanche) ; `cargo test --workspace` 750 réussis, clippy et fmt propres ; vitest 1174 réussis, `tsc -b` et `npm run build` propres. Niveau : testé (banc : 4 scénarios). Non vérifié ici : les entrées du menu à la souris, le WASM (à construire chez JC).
- Limites connues : matières, étiquettes, arêtes adoucies et courbes du volume éclaté sont perdues ; il faut choisir toutes les faces à la main (pas encore de « sélectionner les faces connexes ») ; deux faces coplanaires voisines restent deux faces du solide refait.

## H-20261117-6 · correction · module · Faîtage aligné sur un coin survolé (E-29)

- Origine : JC (17 nov. 2026, 07 h 45) : « j'essaye de construire la ligne du faite en utilisant la profondeur de la façade mais au moment de faire la jonction avec le trais vert il perd le croisement » (3 captures) ; E-29.
- Constat : « From Point » ne cherchait que le croisement exact entre la ligne d'axe du départ et une ligne d'axe passant par le point survolé. Le coin arrière du mur est plus bas que le sommet du pignon : ces deux droites ne se coupent pas dans l'espace, donc aucun croisement, et seule « On Green Axis » restait.
- Fichiers : `crates/inference/src/infer.rs` (le point de la ligne du départ à l'aplomb du point survolé, c'est-à-dire au même niveau le long de cette ligne) ; `crates/inference/tests/infer.rs` (1 scénario).
- Correction : le long d'une ligne d'axe (ou parallèle, perpendiculaire) depuis le départ, le point au niveau d'un point survolé est proposé « From Point », avec les deux lignes pointillées.
- Écarté : projeter le point survolé sur le plan du sommet seulement (ne couvrait pas les lignes parallèles ou perpendiculaires).
- Effet sur les résultats, les projets : aucun.
- Confidentialité : non.
- Vérification : 1 scénario, qui échouait avant (« On Green Axis ») et passe ; `cargo test --workspace` 751 réussis ; clippy et fmt propres. Niveau : testé (banc : 1 scénario). Non vérifié ici : à la souris (le WASM est à reconstruire chez JC).
- Limites connues : il faut avoir survolé le coin pendant le tracé, comme dans SketchUp.

## H-20261117-7 · évolution · module|interface · Extruder ensemble des faces le long d'un axe (E-30)

- Origine : JC (17 nov. 2026, 08 h 13) : « il faudrait une fonction extruder ensemble sur l'axe ( ici je veux extruder l'ensemble de 30 cm sur l'axe vertical pour simuler la toiture, mais l'extrusion actuelle fonctionne uniquement sur la normale d'une face unique ) » ; 08 h 19 : « en fait il faut conserver les deux , c'est une autre façon de procéder » ; E-30.
- Fichiers : `crates/kernel/src/model.rs` (op `ExtrudeFaces` ; `extrude_faces` ; `face_rings`, `weld_rings`, `solid_from_rings`, `not_a_volume` tirés de `solid_from_faces`, sans changer son comportement) ; `crates/kernel/tests/explode_solid.rs` (2 scénarios) ; `src/lib/extrudeAlong.ts` et son test (3) ; `src/kernel/types.ts` ; `src/App.tsx` (Édition > Extrude Faces Along Axis…).
- Évolution : les faces choisies sont extrudées ensemble le long d'un vecteur en un seul volume : les faces, les faces déplacées, et un côté le long de chaque arête du pourtour (l'arête commune, comme le faîtage, n'en a pas). Les faces restent (elles servent encore à fermer la maison). Le Pousser/Tirer par la normale d'une face est inchangé : les deux façons coexistent.
- Écarté : ôter les faces comme le fait l'extrusion d'une région (elles manqueraient ensuite pour refaire le volume de la maison).
- Effet sur les résultats, les projets : aucun sur l'existant ; une op nouvelle dans le journal.
- Confidentialité : non.
- Vérification : noyau, 2 scénarios (deux pans → 0,6 m³ étanche, faces gardées ; direction dans le plan des faces → refus, rien de créé) ; interface, 3 tests de lecture de la valeur ; `cargo test --workspace` 753, vitest 1177 réussis ; clippy, fmt, `tsc -b`, `npm run build` propres. Niveau : testé (banc : 5 scénarios). Non vérifié ici : à la souris, le WASM (à construire chez JC).
- Limites connues : des faces disjointes (qui ne se touchent pas) sont refusées (« more than one piece ») ; extrudez-les séparément. Pas encore d'aperçu pendant la saisie ni de sens donné à la souris.

## H-20261117-8 · évolution · interface · Outil « Push ax. » : pousser ensemble le long d'un axe (E-31)

- Origine : JC (17 nov. 2026, 09 h 01) : « ca ne va pas comme ca , il faut rajouter un bouton pour ce push sous l'autre et ajouter aussi la contrainte de l'axe directement dans la fenêtre avec le méme principe que l'accrochage classique » ; E-31 (suite de E-30).
- Fichiers : `src/tools/pushaxis.ts` (outil) et `src/tools/pushaxis.test.ts` (3 tests) ; `src/lib/tools.ts` (entrée sous Push, sans touche) ; `src/components/glyphs.tsx` (icône) ; `src/App.tsx` (outil enregistré) ; `src/App.test.tsx` (20 outils).
- Évolution : un appui sur une face d'esquisse puis un mouvement : cette face, ou toutes les faces sélectionnées si elle en fait partie, montent ensemble le long du bleu, avec l'aperçu des contours et la distance en direct. L'axe est une ligne d'inférence verrouillée : le curseur s'accroche aux points du modèle projetés dessus (comme Push/Pull le long de la normale). Flèches : droite rouge, gauche vert, haut bleu ; bas ou Maj : la parallèle ou perpendiculaire montrée. Glisser-lâcher, clic-bouger-clic, ou valeur tapée. L'op est celle de E-30 (`extrude_faces`) ; le noyau n'a pas changé.
- Écarté : la fenêtre de saisie seule (E-30), jugée peu pratique par JC ; elle reste dans le menu Édition.
- Effet sur les résultats, les projets : aucun.
- Confidentialité : non.
- Vérification : 3 tests (deux faces sélectionnées → `extrude_faces` des deux vers le haut de 0,30 ; face hors sélection seule, flèche gauche → vert, valeur tapée 50 cm ; face d'un solide → renvoi vers Push/Pull) ; test de la barre mis à 20 outils ; vitest 1180 réussis, `tsc -b` et `npm run build` propres. Niveau : testé (banc : 3 scénarios). Non vérifié ici : à la souris (pas de WebGL ici).
- Limites connues : les faces d'un solide ne sont pas prises (Push/Pull, ou éclater d'abord) ; un double-clic ne répète pas encore la dernière distance.

## H-20261117-9 · évolution · interface · Vues : Left sur la barre, Persp et Iso en premier (E-32)

- Origine : JC (17 nov. 2026, 09 h 26) : « dans les vues il manque "left" , je placerai perspective et isométrique en premier dans la liste c'est plus logique » ; E-32.
- Fichiers : `src/viewport/camera.ts` (ordre de `VIEWS`, Left n'est plus réservée au menu) ; `src/viewport/camera.test.ts`, `src/App.test.tsx` (ordre attendu).
- Vérification : vitest 1180 réussis, `tsc -b` et build propres. Les e2e Playwright cliquent les boutons par leur nom (pas d'effet attendu), non lancés ici. La vue au démarrage reste Iso.

## H-20261117-10 · évolution · interface|documentation · Le programme s'appelle VolumeCraft (E-33)

- Origine : JC (17 nov. 2026, 09 h 51) : « changer le nom du programme volumecraft , archicraft , Geocraft, Alphasketch .... » ; 09 h 52, après l'avis de Claude : « VolumeCraft donc seulement le programme » ; E-33.
- Fichiers : `src/lib/brand.ts` (nouveau : `APP_NAME`, `APP_CREDIT`) ; `index.html` (titre, description) ; `src/components/TopBar.tsx` (logo ; info-bulle « VolumeCraft, built on Sawhorse (MIT OR Apache-2.0) ») ; `src/App.tsx` (titre de fenêtre, fichier non ouvrable, Fermer) ; `src/lib/platform.ts` (Quit sur macOS) ; `src/io/closing.ts`, `src/io/files.ts` (question avant de perdre les changements) ; `src-tauri/tauri.conf.json` (productName, titre, description ; copyright inchangé) ; `scripts/sketchcraft/install-linux.sh` (Name=VolumeCraft), `install-windows.ps1` (description du raccourci) ; tests vitest et e2e (titres attendus).
- Évolution : le nom affiché devient VolumeCraft. Restent « Sawhorse » : le dépôt et le fork GitHub, le format .shz (« Sawhorse document »), les extensions, le style « Sawhorse », les notices de licence et de copyright, et des messages techniques (limites de taille des fichiers, service de rendu, notes d'import .skp, PDF du débit), à reprendre si JC le souhaite.
- Écarté : AlphaSketch (proche des marques SketchUp et Sketch), ArchiCraft (galvaudé selon JC, proche d'ArchiCAD), GeoCraft (préfixe très pris en géologie et SIG). Recherche de marque faite rapidement seulement : à vérifier (EUIPO) avant toute diffusion publique sous ce nom.
- Effet sur les résultats, les projets : aucun ; les fichiers .shz s'ouvrent comme avant. L'app de bureau construite aura le nom VolumeCraft (chemin d'installation Tauri différent).
- Confidentialité : non.
- Vérification : vitest 1180 réussis, `tsc -b` et build propres ; e2e mis à jour mais non lancés ici. Lanceur existant : le nom affiché change après réinstallation (`install-linux.sh`), la commande reste `sketchcraft`.

## H-20261117-11 · évolution · interface · Tablette Android : application installable et touches à l'écran (E-34, première partie)

- Origine : JC (17 nov. 2026, 10 h 40) : « il faudrait une version compatible android » ; 10 h 42 : « android tablette avec pen ou souris » ; E-34.
- Constat : le stylet et la souris arrivent déjà aux outils comme une souris (seul le doigt est pris par la navigation : un doigt tourne, deux déplacent et zooment). Manquaient : les touches (Esc, Entrée, flèches d'axe, Maj, Ctrl+Z), l'installation et l'usage sans réseau.
- Fichiers : `src/components/KeyBar.tsx` (barre de touches : chaque bouton envoie la touche qu'enverrait un clavier) et son test ; `src/App.tsx` (Affichage > On-screen Keys, gardé par visiteur, mis tout seul au premier appui d'un stylet ou d'un doigt ; dépendances du menu complétées avec les fonctions de E-28 et E-30) ; `src/styles/app.css` ; `public/manifest.webmanifest`, `public/sw.js`, `public/icons/` (copies des icônes de `src-tauri/icons`) ; `index.html` (manifeste, couleur) ; `src/main.tsx` (service worker enregistré seulement par la version construite, en contexte sécurisé, hors application de bureau).
- Évolution : sur tablette, une barre Esc, ↵, X (rouge), Y (vert), Z (bleu), ∥, ⇧ (maintenu jusqu'au prochain appui), ↶ (annuler). L'application web devient installable et utilisable hors ligne une fois servie en HTTPS.
- Écarté : une application Android (APK, Tauri) : kit Android à installer chez JC, rien de plus pour l'usage visé. Un mode tactile complet (dessin au doigt, loupe) : non demandé (stylet ou souris).
- Effet sur les résultats, les projets : aucun. Le serveur de développement et l'application de bureau n'enregistrent pas de service worker.
- Confidentialité : non (rien ne quitte l'appareil ; le service worker ne met en cache que l'application elle-même).
- Vérification : 1 test (les 9 boutons envoient les bonnes touches, Maj enfoncée puis relâchée, fermeture) ; vitest 1181 réussis ; `tsc -b`, build et contrôles de provenance propres ; `dist` contient le manifeste, `sw.js` et les icônes. Niveau : testé (banc : 1 scénario). Non vérifié : sur une vraie tablette (installation, hors ligne, stylet, survol du stylet pour l'accrochage), le rendu de la barre.
- Limites connues : il faut un hébergement HTTPS pour installer (à décider avec JC) ; l'accrochage par survol dépend d'un stylet qui signale le survol (S Pen, par exemple) ; sans survol, les points de référence ne se mémorisent pas.

## H-20261117-12 · évolution · interface|documentation · Projet Android (APK) avec Capacitor (E-35)

- Origine : JC (17 nov. 2026, 12 h 14) : « tu peux faire un apk à instaler » ; E-35 (suite de E-34).
- Constat : l'APK ne peut pas être construit dans l'environnement de Claude : le réseau y refuse dl.google.com (kit Android) et services.gradle.org (Gradle), comme static.rust-lang.org. Il se construit donc sur la machine Linux de JC.
- Fichiers : `mobile/` (nouveau : `package.json` avec Capacitor 8.6.2, `capacitor.config.json` : appId `org.volumecraft.app`, nom VolumeCraft, application web prise dans `../dist` ; `android/` : projet Gradle produit par `cap add android`, icônes remplacées par celle du programme ; `.gitignore`) ; `scripts/sketchcraft/build-apk.sh` (vérifie Java 21 et le kit Android, construit noyau et application web, synchronise, `gradlew assembleDebug`, range l'APK sous un nom daté sans rien écraser) ; `scripts/sketchcraft/LISEZMOI.md` (section Tablette Android) ; `src/main.tsx` (pas de service worker dans l'application Android, qui emporte ses fichiers).
- Évolution : une application Android qui contient tout VolumeCraft (WebView : WebGL 2 et WASM), hors ligne, sans hébergement ni publication. Les touches à l'écran de E-34 y servent.
- Écarté : Tauri pour Android (cibles Rust Android et NDK en plus, rien de plus pour l'usage) ; une application web de confiance (TWA) ou une PWA hébergée (elles demandent une publication HTTPS).
- Effet sur les résultats, les projets : aucun. Les fichiers .shz s'ouvrent et s'enregistrent par les boîtes de dialogue du navigateur intégré (non éprouvé sur Android).
- Confidentialité : non (rien ne quitte la tablette).
- Vérification : `cap add android` réussi ici ; syntaxe du script vérifiée (`bash -n`) ; vitest 1181 réussis, `tsc -b` et build propres. Non vérifié : la construction Gradle et l'APK (kit Android injoignable ici), l'installation et l'usage sur la tablette ; à faire par JC.
- Limites connues : APK de débogage (signé avec la clé de débogage : installation manuelle, pas pour le Play Store) ; ouvrir et enregistrer des fichiers sur Android reste à éprouver.

## H-20261117-13 · correction · documentation · build-apk.sh : Gradle sous Java 21 (E-35)

- Origine : JC (17 nov. 2026, 13 h 25), sortie du script : « BUG! exception in phase 'semantic analysis' … Unsupported class file major version 69 » ; E-35.
- Constat : la version 69 est Java 25, celui fourni avec Android Studio (`~/android-studio/jbr`), mis en tête du PATH sur ma consigne de 12 h 26. Gradle 8.14.3 (celui du projet Capacitor) ne tourne qu'avec Java 21 à 24. Le script acceptait tout Java ≥ 21. Ma consigne de se passer d'`openjdk-21-jdk` était fausse.
- Fichiers : `scripts/sketchcraft/build-apk.sh` (cherche un Java 21 à 24 : `JAVA_HOME`, `/usr/lib/jvm/*21*`, 22 à 24, le Java d'Android Studio, celui du PATH ; le donne à Gradle par `JAVA_HOME` ; sinon dit d'installer `openjdk-21-jdk`) ; `scripts/sketchcraft/LISEZMOI.md`.
- Écarté : passer à Gradle 9 (accepte Java 25) : le greffon Android du projet (AGP 8.13) n'y est pas prévu.
- Vérification : `bash -n` ; la recherche du Java, jouée ici, trouve `/usr/lib/jvm/java-21-openjdk-amd64` (version 21). Non vérifié : la construction complète chez JC.

## H-20261117-14 · évolution · module|interface · Consolidation : cycle complet testé ; union de solides ; volume des faces simples ; faces connexes (E-04, E-36, E-37)

- Origine : JC (17 nov. 2026, 16 h 41) : « pour le moment pas encore de hvac on va faire 1 et 2 » (1 : consolider ; 2 : volume des faces fermées, faces connexes, union de solides) ; E-04, E-36, E-37.
- Fichiers : `crates/kernel/src/model.rs` (op `UnionSolids` ; `closed_faces_volume`, `connected_faces`, `region_world_rings` en lecture seule) ; `crates/kernel/tests/volumetrie_cycle.rs` (nouveau) ; `crates/wasm-api/src/lib.rs` (`faces_query_json`, `faces_query`) ; `src/kernel/types.ts`, `kernel.worker.ts`, `client.ts` (requête `facesQuery`) ; `src/lib/measure.ts` (« enclosing … m³ ») ; `src/tools/tool.ts`, `src/tools/select.ts` (double-clic sur une face simple) ; `src/App.tsx` (volume demandé au noyau quand 4 faces simples ou plus sont choisies ; Édition > Join Solids ; hôte `connectedFaces`) ; tests `select.test.ts`, `measure.test.ts`.
- Évolution : le cycle de la volumétrie (boîte, éclatement, dessus retiré, toit à deux pans, dalle de toiture, maison refaite, union) est éprouvé avec annuler puis rétablir et enregistrer puis rouvrir à chaque étape ; des solides qui se touchent ou se recouvrent deviennent un seul (Édition > Join Solids) ; des faces simples qui ferment une coque disent leur volume ; un double-clic sur une face simple prend toute la coque.
- Écarté : calculer le volume des faces dans l'interface (orientation des faces à refaire en double du noyau).
- Effet sur les résultats, les projets : aucun sur l'existant ; une op nouvelle dans le journal.
- Confidentialité : non.
- Vérification : noyau, 2 scénarios (le cycle entier : 13 étapes, chacune annulée, rétablie, enregistrée et rouverte à l'identique ; union refusée de solides éloignés, modèle inchangé) ; interface, 2 tests (double-clic → faces connexes ; texte du volume). Niveau : testé (banc : 4 scénarios). Non vérifié : à la souris ; le WASM est à reconstruire chez JC (`npm run wasm`), sans quoi le volume des faces ne s'affiche pas (l'interface se tait alors).
- Limites connues : voir E-38.

## H-20261117-15 · correction · module|interface · Constats de la relecture à froid (agent neutre)

- Origine : relecture à froid demandée dans le point 1 retenu par JC (17 nov. 2026, 16 h 41) ; 13 constats (aucun grave), 7 traités ici, 6 notés en E-38.
- Corrections et leurs tests :
  - constat 2 : `ExtrudeFaces` refusait des faces disjointes alors que l'op les annonçait : un solide par groupe de faces qui se touchent (`face_groups`) ; test `faces_apart_extrude_together_into_one_solid_each` (échouait avant).
  - constat 3 : la touche Maj à l'écran n'atteignait pas les clics du stylet (Maj+clic n'ajoutait rien) : `virtualShift()` ajouté aux événements de la vue ; Maj relâchée quand la barre disparaît ; test KeyBar (échouait avant).
  - constat 4 : seuil de volume pris sur une tolérance de longueur (en dessous d'environ 1 cm³, « pas de volume ») : seuil PLANARITY³ ; test `small_volumes_are_volumes` (échouait avant).
  - constat 5 : soudure des sommets en temps quadratique : grille de cellules ; coupure aux coins limitée à la plage en x de chaque côté. Comportement identique (tests existants).
  - constat 7 : mesure de la sélection recalculée à chaque rendu : `useMemo`.
  - constat 9 : service worker : cache jamais renouvelé ni purgé, page d'erreur gardée hors ligne, requêtes du service de rendu servies du cache : `sw.js` est maintenant écrit par la construction (`src/lib/swSource.ts`, greffon Vite) avec la liste exacte des fichiers et une version tirée de cette liste ; l'ancien cache est effacé ; seuls ces fichiers sont gardés ; la page n'est gardée que si elle est revenue entière. `public/sw.js` retiré (remplacé par le fichier construit) ; 2 tests.
  - constat 11 : union de solides qui ne se touchent que par une arête : message clair au lieu de l'erreur brute du moteur booléen ; test (échouait avant).
- Fichiers : `crates/kernel/src/model.rs`, `crates/kernel/tests/volumetrie_cycle.rs`, `src/components/KeyBar.tsx` et son test, `src/components/Viewport.tsx`, `src/App.tsx`, `src/lib/swSource.ts` et son test, `vite.config.ts`, `public/sw.js` (retiré).
- Vérification : `cargo test --workspace` 758 réussis, clippy et fmt propres ; vitest 1186 réussis ; `tsc -b` et build propres (`dist/sw.js` écrit avec 87 fichiers). Niveau : testé (banc : 6 scénarios ajoutés). Non vérifié : les e2e Playwright (à lancer chez JC), le service worker sur une vraie tablette.

## H-20261117-16 · évolution · interface · Import COLLADA (.dae) (E-39)

- Origine : JC (17 nov. 2026, 20 h 37) : « pour l'export actuel de geohvac c'est du dae mais volumecraft ne le supporte pas » (capture du sélecteur de fichiers sans .dae) ; E-39.
- Fichiers : `src/io/dae.ts` (nouveau : lecture par le lecteur COLLADA de three.js, unités et axe vertical du fichier appliqués, maillages ramenés en coordonnées du monde avec une couleur unie, écrits en .glb par l'exporteur glTF de three.js, sans charger aucune image) et son test ; `src/io/files.ts` (type COLLADA dans Import et Ouvrir) ; `src/App.tsx` (un .dae passe au lecteur glTF du noyau sous le nom `<nom>.glb`, avec une note dans le compte rendu ; chargé seulement au premier .dae ; libellés du menu, du glisser-déposer et du refus) ; `src-tauri/start-file/src/lib.rs` (« dae » dans la liste du programme de bureau, gardée égale à celle de la page par `startup.test.ts`).
- Écarté : un lecteur COLLADA dans le noyau Rust (plus long ; le lecteur glTF existant fait déjà la suite : axe, unités, maillages fermés en solides).
- Effet sur les projets : aucun ; le noyau n'a pas changé (pas de `npm run wasm` pour cette entrée).
- Confidentialité : non (lecture locale ; aucune image ni adresse du fichier n'est chargée).
- Vérification : 2 tests (une boîte de 2 × 1 × 1 m écrite en centimètres, Z vers le haut, devient un .glb de 2 × 1 × 1 m, Y vers le haut, avec la note « 1 mesh, 12 triangles » ; un fichier qui n'est pas du COLLADA est refusé) ; vitest 1188 réussis ; tests du lanceur de bureau (25) ; build propre (lecteur dans un morceau à part de 92 kB). Niveau : testé (banc : 2 scénarios). Non vérifié : un vrai export de GeoHVAC ; l'import complet par le noyau (WASM).
- Limites connues : textures laissées de côté (couleur unie) ; les maillages non fermés restent des maillages importés (non solides), comme pour le glTF.

## H-20261117-17 · évolution · module|interface · Unify Imported Meshes : un bâtiment importé refait en solide à partir de son toit (E-40)

- Origine : JC (17 nov. 2026, 21 h 20) : « c'est importé mais inutilisable , pas moyen de convertir en solide ou faire quelque chose » ; 21 h 21 : « il faudrait une fonction unify import » ; 21 h 22 : le fichier `geohvac-batiment-5719e678.dae` joint ; E-40.
- Constat (analyse du fichier, avec l'accord de JC qui l'a joint) : 38 points, 47 triangles (toit 14, murs 27, plancher 6). Le toit est fait de morceaux décalés de 1 à 2 cm (coins presque confondus) avec de petits murs de raccord ; le plancher n'est pas cousu aux murs (des coins de murs tombent au milieu de ses bords) et n'est pas plat (−0,15 à +0,10 m). Une réparation générale (soudure jusqu'à 15 cm, coupure aux coins) laisse encore des arêtes à trois faces : il n'y a pas de coque à réparer, il faut la refaire.
- Fichiers : `crates/kernel/src/model.rs` (op `UnifyMesh`, issue `MeshUnified`, `unify_25d`, `UNIFY_GAP` = 3 cm) ; `crates/kernel/src/meshbuild.rs` (`weld_points`, accès à la soudure existante) ; `crates/kernel/tests/unify_mesh.rs` (2 scénarios) ; `src/kernel/types.ts` ; `src/App.tsx` (Édition > Unify Imported Meshes : chaque maillage importé, une étape d'annulation chacun, compte rendu de ce que chacun est devenu ou pourquoi pas).
- Évolution : le bâtiment est refait à partir de son toit (faces inclinées de plus de 12° environ, au-dessus de la bande du plancher) : morceaux soudés à 3 cm près et coins ramenés à une même position en plan ; un mur vertical sous chaque bord du contour du toit, jusqu'au plancher ; là où deux bords du toit se superposent en plan (marche entre une partie haute et une basse), un mur de marche entre eux ; un plancher plat au pied des murs (médiane des points du demi-mètre le plus bas). Les murs et le plancher du fichier ne sont pas repris.
- Écarté : réparer la coque telle quelle (impossible sur ce fichier, voir Constat) ; combiner des prismes par booléens (plus fragile).
- Effet sur les résultats : le volume est celui du bâtiment refait (murs verticaux, plancher plat) ; le toit est celui du fichier à 3 cm près.
- Effet sur les projets : aucun ; une op nouvelle dans le journal.
- Confidentialité : le fichier de JC a été lu localement pour l'analyse et un essai hors du dépôt (`~/scratch`, copie dérivée effacée) ; rien de lui n'est dans le code ni dans les tests (le test du dépôt est un bâtiment fabriqué).
- Vérification : 2 scénarios (bâtiment fabriqué aux mêmes défauts : partie haute en deux morceaux décalés de 2 cm, partie basse, marche de 2,7 m, mur mal cousu, pas de plancher → un solide étanche de 148,8 m³ exactement ; maillage sans toit → refusé, gardé) ; essai local sur le fichier de JC : un solide étanche de 914,45 m³, 26 faces, 13 faces de toit, 1 mur de marche, plancher à −0,03 m. `cargo test --workspace` 760 réussis, clippy et fmt propres ; vitest 1188 réussis ; build propre. Niveau : testé (banc : 2 scénarios) et essayé sur le fichier réel hors de l’application. Non vérifié : dans l'application (WASM à reconstruire chez JC).
- Limites connues : seulement des bâtiments 2,5D (murs verticaux, un toit au-dessus de chaque point du plan) ; porte-à-faux, balcons, toits qui se recouvrent en plan sont refusés ou simplifiés ; les faces du toit restent des triangles (non regroupées en pans).

## H-20261117-18 · évolution · module|interface · Merge Faces : faces presque coplanaires fusionnées (E-41)

- Origine : JC (17 nov. 2026, 22 h 28) : « c'est parfait mais on ne sait rien faire , pas supprimer les segments inutiles , sélectionner les points , supprimer des faces , rien » (capture du bâtiment unifié : 26 faces, toit en triangles) ; E-41.
- Constat : sur les trois demandes, deux existaient déjà sans être visibles : un sommet se sélectionne avec Select (un clic sur un coin, à 8 pixels près) puis se déplace avec Move (les faces voisines suivent, Alt plie celles qui casseraient) ; une face d'un solide se retire par Édition > Explode Solid into Faces puis Sélection + Suppr (E-28). Le manque réel : les diagonales des triangles du toit séparent des faces à 1 ou 2 cm d'un même plan ; la Gomme les refuse (le solide plierait) ; Ctrl+Gomme ne fait que les adoucir (cachées, faces toujours distinctes).
- Fichiers : `crates/kernel/src/model.rs` (op `MergeFaces`, `merge_faces`, `MERGE_TOLERANCE` = 5 cm ; issue `FacesMerged` existante ; plancher à z = 0 pour un toit seul dans `unify_25d`) ; `crates/kernel/tests/unify_mesh.rs` (2 scénarios) ; `src/kernel/types.ts` ; `src/App.tsx` (Édition > Merge Faces).
- Évolution : des faces d'un même solide, voisines et à 5 cm près d'un même plan, deviennent une seule face plane : leurs coins vont sur le plan qui leur va le mieux (à la verticale, sauf pour un plan vertical), les arêtes entre elles disparaissent. Si une face voisine devait plier, refus avec la raison (la fusionner aussi, ou la couper d'abord) ; rien ne change alors.
- Écarté : regrouper automatiquement les pans dans Unify (des coins partagés par plusieurs pans ne peuvent pas être sur tous leurs plans à la fois sans les déplacer aussi en plan : à reprendre plus tard).
- Effet sur les résultats : les coins bougent de 5 cm au plus ; le volume change d'autant.
- Confidentialité : non.
- Vérification : 2 scénarios (bloc 4 × 4 m au toit en deux triangles dont un coin est 2 cm plus haut, unifié puis toit fusionné : une face de moins, étanche, 16 × 3,005 m³ ; faces d'une boîte à angle droit : refus, modèle inchangé) ; un scénario de E-40 couvre le toit seul (plancher à 0). `cargo test --workspace` 762 réussis, clippy et fmt propres ; vitest 1188 réussis ; build propre. Niveau : testé (banc : 2 scénarios). Non vérifié : sur le bâtiment de JC dans l’application.

## H-20261117-19 · évolution · module|interface · Fusion automatique des faces presque coplanaires (E-42)

- Origine : JC (17 nov. 2026, 23 h 19) : « il faudrait un automerge pour cela » (après Merge Faces, H-20261117-18) ; E-42.
- Fichiers : `crates/kernel/src/model.rs` (op `AutoMergeFaces`, issue `AutoMerged` ; `merge_faces` réécrit autour de la fonction `merged_object`, sans changement de comportement ; `auto_merged` ; `unify_25d` l'appelle en fin de reconstruction et le dit dans ses notes) ; `crates/kernel/tests/unify_mesh.rs` (test du toit en deux triangles réécrit : Unify le fusionne de lui-même ; 2 scénarios ajoutés sur une boîte dont le dessus est coupé en deux) ; `src/kernel/types.ts` ; `src/App.tsx` (Édition > Auto Merge Faces, sur le ou les solides choisis, une étape d'annulation).
- Évolution : à partir de chaque face (la plus grande d'abord), ses voisines par une arête la rejoignent tant qu'elles regardent à moins de 5° d'elle et que tous leurs coins restent à 5 cm d'un même plan ; chaque groupe de deux faces ou plus est fusionné comme par Merge Faces ; un groupe qui ferait plier une face voisine est laissé ; on recommence jusqu'à ce que plus rien ne fusionne. Unify Imported Meshes le fait de lui-même.
- Effet sur les résultats : coins déplacés de 5 cm au plus ; volume changé d'autant (bâtiment 5719E678 : 914,45 → 914,49 m³).
- Confidentialité : non (essai local sur le fichier de JC, copie dérivée effacée ensuite).
- Vérification : 3 scénarios (toit en deux triangles décalés de 2 cm : une face de toit après Unify, 6 faces, étanche ; dessus coupé en deux fusionné par Merge Faces ; trouvé seul par Auto Merge, rien d'autre ne change, un second passage ne change rien) ; essai local du fichier de JC : 26 → 20 faces, étanche. `cargo test --workspace` 764 réussis, clippy et fmt propres ; vitest 1188 réussis ; build propre. Niveau : testé (banc : 3 scénarios). Non vérifié : dans l’application chez JC.
- Limites connues : seuils fixes (5°, 5 cm) ; des pans séparés par plus de 5° restent distincts (c'est voulu : ce sont de vrais pans).

## H-20261118-1 · évolution · interface · Align to Axes : un bâtiment de travers aligné sur le rouge et le vert (E-43)

- Origine : JC (18 nov. 2026, 10 h 24) : « je ne sais pas comment aligner automatiquement le batiment importé dans la vue de volume craft » (capture : le bâtiment unifié de travers, outil Rotation ouvert) ; E-43.
- Fichiers : `src/lib/align.ts` (nouveau : arêtes des solides choisis dans le maillage affiché ; direction des murs par vote des arêtes horizontales, chacune selon sa longueur, sur 4 × leur angle, donc à 90° près ; centre du plan) et son test ; `src/App.tsx` (Édition > Align to Axes : les solides choisis, ou tous si rien n'est choisi, tournés autour du bleu par le centre de leur plan, une étape d'annulation ; le message donne l'angle).
- Évolution : une commande au lieu de chercher l'angle à la main avec l'outil Rotation. Le noyau n'a pas changé (op `transform` existante).
- Écarté : tourner les axes de dessin plutôt que le bâtiment (E-02, pas encore fait) ; ce serait mieux pour garder le nord : à reprendre.
- Effet sur les résultats : le bâtiment perd son orientation réelle de l'angle annoncé (utile plus tard pour le soleil et GeoHVAC) ; Annuler la lui rend.
- Confidentialité : non (l'angle de 6° du bâtiment de JC a été estimé localement depuis le fichier joint).
- Vérification : 3 tests (boîte tournée de 0, 17, −23, 40, 72, 110° : angle retrouvé à 90° près ; un petit mur de travers ne compte presque pas ; sans arête horizontale, rien ; centre du plan) ; vitest et build propres. Niveau : testé (banc : 3 scénarios). Non vérifié : dans l'application.

## H-20261118-2 · correction · module · Unify : couture du toit à 5 cm (E-40)

- Origine : JC (18 nov. 2026, 11 h 19) : « il ne veut pas unifier » (fichier `geohvac-batiment-a2cdd7e9.dae` joint) ; E-40.
- Constat : le refus était « the rebuilt faces do not close a volume ». Deux morceaux du toit se rejoignent à un coin écarté de 5,59 − 5,56 m en plan, soit un rien plus de 3 cm en virgule flottante (0,030000000000000027) : la couture à 3 cm ne les joignait pas, les deux morceaux se croisaient en plan.
- Fichiers : `crates/kernel/src/model.rs` (`UNIFY_GAP` 3 → 5 cm) ; `crates/kernel/tests/unify_mesh.rs` (1 scénario : un coin décalé d'un rien plus de 3 cm en travers ; il échoue à 3 cm avec le même message, passe à 5 cm).
- Effet sur les résultats : des points de toit à moins de 5 cm sont confondus (sans effet sensible sur une volumétrie).
- Vérification : le scénario ; essai local du fichier de JC : un solide étanche de 581,40 m³ (le volume sous les triangles du toit du fichier vaut 581,6 m³ ; plancher 176,6 m²), 25 faces après fusion automatique ; `cargo test --workspace` 765 réussis, clippy et fmt propres. Niveau : testé (banc : 1 scénario) et essayé sur le fichier réel hors de l'application.
- Suite : la cause est à l'export de GeoHVAC (morceaux non cousus) ; note explicative demandée par JC pour « Profondeur Alpha » (11 h 30).

## H-20261118-3 · évolution · interface · Le modèle revient de lui-même après un rechargement de la page (E-44)

- Origine : JC (18 nov. 2026, 12 h 46) : « si je met à jour la page je perds tout le modele chargé » ; E-44.
- Constat : le modèle n'était pas perdu. Sawhorse le garde dans le navigateur après chaque modification (sauvegarde automatique) et, au chargement, propose dans la barre d'état « Restore unsaved work? » avec un bouton Restore. Le message passe facilement inaperçu, et sans clic le modèle reste de côté.
- Fichiers : `src/lib/autosave.ts` (`pageWasReloaded` : l'entrée de navigation du navigateur dit « reload ») et son test ; `src/App.tsx` (après un rechargement, la sauvegarde utilisable est restaurée d'elle-même, une fois ; sinon la question reste).
- Écarté : restaurer aussi à chaque nouvelle ouverture (la question garde son sens quand on revient des jours plus tard, ou quand un fichier est ouvert au démarrage).
- Effet sur les projets : aucun ; le vrai enregistrement reste File > Save (.shz).
- Confidentialité : non (tout reste dans le navigateur).
- Vérification : 1 test (rechargement distingué d'une ouverture) ; vitest 1192 réussis, `tsc -b` et build propres. Niveau : testé (banc : 1 scénario). Non vérifié : un vrai rechargement dans Firefox chez JC.

## H-20261118-4 · évolution · interface · Cote alignée sur la face source ou d'équerre à elle (E-45)

- Origine : JC (18 nov. 2026, 13 h 55) : « pour la cotation possibilité d'aligner ou perpendiculaire a une des faces sources » (capture : une cote de 2,6983 m posée de travers sous un bâtiment, export natif .shz de GeoHVAC ouvert) ; E-45.
- Constat : le décalage de la ligne de cote était la perpendiculaire de la cote au point visé, dans n'importe quelle direction ; un point visé sur le sol faisait partir la cote de biais.
- Fichiers : `src/tools/tape.ts` (`offsetWays`, `snappedOffset` ; l'outil Cotation retient la normale des faces où ses points ont été pris) ; `src/tools/tape.test.ts` (1 scénario).
- Évolution : le décalage s'accroche, à 20° près, à la direction dans le plan de la face source (la cote est posée sur la face, à côté de l'arête mesurée) ou d'équerre à elle (la cote sort de la face), puis au rouge, au vert ou au bleu ; la barre d'état dit laquelle. Hors de ces directions, il reste libre comme avant.
- Effet sur les projets : aucun (les cotes existantes ne changent pas).
- Vérification : 1 scénario (cote sur un pan de toit incliné : dans le plan du pan, puis d'équerre, chacun reconnu et exact) ; le test existant (décalage le long du vert) passe toujours ; vitest et build propres. Niveau : testé (banc : 1 scénario). Non vérifié : à la souris.
- Limites connues : pour une arête cliquée, seule la face visée au clic est connue (pas sa voisine de l'autre côté de l'arête) ; pas encore de verrou par flèche.

## H-20261118-5 · rectification · interface · H-20261118-4 : le test cassait la compilation

- Origine : constat de Claude juste après H-20261118-4 : `npm run build` (qui compile aussi les tests, `tsc -b`) refusait `snapKind: "endpoint"` (une chaîne au lieu du type `InferenceKind`) ; vitest, lui, passait. Le commit 60e4aeb a été poussé avant la vérification complète : le fork n'a pas compilé pendant quelques minutes.
- Fichiers : `src/tools/tape.test.ts` (`"endpoint" as const`).
- Vérification : `tsc -b` et `npm run build` propres ; le test passe.
- Leçon : la construction complète passe avant chaque envoi, pas après.

## H-20261118-6 · correction · interface · Les cotes ne se voient plus à travers le solide (E-46)

- Origine : JC (18 nov. 2026) : « on voit les cotations a travers le solide » (capture : cotes de l'arrière du bâtiment dessinées par-dessus les faces) ; E-46.
- Constat : les lignes des cotes posées étaient dessinées sans test de profondeur (toujours par-dessus tout), contrairement aux guides ; leur texte est une étiquette HTML posée sur la vue, sans aucun test de masquage.
- Fichiers : `src/lib/annotations.ts` (les lignes de cote avec `depthTest: true` ; l'étiquette marquée `occlude`) ; `src/components/Viewport.tsx` (`Label.occlude` : l'étiquette est cachée quand une face est entre l'œil et son point, par `engine.occluded`, déjà utilisé pour les extrémités visibles) ; `src/lib/annotations.test.ts` (2 scénarios).
- Correction : une cote derrière le volume disparaît (lignes et texte) ; la cote en cours de pose avec l'outil Cotation reste visible par-dessus, pour qu'on voie ce qu'on fait.
- Écarté : masquer aussi les cotes en cours de pose (on ne verrait plus la cote qu'on place derrière une face) ; dessiner les cotes cachées en pointillé (non demandé).
- Effet sur les projets : aucun (seul l'affichage change).
- Confidentialité : non.
- Vérification : 2 scénarios qui échouent sans la correction et passent avec ; vitest 1195 réussis, `tsc -b` et `npm run build` propres avant l'envoi. Niveau : testé (banc : 2 scénarios). Non vérifié : à l'écran dans Firefox.
- Limites connues : le texte se cache d'un coup quand son milieu passe derrière une face, alors que la ligne se cache au pixel près ; une cote posée exactement sur une face peut scintiller par endroits (même profondeur).

## H-20261118-7 · correction · module · Enregistrement refusé après ouverture d'un .shz de GeoHVAC (E-47)

- Origine : JC (18 nov. 2026, 22 h 19) : « c'est embêtant on ne sais pas sauvegarder » (capture : « Could not save: the saved copy would not have opened (… a group or instance: a name longer than 64 characters) ») ; E-47.
- Constat : le format garde deux façons de décrire un groupe ; l'ancienne liste `groups`, celle que la notice proposait à Profondeur Alpha, n'avait pas de contrôle de longueur des noms à l'ouverture, alors que la nouvelle (`nodes`), seule écrite à l'enregistrement, en a un. L'export natif de GeoHVAC nomme son groupe « GeoHVAC · bâtiment … » avec plus de 64 caractères : le fichier s'ouvrait, puis l'enregistrement refusait le nom.
- Fichiers : `crates/kernel/src/snapshot.rs` (le nom d'un groupe de l'ancienne liste est coupé à 64 caractères, « Group » s'il est vide) ; `crates/kernel/src/model.rs` (`clip_name` rendu visible dans le noyau) ; `crates/shz/src/lib.rs` (1 scénario).
- Correction : le nom trop long est coupé à 64 caractères à l'ouverture, comme pour un import ; l'enregistrement passe.
- Écarté : refuser le fichier à l'ouverture (le modèle de GeoHVAC ne s'ouvrirait plus) ; relever la limite (elle vaut pour tout le format).
- Effet sur les projets : le nom du groupe est raccourci au premier enregistrement ; rien d'autre ne change.
- Confidentialité : non.
- Vérification : 1 scénario (nom de 90 caractères : ouvert, coupé à 64, enregistré puis relu) qui échoue sans la correction ; cargo 766 réussis, clippy et fmt propres ; vitest 1195, `tsc -b` et build propres. Niveau : testé (banc : 1 scénario). Non vérifié : avec le vrai fichier de GeoHVAC (pas reçu ici).
- Limites connues : demande de reconstruire le noyau (`npm run wasm`).

## H-20261119-1 · évolution · interface · Aligner le bâtiment d'après la façade sélectionnée (E-48)

- Origine : JC (19 nov. 2026, 12 h 20) : « il manque la possibilité d'aligner un bâtiment sur un axe en utilisant la façade selectionnée » (capture : bâtiment 740C7886, 706,2 m³, posé de biais) ; E-48.
- Fichiers : `src/lib/align.ts` (`facadeAngle` : direction horizontale de la normale de la face, modulo 90°) ; `src/lib/align.test.ts` (4 scénarios) ; `src/App.tsx` (Edit > Align Façade to Axes, actif quand une seule face est sélectionnée).
- Évolution : on sélectionne une façade (double-clic dans le bâtiment, puis clic sur le mur), puis Edit > Align Façade to Axes. Le solide tourne autour du bleu, sur place, jusqu'à ce que la façade fasse face au rouge ou au vert, le plus proche des deux ; les autres solides sélectionnés tournent avec lui. La barre d'état dit l'angle et l'orientation réelle (pour le nord). Un toit ou un plancher est refusé.
- Écarté : laisser choisir l'axe (rouge ou vert) et le sens : un second tour de 90° avec Rotate le fait déjà ; changer Align to Axes (son sens actuel, les murs les plus longs, reste utile).
- Effet sur les projets : aucun en dehors de l'opération ; Undo la défait.
- Confidentialité : non.
- Vérification : 4 scénarios (façade à 20°, chemin le plus court vers le vert, façade qui l'emporte sur les murs longs, mur incliné accepté et toit refusé) ; vitest 1199, `tsc -b` et build propres. Niveau : testé (banc : 4 scénarios). Non vérifié : à la souris dans Firefox ; le cas d'un groupe lui-même tourné (comme Align to Axes).

## H-20261119-2 · évolution · interface · Filtre de sélection Points / Arêtes / Faces et panneau du point (E-49)

- Origine : JC (19 nov. 2026, 12 h 26) : « un mode point face et arrêtes ( coordonnées des points ) » ; à la question posée, JC a choisi « Filtre de sélection » et « Coordonnées modifiables, Panneau du point choisi » ; E-49.
- Fichiers : `src/lib/pickFilter.ts` (nouveau : `filterPick`, `boxFilter`, `meshPoints`) et son test ; `src/lib/entities.ts` (aides du cadre de sélection rendues publiques) ; `src/tools/select.ts` (le filtre au survol, au clic et au cadre ; la barre d'état donne x, y, z d'un point) ; `src/tools/tool.ts` (`pickFilter` de l'hôte) ; `src/tools/select.test.ts` (2 scénarios) ; `src/components/PointPanel.tsx` (nouveau : sélecteur All / Points / Edges / Faces, panneau X, Y, Z) et son test ; `src/App.tsx` (état du filtre, pastilles des points, panneau, déplacement par `transform_geometry`) ; `src/styles/app.css`.
- Évolution : dans la rangée des onglets, à côté de Model / Sheets, quatre boutons All, Points, Edges, Faces. Avec Points, un clic prend le coin le plus proche (à 30 pixels) du solide visé, un cadre prend les points qu'il contient, et tous les points sont montrés en pastilles (cachées derrière le solide). Avec Edges, l'arête la plus proche ; avec Faces, la face sous le curseur, même sur une arête. Un seul point choisi : le panneau Point montre X, Y, Z dans l'unité affichée ; une valeur tapée puis Entrée (ou en quittant le champ) déplace le point là, les faces qui doivent rester planes se plient au besoin, une seule annulation ; seul le champ modifié compte (un arrondi d'affichage ne déplace rien).
- Écarté : des étiquettes de coordonnées dans la vue, une liste exportable (non choisies par JC) ; des raccourcis clavier (risque de conflit avec les outils).
- Effet sur les projets : aucun hors des déplacements faits ; Undo les défait.
- Confidentialité : non.
- Vérification : 11 scénarios (6 du filtre, 2 de l'outil Sélection, 3 du panneau) ; vitest 1210, `tsc -b` et build propres avant l'envoi. Niveau : testé (banc : 11 scénarios). Non vérifié : à la souris dans Firefox, et le pliage des faces sur un vrai bâtiment.
- Limites connues : un point pris en mode Points vient du maillage d'affichage (précision simple, environ 0,01 mm près de l'origine) ; loin de l'origine (au-delà d'environ 500 m), le noyau pourrait ne pas reconnaître le point et refuser le déplacement. Le panneau ne s'ouvre que pour un point d'un solide (pas d'une esquisse). Il faut ouvrir le groupe (double-clic) pour prendre les points d'un bâtiment groupé.

## H-20261119-3 · évolution · module · Supprimer un point entre deux segments ; Clean Up Points (E-50)

- Origine : JC (19 nov. 2026, 12 h 55) : « en mode point on voit bien les anomalies , des points inutile, un autre soucis " il n'est pas possible de supprimer un point entre deux segments » (capture : faces éclatées, 15 régions, « A point goes with its edges: erase an edge instead. ») ; E-50.
- Constat : effacer un point était refusé d'office par l'interface ; le noyau ne savait retirer un point entre deux arêtes alignées qu'après l'effacement d'une arête, et seulement s'il était exactement aligné (0,01 mm).
- Fichiers : `crates/kernel/src/sketch.rs` (`dissolve_point`, `clean_points` ; la réparation existante passe par `straighten`, avec sa tolérance d'origine) ; `crates/kernel/src/model.rs` (ops `DissolvePoint`, `DissolveSketchPoint`, `CleanPoints`, `CleanSketchPoints` ; `dissolved_point`, `cleaned_points` ; issues `PointsCleaned`, `SketchPointsCleaned`) ; `crates/kernel/src/error.rs` (`PointStays`) ; `crates/kernel/tests/dissolve_point.rs` (5 scénarios) ; `src/kernel/types.ts` ; `src/lib/entities.ts` (Suppr et Gomme sur un point ; le même point retiré des autres esquisses où il est entre deux arêtes, pour que mur et toit se rejoignent encore) et son test (3 scénarios, dont 1 mis à jour) ; `src/tools/select.test.ts` (Gomme, mis à jour) ; `src/App.tsx` (Edit > Clean Up Points).
- Évolution : un point choisi (mode Points), puis Suppr ou la Gomme : s'il est entre deux arêtes presque alignées (à 5 cm près, la tolérance des autres outils GeoHVAC), les deux arêtes n'en font plus qu'une, dans un solide comme dans une face éclatée ; les faces voisines reprennent leur plan. Un coin (trois arêtes ou plus), un point d'arc, ou un point trop loin de la ligne reste, et la barre d'état dit pourquoi. Edit > Clean Up Points fait de même pour tous les points de cette sorte des solides et faces sélectionnés (de tout le modèle si rien n'est sélectionné), en une annulation.
- Écarté : retirer un point qui n'est pas aligné (le contour changerait de forme) ; une tolérance plus large que 5 cm.
- Effet sur les projets : aucun hors des opérations faites ; Undo les défait.
- Confidentialité : non.
- Vérification : 5 scénarios noyau (point aligné retiré d'un cube, coin refusé sans rien changer, nettoyage de deux points d'un coup puis rien, point d'esquisse à 1 cm de la ligne retiré et coin refusé, nettoyage d'une esquisse) ; 3 scénarios interface ; cargo 771, clippy et fmt propres ; vitest 1212, `tsc -b` et build propres avant l'envoi. Niveau : testé (banc : 8 scénarios). Non vérifié : sur le bâtiment 740C7886 de JC.
- Limites connues : demande `npm run wasm`. Les doubles points très proches (deux murs à 1 cm l'un de l'autre, visibles sur la capture) ne sont pas des points entre deux arêtes alignées : ils restent ; c'est la jonction de GeoHVAC qui les produit (note Profondeur Alpha). Un point pris loin de l'origine (au-delà de quelques centaines de mètres) peut ne pas être reconnu.

## H-20261119-4 · évolution · interface · Barre du bas : filtre de sélection et accrochages encadrés (E-51)

- Origine : JC (19 nov. 2026, 13 h 41) : « je préférerais un menu sur l'écran de travail , dans la partie inférieure et des encadrés pour les différents choix » (captures : le filtre dans la rangée des onglets, la légende des accrochages en bas) ; E-51.
- Fichiers : `src/components/Viewport.tsx` (`bottomLead` : ce qui précède les accrochages dans la barre du bas) ; `src/App.tsx` (le filtre et le panneau Point passent de la rangée des onglets à la barre du bas) ; `src/components/PointPanel.tsx` (titre « Select ») ; `src/styles/app.css` (barre du bas, groupes encadrés, un cadre par choix).
- Évolution : en bas de la vue, trois groupes encadrés : Select (All, Points, Edges, Faces ; le choix actif en couleur), Point (X, Y, Z quand un point est choisi), Snap (les accrochages, chacun dans son cadre, barré quand il est coupé). À la largeur de l'écran de JC, tout tient sur une ligne ; plus étroit, la barre passe sur deux lignes.
- Effet sur les projets : aucun.
- Confidentialité : non.
- Vérification : vitest 1212 (les tests du filtre et du panneau passent tels quels), `tsc -b` et build propres ; captures dans Chrome sans écran à 1500 et 1835 px de large (une ligne à 1835 px, rien ne chevauche le rappel des axes). Niveau : essayé (capture). Non vérifié : dans Firefox chez JC.

## H-20261119-5 · évolution · module · Effacer arêtes et faces d'un volume (E-52) ; menu Volume (E-53)

- Origine : JC (19 nov. 2026, 14 h 03) : « je ne sais supprimer aucunes lignes ni surface » (capture : « erasing that would open the solid (only lines drawn on a face can be erased; delete the whole solid instead) ») ; JC (14 h 05) : « le menu édit devient confus » ; le choix a) de JC du 18 nov. 2026 pour la suppression d'une face ; E-52, E-53.
- Constat : sur un volume, la Gomme et Suppr n'effaçaient que les lignes tracées sur une face ; toute autre arête ou face était refusée, d'où « aucune ligne ni surface ».
- Fichiers : `crates/kernel/src/model.rs` (ops `EraseSolidEdges`, `EraseSolidFaces`, issue `SolidOpened`) ; `crates/kernel/tests/erase_solid.rs` (5 scénarios) ; `src/kernel/types.ts` ; `src/lib/entities.ts` (Suppr et Gomme passent par ces ops, regroupées par volume ; `erasedWords`) et son test ; `src/tools/erase.ts`, `src/tools/erase.test.ts`, `src/tools/select.test.ts` (tests mis à jour) ; `src/App.tsx` (menu Volume ; messages « Volume > ») ; `src/components/TopBar.tsx` (Volume entre Edit et View) ; `src/tools/pushaxis.ts`, `src/lib/extrudeAlong.ts`, `src/lib/align.ts` (renvois) ; `e2e/files.spec.ts`.
- Évolution : une ligne tracée sur une face part comme avant. Une arête entre deux faces dans un même plan (à 5° et 5 cm près) part, les deux faces fusionnent et le volume reste. Toute autre arête, ou une face : le volume devient des faces simples (comme Explode Solid into Faces) et l'arête ou la face est effacée, avec les faces qu'elle bordait ; la barre d'état le dit, et Volume > Make Solid from Faces referme. Plusieurs arêtes ou faces d'un même volume partent en une fois, une seule annulation. Le nouveau menu Volume : Join Solids, Explode Solid into Faces, Make Solid from Faces, Extrude Faces Along Axis… | Merge Faces, Auto Merge Faces, Clean Up Points | Align to Axes, Align Façade to Axes | Unify Imported Meshes, Delete Imported Meshes.
- Écarté : demander une confirmation avant d'ouvrir le volume (Undo le défait, et c'est le choix a) de JC) ; des sous-menus dans Edit.
- Effet sur les projets : aucun hors des effacements faits.
- Confidentialité : non.
- Vérification : 5 scénarios noyau (arête entre deux faces coplanaires : fusion, volume gardé ; arête de coin : volume ouvert, 2 faces parties ; une face ; deux faces ensemble ; deux arêtes en une fois) ; tests interface mis à jour (4) ; cargo 776, clippy et fmt propres ; vitest 1212, `tsc -b` et build propres avant l'envoi. Niveau : testé (banc : 9 scénarios). Non vérifié : sur le bâtiment de JC ; les essais e2e (Playwright) non lancés ici.
- Limites connues : demande `npm run wasm`. Une sélection qui mêle des arêtes et des faces d'un même volume ouvert par les arêtes est refusée en entier (effacer en deux fois).

## H-20261119-6 · évolution · module · Le point au bout d'une ligne qui dépasse part avec elle (E-54)

- Origine : JC (19 nov. 2026, 14 h 39), capture sans texte : faces éclatées, mode Points, « Can't do that. that point stays: it ends an edge (erase the edge instead) » ; E-54.
- Constat : le point visé ne terminait qu'une seule arête dans sa face (un bout de ligne qui dépasse, tel qu'en laisse un effacement) ; E-50 ne retirait qu'un point entre deux arêtes.
- Fichiers : `crates/kernel/src/sketch.rs` (`dissolve_point` : un point au bout d'une ligne efface la ligne, comme la Gomme, et ressoude ce qui était coupé à son autre bout ; un point seul part) ; `crates/kernel/tests/dissolve_point.rs` (1 scénario).
- Évolution : Suppr ou Gomme sur l'extrémité libre d'une ligne : la ligne part ; un point isolé part. Un point de trois arêtes ou plus, un point d'arc, un coin restent, avec la raison.
- Effet sur les projets : aucun hors des effacements faits.
- Confidentialité : non.
- Vérification : 1 scénario (carré avec une ligne qui dépasse : supprimer son bout efface la ligne et le côté redevient une seule arête) ; cargo 777, clippy et fmt propres ; vitest 1212. Niveau : testé (banc : 1 scénario). Non vérifié : sur le modèle de JC ; je n'ai pas la certitude que c'était un bout de ligne (déduit du message).
- Limites connues : demande `npm run wasm`. Avec plusieurs points sélectionnés, un seul refus fait refuser l'ensemble (les supprimer un à un).

## H-20261120-1 · évolution · interface · Repère des axes dans le coin de la vue (E-55)

- Origine : JC (20 nov. 2026, 14 h 17) : « il manque une gestion des axes dans le coin supérieur droit de la vue » (deux captures : le repère de Blender, la vue de VolumeCraft) ; E-55.
- Fichiers : `src/lib/axisGizmo.ts` (nouveau : `gizmoDots`, place des six pastilles d'après la caméra, la plus proche dessinée en dernier) et son test (3 scénarios) ; `src/viewport/engine.ts` (`axisFrame` : les axes du monde vus par la caméra) ; `src/components/Viewport.tsx` (`AxisGizmo`, `onAxisView`) ; `src/App.tsx` (clic → `setView`) ; `src/styles/app.css`.
- Évolution : en haut à droite de la vue, X rouge, Y vert, Z bleu en pastilles pleines avec leur lettre, leurs opposés en cercles ; le repère suit l'orbite. Un clic : X → Right, −X → Left, Y → Back, −Y → Front, Z → Top, −Z → Bottom (vue depuis ce côté, comme Blender).
- Écarté : tourner la vue en glissant sur le repère (non demandé ; l'orbite reste à la souris).
- Effet sur les projets : aucun.
- Confidentialité : non.
- Vérification : 3 scénarios ; vitest, `tsc -b` et build propres ; dans Chrome sans écran : le repère en vue Persp, un clic sur Z donne la vue Top (data-view = top) et le repère montre Z face à l'œil, X à droite, Y en haut. Niveau : essayé (capture). Non vérifié : dans Firefox chez JC.

## H-20261120-2 · évolution · interface · Onglet Info (E-56) et menu clic droit (E-57)

- Origine : JC (20 nov. 2026, 14 h 24) : lien vers SketchCraft (github.com/jasonsutter87/sketchcraft, MIT) ; 14 h 27 : « c'est surtout les plus de l'interface qui m'intéresse » ; parmi quatre idées proposées, JC choisit « Menu clic droit, Panneau Propriétés » ; 14 h 29 : « ic on reprend l'idée pas le code ». Seuls le README, les guides et les images de démonstration de SketchCraft ont été lus ; aucun code n'en est repris ; E-56, E-57.
- Fichiers : `src/lib/properties.ts` (nouveau : surfaces et volume tirés du maillage d'affichage, sorte de face, orientation depuis le nord, pente, `selectionInfo`) et son test (5 scénarios) ; `src/components/InfoPane.tsx` (nouveau) ; `src/components/SidePanel.tsx` (onglet Info, deuxième) ; `src/lib/viewMenu.ts` (nouveau : les commandes par sorte de sélection, reprises des menus) et son test (3 scénarios) ; `src/components/ViewMenu.tsx` (nouveau) ; `src/components/Viewport.tsx` (clic droit sans glisser) ; `src/App.tsx` ; `src/App.test.tsx` (liste des onglets) ; `src/styles/app.css`.
- Évolution : onglet Info à droite, à jour avec la sélection. Une face : surface, sorte (mur à 10° près de la verticale, toit, plancher), orientation (point cardinal et degrés depuis le nord, le vert) et pente d'un toit. Un volume ou un groupe : volume, enveloppe, murs, toits, plancher. Plusieurs faces : leur total par sorte. Une arête : longueur et pente. Un point : X, Y, Z. Rien : tout le modèle. Clic droit sans glisser dans la vue : ce qui est sous le curseur devient la sélection s'il ne l'était pas, et un menu propose ce qui s'y applique (Delete ; pour une face : Align Façade to Axes, Merge Faces, Auto Merge Faces, Clean Up Points ; pour un volume : Join Solids, Explode Solid into Faces, Align to Axes… ; Make Group, Hide, Lock ; Properties, qui ouvre l'onglet Info). Seules les commandes possibles sont montrées. Un glisser au bouton droit déplace toujours la vue.
- Écarté : la barre de vue rapide et la recherche de commande (non choisies) ; ouvrir Info tout seul à chaque sélection (changerait d'onglet sans le demander).
- Effet sur les résultats : les mesures sont lues dans le maillage d'affichage (précision simple : largement au cm² près) ; le volume du badge reste celui du noyau.
- Effet sur les projets : aucun.
- Confidentialité : non.
- Vérification : 8 scénarios ; vitest 1223, `tsc -b` et build propres ; dans Chrome sans écran : une boîte tracée, clic droit sur sa face du dessus → menu (Delete, Align Façade to Axes, Auto Merge Faces, Clean Up Points, Make Group, Hide, Lock, Properties) ; Properties → Info : « Face of a solid, 0.37 ft², Roof, 0.0° » ; sans sélection, Info donne 0.09 ft³ (158,4 in³ au badge). Niveau : essayé. Non vérifié : Firefox et stylet chez JC (clic droit du stylet : bouton latéral).
- Limites connues : Align Façade to Axes est proposé pour toute face seule, même un toit (la commande refuse alors avec la raison). Le nord est fixe (+Y) tant qu'E-08 n'est pas fait.

## H-20261122-1 · évolution · interface · Copier / Coller ; Move + Ctrl copie le contour d'une face (E-58)

- Origine : JC (22 nov. 2026, 23 h 08) : « j'ai tenter un copié collé de la base du polygone d'un batiment et rien aucune possibilité de le copié de de le déplacer sur la map » ; E-58.
- Constat : VolumeCraft n'avait ni Copier ni Coller ; Move sur une face d'un volume déplace la face elle-même (les murs suivent ou c'est refusé), et Ctrl n'y faisait pas de copie. « La map » est lue comme le sol de la vue de VolumeCraft (lecture de Claude, à confirmer).
- Fichiers : `src/tools/paste.ts` (nouveau : `Clip`, `anchorOf`, `pasteOps`, outil de pose) et `src/tools/paste.test.ts` (6 scénarios) ; `src/tools/move.ts` (`outlineCopyOps` ; Ctrl sur des faces d'un volume : copie du contour, aperçu et messages) ; `src/tools/xform.ts` (`faceRings` du sujet) ; `src/tools/tool.ts` (`done`) ; `src/lib/tools.ts` (outil « paste », sans bouton) ; `src/components/glyphs.tsx` ; `src/App.tsx` (Edit > Copy, Paste, Ctrl+C, Ctrl+V).
- Évolution : sélectionner (une face de base, un volume, un groupe, des faces simples, des arêtes), Ctrl+C, puis Ctrl+V : la copie suit le curseur, tenue par son coin le plus bas, avec l'accrochage habituel ; un clic la pose, en une annulation, et l'outil revient à Select ; Esc annule. Un volume, un groupe ou un composant est copié entier ; une face (d'un volume ou simple) est copiée en contour, posé en face plate sur son plan à l'arrivée (Push/Pull la monte) ; une arête en ligne. Move avec Ctrl sur une face d'un volume copie de même son contour au lieu de déformer le volume ; sans Ctrl, rien ne change.
- Écarté : Couper (il faudrait garder une copie de ce qui est effacé) ; Coller sur place ; une valeur tapée pendant la pose (Move la prend déjà).
- Effet sur les projets : aucun.
- Confidentialité : non (le presse-papier reste dans VolumeCraft, pas celui du système).
- Vérification : 6 scénarios ; vitest 1229, `tsc -b` et build propres ; dans Chrome sans écran : boîte tracée, clic sur sa face du dessus, Ctrl+C (« Copied the outline of a face »), Ctrl+V, clic ailleurs : la face plate est posée au sol, une étape d'annulation, retour à Select. Niveau : essayé. Non vérifié : dans Firefox chez JC ; la face du dessous d'un bâtiment (cachée : la prendre en vue Bottom ou en Faces, ou par un clic à travers en Rayons X).
- Limites connues : un trou dans une face collée revient comme une face à part dans le trou, pas comme un trou. Coller sur un sol déjà occupé par un volume soude le contour aux lignes qui s'y trouvent.

## H-20261124-1 · évolution · interface · Interface en français, étape 1 (E-59)

- Origine : JC (24 nov. 2026, 08 h 47) : « je n'avais pas fait attention mais il faudrait pouvoir changer la langue et mettre en français » ; E-59.
- Fichiers : `src/lib/i18n.ts` (nouveau : langue choisie, gardée dans le navigateur, sinon celle du navigateur ; `t`, `tChip`) ; `src/lib/i18n-fr.ts` (nouveau : le français, indexé par le texte anglais ; phrases exactes, puis modèles pour les phrases à nombres et à noms) ; `src/lib/i18n.test.ts` (5 scénarios) ; textes passés par `t` là où ils s'affichent : `src/components/TopBar.tsx`, `ToolRail.tsx`, `ValueBar.tsx`, `Viewport.tsx`, `SidePanel.tsx`, `InfoPane.tsx`, `PointPanel.tsx`, `ViewMenu.tsx`, `SceneTabs.tsx`, `src/sheets/SheetTabs.tsx` ; `src/App.tsx` (Édition > Langue) ; `src/lib/properties.ts` et son test (ligne « Facing », pour ne pas se confondre avec le filtre Faces).
- Évolution : Édition > Langue (Language) : English ou Français, retenu pour les sessions suivantes ; à la première ouverture, la langue du navigateur. En français : barre de menus et menus, info-bulles, barre d'outils (noms de SketchUp en français : Pousser, Déplacer, Mètre, Rapporteur, Suivez-moi, Colorier…) et leurs aides, vues, onglets de droite et leurs titres, barre du bas (Sélection, Accrochage), case Point, onglet Info, menu du clic droit, repère des axes, chips du haut de la vue, barre d'état et messages les plus courants, refus du noyau propres à VolumeCraft. Le repère interne « sketchcraft: » des info-bulles n'est plus affiché (dans les deux langues).
- Écarté : traduire à chaque endroit où le texte est écrit (des centaines de changements) ; recharger la page au changement de langue (le modèle reste, l'affichage se refait).
- Effet sur les projets : aucun.
- Confidentialité : non.
- Vérification : 5 scénarios ; vitest 1234, `tsc -b` et build propres ; dans Chrome sans écran réglé en français (fr-BE) : menus, barre d'outils, barre du bas, onglet Info et barre d'état en français (captures). Niveau : essayé. Non vérifié : Firefox chez JC.
- Limites connues (étape 2) : boîtes de dialogue (Réglages, unités, extrusion…), liste de débit, matières, styles, feuilles, extensions, structure (noms « Solid 1 »), la plupart des messages des autres outils et du noyau restent en anglais. Les nombres gardent le point décimal (0.37) hors orientation.

## H-20261124-2 · évolution · documentation · Lanceurs Linux et Windows, installation par utilisateur (E-60)

- Origine : JC (24 nov. 2026, 09 h 23) : « un veritable script pour lancer volumecraft sous windows et linux , un programme d'installation » ; E-60.
- Fichiers (nouveaux) : `packaging/linux/volumecraft.sh` (lanceur : Node ≥ 20, npm ci si besoin, noyau reconstruit si ses sources sont plus récentes et Rust/wasm-pack présents, sinon garde le noyau existant ; npm run build si besoin ; vite preview sur 127.0.0.1:4173 ; ouvre le navigateur ; --stop, --status) ; `packaging/linux/install.sh` (entrée de menu avec action Arrêter, icône, lien ~/.local/bin/volumecraft ; --uninstall) ; `packaging/windows/VolumeCraft.ps1` et `VolumeCraft.cmd` (lanceur, serveur caché, -Stop) ; `packaging/windows/install.ps1` et `Install VolumeCraft.cmd` (raccourcis menu Démarrer et bureau, avec icône ; -Uninstall) ; `packaging/README.md`.
- Évolution : VolumeCraft se lance depuis le menu ou un raccourci, sans commandes npm ; relancé, il ne fait qu'ouvrir le navigateur ; « Stop VolumeCraft » l'arrête. Rien n'est écrasé sans copie : un fichier déjà en place est copié dans un dossier `backup-<date>`, et la désinstallation y déplace les fichiers au lieu de les supprimer.
- Écarté : construire les installateurs Tauri ici (pas de cible wasm ni de Windows dans cet environnement ; le flux GitHub Actions existe, à lancer avec l'accord de JC : E-61).
- Effet sur les projets : aucun.
- Confidentialité : non (le serveur n'écoute que 127.0.0.1).
- Vérification : Linux, dans un dossier personnel temporaire : premier lancement (reconstruction du noyau échouée ici faute de cible wasm → noyau existant gardé), serveur répondant (titre VolumeCraft), relance → navigateur seulement, --stop, --status ; lancement suivant sans nouvelle tentative de reconstruction ; install.sh (entrée de menu, icône, lien), réinstallation (copie de sauvegarde), --uninstall (déplacement). Niveau : essayé (Linux). Non vérifié : Windows (pas de PowerShell ici : scripts relus seulement) ; le menu d'un vrai bureau Linux chez JC.
- Limites connues : le lanceur Windows ne construit pas le noyau (src\wasm\pkg doit exister).

## H-20261124-3 · évolution · module · Le nord du document et la rose des vents (E-62)

- Origine : JC (24 nov. 2026, 09 h 41) : « il faudrait intégrer la rose des vents liée au bâtiment importé » ; E-62 (reprend E-08, Nord).
- Constat : le nord était fixé le long du vert (+Y) ; c'est juste pour un bâtiment de GeoHVAC tel qu'il arrive (coordonnées Lambert), faux dès qu'Aligner sur les axes ou Aligner la façade l'a tourné : l'orientation des faces (Info) et le soleil se trompaient alors de l'angle du tour.
- Fichiers : `crates/kernel/src/model.rs` (`north`, `Op::SetNorth`, gardé dans (-180, 180]) ; `crates/kernel/src/snapshot.rs` (champ `north` du scene.json, absent à 0) ; `crates/kernel/src/hash.rs` (le nord compte seulement s'il n'est pas 0 : les anciens documents gardent leur empreinte) ; `crates/wasm-api/src/lib.rs` (état) ; `crates/shz/src/lib.rs` (1 scénario) ; `src/kernel/types.ts` ; `src/lib/looks.ts` et `src/viewport/engine.ts` (le soleil suit le nord) ; `src/lib/properties.ts` et son test (orientation depuis le nord du document) ; `src/lib/align.ts` (`northAfterTurn`) et son test ; `src/App.tsx` (les deux Aligner tournent aussi le nord, en une annulation ; Affichage > Nord… ; un clic sur la rose) ; `src/components/Viewport.tsx` (`CompassRose`, sous le repère des axes, couchée au sol comme la caméra la voit) ; `src/components/NameDialog.tsx` (traduit) ; `src/lib/i18n-fr.ts` ; `src/styles/app.css`.
- Évolution : une rose des vents (N rouge) sous le repère des axes, posée au sol comme la caméra la voit. Le nord est rangé dans le document (enregistré, annulable) ; Aligner sur les axes et Aligner la façade le tournent avec le bâtiment ; Affichage > Nord… (ou un clic sur la rose) le règle en degrés depuis le vert. L'orientation des faces dans Info et la course du soleil en tiennent compte.
- Écarté : ranger le nord dans les données d'extension (non annulable, absent de la sauvegarde automatique) ou dans le réglage du soleil (enregistré aussi par scène, il reviendrait faux avec une ancienne scène).
- Effet sur les résultats : orientations (Info) et ombres justes après un alignement. Nord de grille (Lambert) : le nord géographique en diffère de la convergence des méridiens (moins d'un degré en Belgique).
- Effet sur les projets : format .shz : champ facultatif `north` ; un fichier sans lui s'ouvre comme avant (nord le long du vert) ; un fichier avec lui, ouvert par une version plus ancienne de VolumeCraft, est refusé si celle-ci refuse les champs inconnus (non vérifié).
- Confidentialité : non.
- Vérification : 1 scénario noyau (enregistrer et relire, normalisation, empreinte inchangée à 0, refus d'une valeur non numérique) ; 2 scénarios interface (orientation avec le nord tourné ; nord après un tour) ; 1 de traduction ; cargo 778, clippy et fmt propres ; vitest 1237, `tsc -b` et build propres ; dans Chrome sans écran (en français) : rose en vue Iso (N dans la direction du vert) et en vue de dessus (N en haut, E à droite), clic sur la rose → dialogue « Nord ». Niveau : testé (banc : 4 scénarios) et essayé. Non vérifié : Aligner puis la rose avec le nouveau noyau (pas de cible wasm ici : le noyau du navigateur d'essai est l'ancien) ; chez JC.
- Limites connues : demande `npm run wasm` (ou le lanceur, qui le fait).
