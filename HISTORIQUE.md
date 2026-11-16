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
