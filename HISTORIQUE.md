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
