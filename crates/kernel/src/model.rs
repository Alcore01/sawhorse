//! The geometric state: sketches and objects, changed only through transactional [`Op`]s.
//!
//! `apply` = clone -> mutate -> validate -> commit (ADR-0001). A refused or failed op
//! leaves the model exactly as it was. In debug builds every B-rep and sketch mutation
//! additionally runs the full validator on the spot, so a kernel bug panics at the mutation
//! that caused it.
//!
//! Ops address sketches, regions and objects by **public id** (see [`crate::ids`]) and
//! faces by **persistent face id** within their object (see [`crate::object::Object`]):
//! all stable across edits, so the UI can hold on to them. A face id survives the push
//! that moves the face (topological and boolean paths alike), splits (the first piece
//! keeps it) and coplanar merges (the merged face keeps the smallest member's id).
//! An op sequence is plain data: it serialises to JSON for the fuzzer's repro files, the
//! golden replays and the document's op log, and replays identically everywhere.

use glam::{DVec2, DVec3};
use slotmap::SlotMap;

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{
    DocumentError, FaceEditError, KernelError, OffsetError, PushError, SketchError, TransformError,
};
use crate::ids::{
    DefId, DimensionId, GuideId, MeshId, NodeId, ObjectId, RegionId, SketchId, from_public,
    to_public,
};
use crate::meshbuild::{ImportItem, LooseMesh, build_item};
use crate::nodes::{Definition, Dirty, Item, Node, NodeKind, Thing};
use crate::object::{Object, Provenance, PushOutcome};
use crate::offset::OffsetSource;
use crate::plane::Plane;
use crate::sketch::{NewCurve, Sketch};
use crate::sweep::Profile;
use crate::tolerance::PLANARITY;
use crate::transform::Xform;
use crate::validate::{TopologyError, Violation};

/// Most copies one array op makes.
pub const MAX_ARRAY: u32 = 1000;

fn is_one(n: &u32) -> bool {
    *n == 1
}

/// One undoable edit. Lengths in metres; ids are public ids.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// New empty sketch on the plane through `origin` with `normal`.
    NewSketch {
        origin: [f64; 3],
        normal: [f64; 3],
    },
    /// Rectangle in the sketch's 2D frame: corner (x, y), size (w, h).
    AddRectangle {
        sketch: u64,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    /// One straight segment in the sketch's 2D frame, inserted with the sticky rules.
    InsertSegment {
        sketch: u64,
        a: [f64; 2],
        b: [f64; 2],
    },
    /// Erase every edge lying along a segment (the Eraser), healing the T-junctions and
    /// crossings it leaves behind.
    EraseSegment {
        sketch: u64,
        a: [f64; 2],
        b: [f64; 2],
    },
    /// A polyline (closed: the last point joins the first), as one transaction.
    AddPolyline {
        sketch: u64,
        points: Vec<[f64; 2]>,
        closed: bool,
    },
    /// Extrude region `region` of sketch `sketch` into a new solid. The region leaves the
    /// sketch; the solid's base face takes its place.
    Extrude {
        sketch: u64,
        region: u64,
        distance: f64,
    },
    /// Push/pull face `face` (persistent face id) of object `object` along its outward
    /// normal.
    PushPull {
        object: u64,
        face: u64,
        distance: f64,
    },
    /// Push/pull that leaves a seam where the face was (Ctrl in the UI): an outward pull
    /// gives every edge of the face a new side wall instead of stretching the old walls.
    /// Inward pushes behave like [`Op::PushPull`].
    PushPullNewFace {
        object: u64,
        face: u64,
        distance: f64,
    },
    /// Cut face `face` of `object` in two along `path` (world points on the face's plane;
    /// ends on the face's boundary, interior strictly inside).
    SplitFace {
        object: u64,
        face: u64,
        path: Vec<[f64; 3]>,
    },
    /// Imprint the closed loop `points` (strictly inside face `face`) as a coplanar
    /// sub-face twinned with a new hole in the face.
    Imprint {
        object: u64,
        face: u64,
        points: Vec<[f64; 3]>,
    },
    /// Draw a stroke (world points; `closed` joins the last to the first) as one undoable
    /// edit: into a sketch, onto a plane (the sketch already on that plane, or a new one),
    /// or onto a solid's face plane (cuts and imprints; see [`crate::draw`]).
    Draw {
        target: DrawTarget,
        points: Vec<[f64; 3]>,
        closed: bool,
        /// Lane 23: the stroke is one curve (the Circle, Arc and Polygon tools): its kind and
        /// centre; the points must lie on a circle about the centre (a circle or a polygon
        /// closed, an arc open). Drawn into a sketch it becomes a curve there; on a solid's
        /// face it is drawn as plain edges (see ADR-0013).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        curve: Option<crate::curve::CurveSpec>,
    },
    /// Lane 23: erase a whole curve of a sketch (the Eraser on any of its segments).
    EraseCurve {
        sketch: u64,
        curve: u64,
    },
    /// Lane 23: Explode Curve: a curve of a sketch, or of a solid (its rim), becomes plain
    /// edges again. Exactly one of `sketch` and `object`.
    ExplodeCurve {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sketch: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        object: Option<u64>,
        curve: u64,
    },
    /// Erase the edge of `object` from `a` to `b` (world points at its two ends). Only a
    /// line drawn on a face can go: its two coplanar faces merge (with every other edge they
    /// share). Anything else would open the solid and is refused.
    EraseEdge {
        object: u64,
        a: [f64; 3],
        b: [f64; 3],
    },
    /// Erase face `face` (persistent id) of `object` into a coplanar neighbour across a drawn
    /// line (an imprint, one side of a cut). Refused where it would open the solid.
    EraseFace {
        object: u64,
        face: u64,
    },
    /// Erase a sketch region: its edges that bound no other region leave the sketch.
    EraseRegion {
        sketch: u64,
        region: u64,
    },
    /// Several ops as one transaction and one undo step (no undo, redo or batch inside).
    Batch {
        ops: Vec<Op>,
    },
    /// Add a guide: an infinite line through `point` along `dir`, or a guide point when
    /// `dir` is absent (the Tape measure). Guides are construction help: inferable, never
    /// part of a solid.
    AddGuide {
        point: [f64; 3],
        dir: Option<[f64; 3]>,
    },
    /// Remove one guide.
    EraseGuide {
        guide: u64,
    },
    /// Remove every guide.
    DeleteGuides,
    /// Add a linear dimension between `a` and `b`, its line drawn `offset` away from them.
    AddDimension {
        a: [f64; 3],
        b: [f64; 3],
        offset: [f64; 3],
    },
    /// Remove one dimension.
    EraseDimension {
        dimension: u64,
    },
    /// Move, rotate, scale or mirror whole solids, groups and component instances by an
    /// affine map (Move, Rotate and Scale tools). Face ids, drawn edges, tags and display
    /// numbers stay; each result must validate (see [`crate::transform`]). Solids never weld
    /// to what they touch. A group moves with everything in it; an instance's placement
    /// changes and its solids are re-derived from its definition.
    Transform {
        objects: Vec<u64>,
        xform: Xform,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        nodes: Vec<u64>,
    },
    /// Copy solids, groups and instances, each copy mapped by `xform` (Ctrl with Move /
    /// Rotate). Solid copies are new plain solids (new ids and display numbers) with the
    /// originals' face ids, tags and drawn edges, in the open context. A group's copy is a
    /// new group with copies of everything in it; an instance's copy is a new instance of
    /// the same definition.
    Copy {
        objects: Vec<u64>,
        xform: Xform,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        nodes: Vec<u64>,
    },
    /// An array: `count` copies of each solid (and node), copy k mapped by `xform` applied k
    /// times (x3 after a 36 in move: copies at 36, 72 and 108 in; /4 is the same with a
    /// quarter of the move). Copies are made in order k = 1..=count, solids then nodes.
    Array {
        objects: Vec<u64>,
        xform: Xform,
        count: u32,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        nodes: Vec<u64>,
    },
    /// Move (or scale, or turn) part of a solid: the vertices at `points` (world positions)
    /// by `xform`. A face bent out of its plane is refused unless `autofold` (then it folds
    /// along a triangulation and coplanar pieces merge back).
    TransformGeometry {
        object: u64,
        points: Vec<[f64; 3]>,
        xform: Xform,
        autofold: bool,
    },
    /// Move, turn or scale sketch geometry (lane 22, ADR-0012): the vertices at `points`
    /// (world positions), the edges between the pairs in `edges` (world positions of their
    /// ends) and the whole `regions` of sketch `sketch`, by `xform`. Within the plane (or for
    /// part of a shape, projected into it) the neighbours' edges follow and the result is
    /// re-arranged by the insertion rules (welds, splits, merges); a whole shape mapped off
    /// its plane goes to the sketch on the plane it lands on. With `copy`, `count` copies of
    /// the selected edges and regions' rings are drawn instead (copy k mapped `count` = k
    /// times). See [`crate::sketch_move`].
    TransformSketch {
        sketch: u64,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        points: Vec<[f64; 3]>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        edges: Vec<[[f64; 3]; 2]>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        regions: Vec<u64>,
        /// Lane 23: whole curves of the sketch, by id (their edges are carried; a moved
        /// circle stays a circle, a copy is a new curve).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        curves: Vec<u64>,
        xform: Xform,
        #[serde(default, skip_serializing_if = "is_false")]
        copy: bool,
        #[serde(default = "one", skip_serializing_if = "is_one")]
        count: u32,
    },
    /// Offset a face, a sketch region or a chain of edges by `distance` (positive grows;
    /// for an open chain, positive is to its left). The result is drawn like a stroke: on
    /// the solid when it lies on its faces (imprints, cuts), else into a sketch on the plane
    /// (see [`crate::offset`]).
    Offset {
        source: OffsetSource,
        distance: f64,
    },
    /// Follow Me: sweep a profile along a path of straight segments (world points; `closed`
    /// joins the last to the first). A region profile becomes a new solid; a face profile
    /// adds to its solid (path leaving the face) or cuts it (path running into it). See
    /// [`crate::sweep`].
    FollowMe {
        profile: Profile,
        path: Vec<[f64; 3]>,
        closed: bool,
    },
    /// Delete an object (in the open context).
    DeleteObject {
        object: u64,
    },
    /// Group solids under a name (lane 2's form of [`Op::MakeGroup`]).
    CreateGroup {
        name: String,
        objects: Vec<u64>,
    },
    /// Dissolve a group, keeping what is in it (lane 2's form of [`Op::Explode`]).
    DeleteGroup {
        group: u64,
    },
    /// Open a group or component instance for editing (`None`: the whole model). Not an
    /// undo step: Ctrl+Z still undoes the last edit (and returns to where it was made).
    SetContext {
        node: Option<u64>,
    },
    /// Close the open group or instance (one level up). Not an undo step.
    LeaveContext,
    /// Make Group: solids, groups and instances directly in the open context go into a new
    /// group there.
    MakeGroup {
        items: Vec<Item>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// Make Component: the items become a new definition named `name` with one instance
    /// where they were (nothing moves).
    MakeComponent {
        items: Vec<Item>,
        name: String,
    },
    /// Give one instance its own copy of its definition.
    MakeUnique {
        node: u64,
    },
    /// Explode a group or instance: what is inside moves up a level, unchanged.
    Explode {
        node: u64,
    },
    /// sketchcraft E-28: a solid becomes plain faces: each face a region of the sketch on
    /// its plane in the open context; the solid (and its volume) goes. A face can then be
    /// erased like any region.
    ExplodeSolid {
        object: u64,
    },
    /// sketchcraft E-28: faces (`[sketch, region]` pairs) that close a volume become one
    /// solid; the regions are used up as an extrusion uses its region.
    SolidFromFaces {
        faces: Vec<[u64; 2]>,
    },
    /// sketchcraft E-30: faces (`[sketch, region]` pairs, touching or not) extruded
    /// together along `vector` (metres) into one solid: a roof slab raised straight up from
    /// both slopes. The faces stay.
    ExtrudeFaces {
        faces: Vec<[u64; 2]>,
        vector: [f64; 3],
    },
    /// sketchcraft E-04: solids that touch or overlap become one (the first keeps its
    /// name, tag and paint; the others go).
    UnionSolids {
        objects: Vec<u64>,
    },
    /// sketchcraft E-40: an imported building mesh that is not a solid (GeoHVAC's .dae:
    /// roof pieces a few centimetres apart, walls and floor not sewn) rebuilt as one solid
    /// from its roof: the roof welded, vertical walls from its outline down to the floor
    /// (and up or down the steps between roof parts), a flat floor. The mesh goes.
    UnifyMesh {
        mesh: u64,
    },
    /// sketchcraft E-50: remove a point of a solid that sits between two edges nearly in
    /// line (at most 5 cm off the line between its neighbours): the two edges become one,
    /// the faces on either side refit their planes. `at` is the point (world).
    DissolvePoint {
        object: u64,
        at: [f64; 3],
    },
    /// sketchcraft E-50: the same for a sketch point (sketch-plane coordinates).
    DissolveSketchPoint {
        sketch: u64,
        at: [f64; 2],
    },
    /// sketchcraft E-50: every point of the sketch between two edges nearly in line (within
    /// 5 cm) removed.
    CleanSketchPoints {
        sketch: u64,
    },
    /// sketchcraft E-50: every point of the solid that sits between two edges nearly in line
    /// (within 5 cm) removed, one after another while any is left.
    CleanPoints {
        object: u64,
    },
    /// sketchcraft E-41: faces of one solid, next to each other and nearly in one plane
    /// (within 5 cm), made one flat face: their corners go onto the plane that fits them best
    /// (straight up or down, unless the plane is upright), the edges between them go.
    MergeFaces {
        object: u64,
        faces: Vec<u64>,
    },
    /// sketchcraft E-42: every group of faces of the solid that are side by side, face
    /// within 5° of one another and lie within 5 cm of one plane, merged as
    /// [`Op::MergeFaces`] would (a group that would bend a face beside it is left).
    AutoMergeFaces {
        object: u64,
    },
    /// Delete a group or instance and everything in it.
    DeleteNode {
        node: u64,
    },
    /// Name a solid, group or instance (`None` clears it).
    Rename {
        item: Item,
        name: Option<String>,
    },
    /// Rename a component definition (every instance shows the new name).
    RenameDefinition {
        definition: u64,
        name: String,
    },
    /// Hide or show solids, groups and instances (the Outliner's eye).
    SetHidden {
        items: Vec<Item>,
        hidden: bool,
    },
    /// Lock or unlock solids, groups and instances.
    SetLocked {
        items: Vec<Item>,
        locked: bool,
    },
    /// Put solids, groups and instances on a tag (`None` or "Untagged": no tag). Tags
    /// control visibility only; nothing moves.
    TagItems {
        items: Vec<Item>,
        tag: Option<String>,
    },
    /// Create a tag (visible), with an sRGB colour for the Tags pane.
    CreateTag {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<[u8; 3]>,
    },
    /// Rename a tag everywhere it is used (items, definitions, scenes).
    RenameTag {
        tag: String,
        name: String,
    },
    SetTagColor {
        tag: String,
        color: Option<[u8; 3]>,
    },
    /// Delete a tag: whatever was on it becomes untagged and stays where it is.
    DeleteTag {
        tag: String,
    },
    /// Save a scene: camera, hidden tags and the open context.
    AddScene {
        scene: SceneDef,
    },
    /// Replace the scene named `name` (also renames it).
    UpdateScene {
        name: String,
        scene: SceneDef,
    },
    DeleteScene {
        name: String,
    },
    /// Switch to a scene's tag visibility and editing context (the UI sets its camera).
    ActivateScene {
        name: String,
    },
    /// Put an object on a tag (layer), or take it off (`None`). New tags start visible.
    SetTag {
        object: u64,
        tag: Option<String>,
    },
    /// Show or hide every object on a tag.
    SetTagVisible {
        tag: String,
        visible: bool,
    },
    /// Bring in geometry from a file as one undoable step: polygon soups become solids
    /// where they are valid solids and non-solid meshes (with a note) where they are not,
    /// strokes go into sketches, plus materials, scenes and hidden tags. See
    /// [`crate::meshbuild`].
    Import {
        #[serde(default)]
        items: Vec<ImportItem>,
        #[serde(default)]
        strokes: Vec<ImportStroke>,
        #[serde(default)]
        materials: Vec<MaterialDef>,
        #[serde(default)]
        scenes: Vec<SceneDef>,
        #[serde(default)]
        hidden_tags: Vec<String>,
        /// Component definitions the import places (a `.skp`'s components), in dependency
        /// order: a definition only places definitions listed before it.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        definitions: Vec<ImportDefinition>,
        /// Groups and component instances, top level (put in the open context).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        nodes: Vec<ImportNode>,
    },
    /// Add or replace a material: a colour, a texture (a path of an image the model holds,
    /// or the file name of one it does not, kept as a reference), the texture's real-world
    /// size, rotation and offset, and what it is as stock (lane 10).
    DefineMaterial {
        name: String,
        color: [u8; 4],
        #[serde(default)]
        texture: Option<String>,
        /// Texture width and height, metres (absent: one foot across, at the image's aspect).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        size: Option<[f64; 2]>,
        /// Texture rotation, degrees.
        #[serde(default, skip_serializing_if = "is_zero_f")]
        rotation: f64,
        /// Texture offset along the face, metres.
        #[serde(default, skip_serializing_if = "is_zero2")]
        offset: [f64; 2],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        stock: Option<crate::looks::StockLink>,
    },
    /// Rename a material everywhere it is used.
    RenameMaterial {
        name: String,
        to: String,
    },
    /// Delete a material: what was painted with it shows the default (or what is around it).
    DeleteMaterial {
        name: String,
    },
    /// Put a texture image into the model (PNG or JPEG bytes, base64 in JSON) under
    /// `assets/<name>.png|.jpg`. The same path again replaces the image.
    AddTexture {
        path: String,
        mime: String,
        #[serde(with = "crate::looks::b64::serde_bytes")]
        data: Vec<u8>,
    },
    /// Paint faces of a solid, or solids, groups and instances (their default), with a
    /// material (`None`: take it off). Faces with a material of their own keep it when their
    /// container is painted.
    Paint {
        target: crate::looks::PaintTarget,
        material: Option<String>,
    },
    /// Lane 15: take the paint off what is inside these items (and off solids named
    /// directly), so the material of what holds them shows: every face's own material, every
    /// solid's and mesh's, and nested groups' (not inside component instances, whose parts
    /// belong to the definition: open the component). Texture positions stay. One undo step.
    ClearPaint {
        items: Vec<crate::nodes::Item>,
    },
    /// How the texture sits on one face (`None`: the material's own placement).
    SetFaceUv {
        object: u64,
        face: u64,
        uv: Option<crate::looks::UvXform>,
    },
    /// The document's drawing style (`None`: the default).
    SetStyle {
        style: Option<crate::looks::Style>,
    },
    /// The document's sun and shadows (`None`: the default, off).
    SetShadows {
        shadows: Option<crate::looks::Shadows>,
    },
    /// Paint an object with a material, or take its material off (`None`).
    SetMaterial {
        object: u64,
        material: Option<String>,
    },
    /// Name an object, or clear its name (`None`).
    RenameObject {
        object: u64,
        name: Option<String>,
    },
    /// Delete an imported non-solid mesh.
    DeleteMesh {
        mesh: u64,
    },
    /// Cut list (lane 7): which way a part's grain runs. `Length` is the default and clears
    /// the override. A component instance names its definition. Metadata: allowed anywhere
    /// in the tree, like a name, and it reaches every copy of a component.
    SetGrain {
        part: crate::cutlist::PartRef,
        grain: crate::cutlist::Grain,
    },
    /// Cut list: the stock a part is cut from, in words (`None`: its material's name), and
    /// whether it is sheet goods or solid lumber (`None`: decided from the name and the
    /// thickness).
    SetStock {
        part: crate::cutlist::PartRef,
        #[serde(default)]
        stock: Option<String>,
        #[serde(default)]
        sheet: Option<bool>,
    },
    /// Cut list (lane 15): count a part, or drop it from the totals, the outputs' counts and
    /// the nesting (it stays listed, marked). Metadata, like grain.
    SetInclude {
        part: crate::cutlist::PartRef,
        include: bool,
    },
    /// Lane 15: put each edge-connected piece of non-solid meshes into a group of its own, so
    /// the cut list counts it as a part (loose faces are never parts). `meshes` empty: every
    /// mesh in the open context. One undo step; the geometry does not move or change.
    GroupLoose {
        #[serde(default)]
        meshes: Vec<u64>,
    },
    /// Lane 15: set and clear soft (1), smooth (2) and hidden (4) on edges of solids and
    /// meshes in the open context, each named by its owner and end points (the Eraser:
    /// Ctrl softens and smooths, Shift hides, Ctrl+Shift unsoftens, smooths off and shows).
    /// One undo step for all of them; geometry never changes.
    SetEdgeFlags {
        edges: Vec<crate::edges::EdgeAt>,
        #[serde(default)]
        set: u8,
        #[serde(default)]
        clear: u8,
    },
    /// Lane 15: soften by angle (the Styles pane): on every edge of these solids and meshes
    /// whose two faces turn by at most `angle` degrees (0 to 180), set soft and / or smooth;
    /// every other edge of them loses both. Hidden edges stay hidden. One undo step.
    SoftenEdges {
        #[serde(default)]
        objects: Vec<u64>,
        #[serde(default)]
        meshes: Vec<u64>,
        angle: f64,
        soft: bool,
        smooth: bool,
    },
    /// Nesting (lane 9): add a stock to the document's library, or replace the stock named
    /// `replace` with it (a rename carries the per-material defaults along).
    DefineStock {
        stock: crate::stock::StockDef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        replace: Option<String>,
    },
    /// Nesting: remove a stock from the library (materials that defaulted to it go back to
    /// the automatic choice).
    DeleteStock {
        name: String,
    },
    /// Nesting: which stock a material's parts come from (`None`: chosen automatically).
    SetMaterialStock {
        material: String,
        #[serde(default)]
        stock: Option<String>,
    },
    /// Nesting: the saw's kerf, the trim per edge and the algorithm.
    SetNestSettings {
        settings: crate::stock::NestSettings,
    },
    /// Nesting: back to the built-in stock library and settings.
    ResetStockLibrary,
    /// Section planes (lane 12): a new plane in the open context through `origin`, keeping
    /// the side `normal` points to; active by default (the other planes of its context are
    /// then deactivated).
    AddSection {
        origin: [f64; 3],
        normal: [f64; 3],
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default = "yes", skip_serializing_if = "is_true")]
        active: bool,
        /// Placed hidden (an import's hidden planes).
        #[serde(default, skip_serializing_if = "is_false")]
        hidden: bool,
    },
    /// Put a section plane exactly here (it must be in the open context).
    PlaceSection {
        section: u64,
        origin: [f64; 3],
        normal: [f64; 3],
    },
    /// Keep the other side (the plane must be in the open context).
    FlipSection {
        section: u64,
    },
    /// Activate a section plane (the others of its context are deactivated), or deactivate
    /// it. Allowed anywhere, like the Outliner's flags.
    SetSectionActive {
        section: u64,
        active: bool,
    },
    RenameSection {
        section: u64,
        name: String,
    },
    /// Hide or show a section plane (an active hidden plane still cuts).
    SetSectionHidden {
        section: u64,
        hidden: bool,
    },
    /// Delete a section plane (in the open context).
    DeleteSection {
        section: u64,
    },
    /// Move, turn or mirror section planes (Move / Rotate / Scale; in the open context). A
    /// selection of solids and planes moves as one batch of [`Op::Transform`] and this.
    MoveSections {
        sections: Vec<u64>,
        xform: Xform,
    },
    /// `count` copies of each plane, copy k mapped by `xform` k times (Ctrl with Move or
    /// Rotate, arrays). The copies start inactive.
    CopySections {
        sections: Vec<u64>,
        xform: Xform,
        #[serde(default = "one")]
        count: u32,
    },
    /// Create Group from Slice: the plane's cut outline drawn into a sketch on the plane,
    /// in a new group in the open context (closed loops become faces).
    SliceToGroup {
        section: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    /// Drawing sheets (lane 12, see [`crate::sheets`]): a new sheet at the end.
    AddSheet {
        setup: Box<crate::sheets::SheetSetup>,
    },
    /// A sheet's paper, margins, units, grid, length format, title block and name.
    SetSheet {
        sheet: u64,
        setup: Box<crate::sheets::SheetSetup>,
    },
    DeleteSheet {
        sheet: u64,
    },
    /// Put a sheet at position `to` in the document's order.
    MoveSheet {
        sheet: u64,
        to: u32,
    },
    /// Place an item on a sheet (viewport, dimension, label, table, image, line, rectangle).
    AddSheetItem {
        sheet: u64,
        item: Box<crate::sheets::ItemKind>,
    },
    /// Replace an item (moved, resized or changed); it keeps its id and kind.
    SetSheetItem {
        sheet: u64,
        item: u64,
        kind: Box<crate::sheets::ItemKind>,
    },
    /// Move items on the paper by `by` (metres).
    MoveSheetItems {
        sheet: u64,
        items: Vec<u64>,
        by: [f64; 2],
    },
    /// Delete items (a viewport's dimensions go with it).
    DeleteSheetItems {
        sheet: u64,
        items: Vec<u64>,
    },
    /// Put an item at position `to` in its sheet's drawing order.
    OrderSheetItem {
        sheet: u64,
        item: u64,
        to: u32,
    },
    /// Replace the whole model with a saved one (opening a `.shz`). Validated like any
    /// edit; refused (typed) when the snapshot does not validate.
    Restore {
        snapshot: Box<crate::snapshot::Snapshot>,
    },
    /// Step back one op (document command; see [`crate::document::Document`]).
    Undo,
    /// Step forward again after an undo (document command).
    Redo,
}

impl Op {
    /// Opening or closing a group or component for editing: logged, never an undo step.
    pub fn is_navigation(&self) -> bool {
        matches!(self, Op::SetContext { .. } | Op::LeaveContext)
    }
}

/// Where a stroke goes.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[serde(tag = "on", rename_all = "snake_case")]
pub enum DrawTarget {
    /// Into this sketch (points must lie on its plane).
    Sketch { sketch: u64 },
    /// Onto this plane: into the sketch already lying on it (either facing), or a new one.
    Plane { origin: [f64; 3], normal: [f64; 3] },
    /// Onto the plane of this face of a solid (persistent face id).
    Face { object: u64, face: u64 },
}

/// A stroke from a file (DXF lines and polylines, SVG paths): world points on one plane.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct ImportStroke {
    pub points: Vec<[f64; 3]>,
    #[serde(default)]
    pub closed: bool,
    /// Lane 23: the stroke is one curve in the file (a DXF `CIRCLE` or `ARC`): it comes in
    /// as a curve when its points fit (see [`crate::curve::CurveSpec`]), else as plain edges.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curve: Option<crate::curve::CurveSpec>,
}

/// A material as an op or a file carries it: an sRGB colour with alpha, a texture (a path
/// of an image the model holds, or the file name of one it does not), its real-world size,
/// rotation and offset, and its stock link (see [`crate::looks`]).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct MaterialDef {
    pub name: String,
    pub color: [u8; 4],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub texture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "is_zero_f")]
    pub rotation: f64,
    #[serde(default, skip_serializing_if = "is_zero2")]
    pub offset: [f64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stock: Option<crate::looks::StockLink>,
}

impl MaterialDef {
    /// The material it defines.
    pub fn material(&self) -> Material {
        Material {
            color: self.color,
            texture: self.texture.clone(),
            size: self.size,
            rotation: self.rotation,
            offset: self.offset,
            stock: self.stock.clone(),
        }
    }

    /// As an op or a file writes it.
    pub fn of(name: &str, m: &Material) -> MaterialDef {
        MaterialDef {
            name: name.to_string(),
            color: m.color,
            texture: m.texture.clone(),
            size: m.size,
            rotation: m.rotation,
            offset: m.offset,
            stock: m.stock.clone(),
        }
    }

    /// A plain colour, as before lane 10.
    pub fn plain(name: &str, color: [u8; 4], texture: Option<String>) -> MaterialDef {
        MaterialDef::of(name, &Material::plain(color, texture))
    }
}

pub(crate) fn is_zero_f(x: &f64) -> bool {
    *x == 0.0
}

pub(crate) fn is_zero2(x: &[f64; 2]) -> bool {
    *x == [0.0, 0.0]
}

fn yes() -> bool {
    true
}

fn one() -> u32 {
    1
}

fn is_true(b: &bool) -> bool {
    *b
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// A saved view (SketchUp "scene"): camera in metres, Z up, and the tags it hides.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct SceneDef {
    pub name: String,
    pub eye: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    /// Vertical field of view, degrees.
    #[serde(default = "default_fov")]
    pub fov: f64,
    #[serde(default)]
    pub parallel: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hidden_tags: Vec<String>,
    /// The group or instance open when the scene was saved (public id; `None`: the model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<u64>,
    /// The drawing style saved with the scene (lane 10); activating the scene brings it
    /// back. Absent: the scene leaves the style alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<crate::looks::Style>,
    /// The sun and shadows saved with the scene, likewise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadows: Option<crate::looks::Shadows>,
    /// The active and hidden section planes saved with the scene (lane 12); activating
    /// the scene brings them back. Absent: the scene leaves the sections alone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Box<crate::section::SceneSections>>,
    /// The ground grid shown when the scene is activated (lane 26, issue #6): activating
    /// the scene sets the style's `grid` to it, so a scene saved with the default style
    /// (stored as no style) still brings the grid back. The grid itself lives only in the
    /// style; the scene remembers what to set it to. Absent (older files, imported scenes):
    /// the scene leaves the grid as it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<bool>,
}

fn default_fov() -> f64 {
    35.0
}

/// A component definition in an import: its own geometry (in its own coordinates) and the
/// groups and instances nested in it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct ImportDefinition {
    pub name: String,
    #[serde(default)]
    pub items: Vec<ImportItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<ImportNode>,
    /// Section planes drawn in the definition (lane 12), in its own coordinates: each copy
    /// placed at the top of the import gets its own, inactive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<ImportSection>,
}

/// A section plane in an import (lane 12): a point, the direction of the side it keeps, a
/// name, hidden or not. Imported planes come in inactive.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct ImportSection {
    pub origin: [f64; 3],
    pub normal: [f64; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub hidden: bool,
}

/// A group (geometry in the enclosing frame's coordinates, plus nested nodes) or an instance
/// of an import definition (index into `definitions`, placed by `xform` in the enclosing
/// frame).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct ImportNode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    /// Section planes drawn in the group (lane 12), in the enclosing frame (world for a
    /// group at the top of the import); they come in inactive.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<ImportSection>,
    #[serde(flatten)]
    pub kind: ImportNodeKind,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImportNodeKind {
    Group {
        #[serde(default)]
        items: Vec<ImportItem>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        children: Vec<ImportNode>,
    },
    Instance {
        definition: usize,
        xform: Xform,
    },
}

/// What an op did, with the ids the UI needs next.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OpOutcome {
    SketchCreated {
        sketch: u64,
    },
    /// A sketch edit. `regions`: the regions it drew (inside a rectangle) or touched.
    SketchEdited {
        regions: Vec<u64>,
    },
    ObjectCreated {
        object: u64,
    },
    FaceMoved,
    /// A face was split; the two pieces' persistent ids (the first keeps the old id).
    FaceSplit {
        faces: Vec<u64>,
    },
    /// A loop was imprinted; the new sub-face's persistent id.
    Imprinted {
        face: u64,
    },
    ObjectDeleted,
    GuideAdded {
        guide: u64,
    },
    DimensionAdded {
        dimension: u64,
    },
    AnnotationErased,
    /// A stroke was drawn: into `sketch` (the regions it bounds), or on a solid (the
    /// persistent ids of the faces it cut or made).
    Drawn {
        sketch: Option<u64>,
        regions: Vec<u64>,
        faces: Vec<u64>,
        /// A closed stroke: the region or face it now encloses (what Push/Pull acts on next).
        inside: Option<u64>,
        /// Lane 23: the curve the stroke became in the sketch (the first piece, when it was
        /// drawn across other edges and split), absent for plain strokes.
        #[serde(skip_serializing_if = "Option::is_none")]
        curve: Option<u64>,
    },
    /// An edge or face was erased; the merged face's persistent id.
    FacesMerged {
        face: u64,
    },
    /// What each op of a batch did, in order.
    Batch {
        outcomes: Vec<OpOutcome>,
    },
    GroupCreated {
        group: u64,
    },
    GroupDeleted,
    /// The editing context changed (`None`: the model).
    ContextChanged {
        context: Option<u64>,
    },
    ComponentCreated {
        instance: u64,
        definition: u64,
    },
    MadeUnique {
        definition: u64,
    },
    /// A node was exploded: the solids and nodes that were directly in it.
    Exploded {
        objects: Vec<u64>,
        nodes: Vec<u64>,
    },
    /// sketchcraft E-42: `merged` faces of `object` went into their neighbours.
    AutoMerged {
        object: u64,
        merged: u64,
    },
    /// sketchcraft E-40: a mesh rebuilt as the solid `object`; `notes` say how.
    /// sketchcraft E-50: points of a solid removed (DissolvePoint: 1; CleanPoints: how many).
    PointsCleaned {
        object: u64,
        removed: u64,
    },
    /// sketchcraft E-50: points of a sketch removed (CleanSketchPoints).
    SketchPointsCleaned {
        sketch: u64,
        removed: u64,
    },
    MeshUnified {
        object: u64,
        notes: Vec<String>,
    },
    /// sketchcraft E-28: a solid became `face_count` plain faces.
    SolidExploded {
        face_count: u64,
    },
    NodeDeleted,
    VisibilitySet,
    LockSet,
    SceneSet,
    SceneActivated {
        context: Option<u64>,
    },
    TagSet,
    Undone,
    Redone,
    /// Follow Me on a face: the solid grew along the path, or (`cut`) the sweep was cut out
    /// of it; `objects` is the solid, then any pieces it came apart into.
    Swept {
        objects: Vec<u64>,
        cut: bool,
    },
    /// Solids (and groups and instances) were moved, turned or scaled (the same ids).
    Transformed {
        objects: Vec<u64>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        nodes: Vec<u64>,
    },
    /// New solids (and groups and instances) from a copy or an array, in creation order.
    Copied {
        objects: Vec<u64>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        nodes: Vec<u64>,
    },
    /// Part of a solid moved; the persistent ids of the faces that changed (folds included).
    GeometryMoved {
        faces: Vec<u64>,
    },
    /// Sketch geometry moved, turned or scaled (lane 22): the sketch it is in now (another
    /// one when a whole shape left its plane) and the live regions its edges now bound, as
    /// (sketch, region) public id pairs.
    SketchMoved {
        sketch: u64,
        regions: Vec<[u64; 2]>,
        /// Lane 23: the curves the op's `curves` are now, in its order, as (sketch, curve)
        /// pairs (the same ids within the plane; new ones in the sketch a shape was lifted
        /// to; a curve the move broke up is its first piece, one that went is absent).
        #[serde(skip_serializing_if = "Vec::is_empty")]
        curves: Vec<[u64; 2]>,
        /// Lane 43 (#7): the further pieces of each of the op's `curves` that the move broke
        /// up, within its plane or on the plane it was lifted to, as (curve in the op, sketch,
        /// piece) triples; with its first piece in `curves`, all it became. Empty when no
        /// curve broke up.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pieces: Vec<[u64; 3]>,
    },
    /// Sketch geometry was copied (or arrayed): the live regions the copies bound, as
    /// (sketch, region) public id pairs.
    SketchCopied {
        regions: Vec<[u64; 2]>,
        /// Lane 23: the curves the copies made, as (sketch, curve) pairs.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        curves: Vec<[u64; 2]>,
    },
    /// Lane 23: a curve was exploded into `edges` plain edges.
    CurveExploded {
        edges: u64,
    },
    /// A push/pull (or a sweep) cut a solid into pieces: each piece is its own solid. The
    /// first id is the original solid (the piece holding its smallest face id).
    Severed {
        objects: Vec<u64>,
    },
    /// A file was imported: the solids, non-solid meshes, groups and sketches it made, and
    /// plain-language notes (what was healed, what stayed a mesh and why, what was skipped).
    Imported {
        objects: Vec<u64>,
        meshes: Vec<u64>,
        groups: Vec<u64>,
        sketches: Vec<u64>,
        notes: Vec<String>,
    },
    MaterialSet,
    /// Lane 15: paint taken off `faces` faces and `items` solids, meshes and groups.
    PaintCleared {
        cleared_faces: usize,
        cleared_items: usize,
    },
    /// A texture image went into the model under `path`.
    TextureAdded {
        path: String,
    },
    /// The style or the shadows changed.
    StyleSet,
    Renamed,
    MeshDeleted,
    /// A part's grain or stock was set (cut list).
    PartSet,
    /// Loose meshes went into groups, one per edge-connected piece (lane 15).
    LooseGrouped {
        groups: Vec<u64>,
    },
    /// Soft, smooth or hidden flags changed on `changed` edges (lane 15).
    EdgesSet {
        changed: usize,
    },
    /// The stock library or the nesting settings changed.
    StockSet,
    /// A saved model was restored.
    Restored {
        objects: usize,
        meshes: usize,
        sketches: usize,
    },
    /// A sheet was added (lane 12).
    SheetAdded {
        sheet: u64,
    },
    /// An item was placed on a sheet.
    SheetItemAdded {
        sheet: u64,
        item: u64,
    },
    /// A sheet or its items changed.
    SheetSet,
    /// A section plane was added (lane 12).
    SectionAdded {
        section: u64,
    },
    /// A section plane was placed, flipped, (de)activated, renamed, hidden or shown.
    SectionSet,
    SectionDeleted,
    /// Section planes moved (the same ids) or copied (the new ids, in creation order).
    SectionsMoved {
        sections: Vec<u64>,
    },
    SectionsCopied {
        sections: Vec<u64>,
    },
    /// Create Group from Slice: the new group and the sketch in it.
    Sliced {
        group: u64,
        sketch: u64,
    },
    /// The pushed solid flattened to nothing and was removed. When the flat footprint lay
    /// on the solid's source sketch it went back there as a region.
    ObjectCollapsed {
        regions: Vec<u64>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct Model {
    pub(crate) sketches: SlotMap<SketchId, Sketch>,
    pub(crate) sketch_order: Vec<SketchId>,
    pub(crate) objects: SlotMap<ObjectId, Object>,
    pub(crate) object_order: Vec<ObjectId>,
    /// Groups and component instances (see [`crate::nodes`]).
    pub(crate) nodes: SlotMap<NodeId, Node>,
    /// Component definitions.
    pub(crate) defs: SlotMap<DefId, Definition>,
    /// The open group or instance (`None`: the model).
    pub(crate) context: Option<NodeId>,
    /// Tag name -> visible. Every tag an object uses is listed here.
    pub(crate) tags: BTreeMap<String, bool>,
    /// Tag name -> sRGB colour (the Tags pane), for tags that have one.
    pub(crate) tag_colors: BTreeMap<String, [u8; 3]>,
    /// What the op in progress changed (propagation; transient).
    pub(crate) dirty: Dirty,
    pub(crate) guides: SlotMap<GuideId, Guide>,
    pub(crate) dimensions: SlotMap<DimensionId, Dimension>,
    /// Imported geometry that is not a valid solid (see [`crate::meshbuild`]).
    pub(crate) meshes: SlotMap<MeshId, LooseMesh>,
    pub(crate) mesh_order: Vec<MeshId>,
    /// Material name -> definition.
    pub(crate) materials: BTreeMap<String, Material>,
    /// Texture images by path (`assets/...`), lane 10.
    pub(crate) textures: BTreeMap<String, crate::looks::TextureImage>,
    /// The drawing style (`None`: the default) and the sun and shadows (`None`: off).
    pub(crate) style: Option<crate::looks::Style>,
    pub(crate) shadows: Option<crate::looks::Shadows>,
    /// Saved views, in order.
    pub(crate) scenes: Vec<SceneDef>,
    /// Last display number handed to an object (see [`Object::number`]).
    pub(crate) last_number: u64,
    /// The stock library and nesting settings (lane 9); `None`: the built-in library.
    pub(crate) stock: Option<crate::stock::StockLibrary>,
    /// Section planes (lane 12), see [`crate::section`].
    pub(crate) sections: SlotMap<crate::ids::SectionId, crate::section::SectionPlane>,
    /// Drawing sheets and their order (lane 12), see [`crate::sheets`].
    pub(crate) sheets: SlotMap<crate::ids::SheetId, crate::sheets::Sheet>,
    pub(crate) sheet_order: Vec<crate::ids::SheetId>,
    /// Lane 53 (#65; transient, like `dirty`): the op in progress is the last before the
    /// edit is held to the limits (the op itself, or the last op of its batch), so a stroke
    /// certain to take its sketch past [`crate::limits::MAX_SKETCH_EDGES`] need not be
    /// inserted: nothing after it can take edges away again.
    pub(crate) last_op: bool,
    /// Lane 53 (#65; transient): such a stroke, not inserted: its sketch and the edges the
    /// sketch would hold with it. The limit check after the edit reads that count for that
    /// sketch ([`crate::limits::check_model_with`]), so the edit is refused exactly as it is
    /// with the stroke inserted, by the first thing over a limit in the check's own order.
    pub(crate) unbuilt_stroke: Option<(SketchId, usize)>,
}

/// A material: sRGB colour with alpha; a texture (a path in the model's texture table, or a
/// file name kept as a reference) with its real-world size (metres; `None`: one foot across
/// at the image's aspect), rotation (degrees) and offset (metres); and its stock link.
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub color: [u8; 4],
    pub texture: Option<String>,
    pub size: Option<[f64; 2]>,
    pub rotation: f64,
    pub offset: [f64; 2],
    pub stock: Option<crate::looks::StockLink>,
}

impl Material {
    /// A plain colour (and maybe a texture reference), as before lane 10.
    pub fn plain(color: [u8; 4], texture: Option<String>) -> Material {
        Material {
            color,
            texture,
            size: None,
            rotation: 0.0,
            offset: [0.0, 0.0],
            stock: None,
        }
    }

    /// Nothing beyond the colour and the texture name (hashed as before lane 10).
    pub fn is_plain(&self) -> bool {
        self.size.is_none()
            && self.rotation == 0.0
            && self.offset == [0.0, 0.0]
            && self.stock.is_none()
    }
}

/// A guide line (unit `dir`) or a guide point (`dir` zero).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    pub point: DVec3,
    pub dir: DVec3,
}

/// A linear dimension: the measured points and where its line is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dimension {
    pub a: DVec3,
    pub b: DVec3,
    pub offset: DVec3,
}

impl Dimension {
    /// The measured length, metres.
    pub fn length(&self) -> f64 {
        self.a.distance(self.b)
    }
}

fn finite3(p: &[f64; 3]) -> Option<DVec3> {
    let v = DVec3::from_array(*p);
    (v.is_finite() && v.abs().max_element() <= crate::tolerance::MAX_COORD).then_some(v)
}

#[cfg(test)]
thread_local! {
    /// Tests: how many edits were done again with a left-out stroke drawn.
    static REDONE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Tests: leave out every stroke whose count is certain, over the limit or not (to reach
    /// the path that does the edit again with it drawn).
    static FORCE_LEAVE_OUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Lane 53 (#66): for a push/pull, extrude or offset at the shortest distance
/// ([`crate::tolerance::at_min_distance`]), the same op at twice the minimum the same way
/// and the refusal it gets when only the shortness stops it. `None` for any other op.
fn at_the_minimum(op: &Op) -> Option<(Op, KernelError)> {
    use crate::tolerance::{MIN_EXTRUDE, at_min_distance};
    let longer = |d: f64| 2.0 * MIN_EXTRUDE * d.signum();
    let short = KernelError::Push(PushError::TooShortHere);
    match op {
        Op::Extrude {
            sketch,
            region,
            distance,
        } if at_min_distance(*distance) => Some((
            Op::Extrude {
                sketch: *sketch,
                region: *region,
                distance: longer(*distance),
            },
            short,
        )),
        Op::PushPull {
            object,
            face,
            distance,
        } if at_min_distance(*distance) => Some((
            Op::PushPull {
                object: *object,
                face: *face,
                distance: longer(*distance),
            },
            short,
        )),
        Op::PushPullNewFace {
            object,
            face,
            distance,
        } if at_min_distance(*distance) => Some((
            Op::PushPullNewFace {
                object: *object,
                face: *face,
                distance: longer(*distance),
            },
            short,
        )),
        Op::Offset { source, distance } if at_min_distance(*distance) => Some((
            Op::Offset {
                source: source.clone(),
                distance: longer(*distance),
            },
            KernelError::Offset(OffsetError::TooShortHere),
        )),
        _ => None,
    }
}

/// Lane 53 (#66): the failures the shortest distance can cause, and only those: the
/// validator's zero-length edges and self-contact (corners rounded to the merge distance),
/// push/pull's degenerate or invalid result and a B-rep that would not build, and a face
/// edit's loop meeting, leaving or touching the face's own edges. Any other failure at the
/// minimum keeps its own message.
fn about_the_minimum(e: &KernelError) -> bool {
    match e {
        KernelError::Topology(t) => {
            !t.violations.is_empty()
                && t.violations.iter().all(|v| {
                    matches!(
                        v,
                        Violation::ZeroLengthEdge(_) | Violation::SelfIntersectingSolid(_)
                    )
                })
        }
        KernelError::Push(PushError::Boolean(_) | PushError::Build(_)) => true,
        KernelError::FaceEdit(f) => matches!(
            f,
            FaceEditError::CrossesBoundary
                | FaceEditError::NotInside
                | FaceEditError::NotOnBoundary
                | FaceEditError::Dangling
                | FaceEditError::TooClose
        ),
        _ => false,
    }
}

/// A group as lane 2 saw it: its name and the solids directly in it (public ids).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub name: String,
    pub objects: Vec<u64>,
}

/// What an import brings in, component copies included: (faces, points, items, nodes),
/// saturating.
type ImportSize = (usize, usize, usize, usize);

fn add_size(a: ImportSize, b: ImportSize) -> ImportSize {
    (
        a.0.saturating_add(b.0),
        a.1.saturating_add(b.1),
        a.2.saturating_add(b.2),
        a.3.saturating_add(b.3),
    )
}

fn items_size(items: &[ImportItem]) -> ImportSize {
    items.iter().fold((0, 0, 0, 0), |s, it| {
        add_size(
            s,
            (it.faces.len() + it.lines.len(), it.positions.len(), 1, 0),
        )
    })
}

fn nodes_size(nodes: &[ImportNode], defs: &[ImportSize]) -> ImportSize {
    let mut s = (0, 0, 0, 0);
    for n in nodes {
        s = add_size(s, (0, 0, 0, 1));
        s = add_size(
            s,
            match &n.kind {
                ImportNodeKind::Group { items, children } => {
                    add_size(items_size(items), nodes_size(children, defs))
                }
                // A definition not defined yet is refused when the import is built.
                ImportNodeKind::Instance { definition, .. } => {
                    defs.get(*definition).copied().unwrap_or_default()
                }
            },
        );
    }
    s
}

/// Lane 16: refuse an import over the import limits before building any of it. A
/// definition is sized once (it can only use definitions listed before it), so a chain of
/// definitions each holding two copies of the next is counted, not built.
fn import_budget(
    items: &[ImportItem],
    strokes: &[ImportStroke],
    definitions: &[ImportDefinition],
    nodes: &[ImportNode],
) -> Result<(), crate::limits::LimitError> {
    use crate::limits as l;
    let mut defs: Vec<ImportSize> = Vec::with_capacity(definitions.len());
    for d in definitions {
        let s = add_size(items_size(&d.items), nodes_size(&d.children, &defs));
        defs.push(s);
    }
    let (faces, points, objects, made_nodes) =
        add_size(items_size(items), nodes_size(nodes, &defs));
    l::import("faces", faces, l::MAX_IMPORT_FACES)?;
    l::import("points", points, l::MAX_IMPORT_POINTS)?;
    l::import("solids and meshes", objects, l::MAX_IMPORT_ITEMS)?;
    l::import("groups and component copies", made_nodes, l::MAX_NODES)?;
    l::import(
        "component definitions",
        definitions.len(),
        l::MAX_DEFINITIONS,
    )?;
    l::import("lines and curves", strokes.len(), l::MAX_IMPORT_STROKES)?;
    let stroke_points = strokes
        .iter()
        .fold(0usize, |n, s| n.saturating_add(s.points.len()));
    l::import(
        "points of lines and curves",
        stroke_points,
        l::MAX_IMPORT_STROKE_POINTS,
    )?;
    Ok(())
}

/// A usable name from a file: trimmed, at most 64 characters, `fallback` when empty.
pub(crate) fn clip_name(n: &str, fallback: &str) -> String {
    let t: String = n.trim().chars().take(64).collect();
    let t = t.trim().to_string();
    if t.is_empty() {
        fallback.to_string()
    } else {
        t
    }
}

fn check_name(n: &str) -> Result<(), DocumentError> {
    if n.trim().is_empty() || n.chars().count() > 64 {
        Err(DocumentError::BadName)
    } else {
        Ok(())
    }
}

/// Lane 23: give a solid just extruded from region `r` of sketch `s` the rims of the
/// region's curves (every curve whose every edge bounds the region): one on the base and
/// one on the cap each, keyed by the cap's face id (1 the base, 2 the cap) and the side
/// wall each curve edge raised ([`Object::extrude`] numbers the walls ring by ring, edge by
/// edge, from 3). The walls between two segments of a circle or an arc are soft and smooth
/// (SketchUp: a pulled circle is a smooth cylinder); a polygon's and a polyline's stay hard.
fn rim_curves(obj: &mut Object, s: &Sketch, r: &crate::sketch::Region) {
    use crate::curve::{CurveKind, RimCurve};
    if s.curve_count() == 0 {
        return;
    }
    let mut side: BTreeMap<(crate::ids::SVertexId, crate::ids::SVertexId), u64> = BTreeMap::new();
    let mut next = 3u64;
    for ring in std::iter::once(&r.outer).chain(r.holes.iter()) {
        let n = ring.len();
        for k in 0..n {
            let (a, b) = (ring[k], ring[(k + 1) % n]);
            side.insert((a.min(b), a.max(b)), next);
            next += 1;
        }
    }
    let mut id = 1u64;
    for (_, c) in s.curves() {
        let walls: Option<Vec<u64>> = c
            .pairs()
            .map(|(a, b)| side.get(&(a.min(b), a.max(b))).copied())
            .collect();
        let Some(walls) = walls else { continue };
        for cap in [1u64, 2] {
            obj.curves.insert(
                id,
                RimCurve {
                    kind: c.kind,
                    closed: c.closed,
                    segments: c.segments,
                    keys: walls.iter().map(|&w| crate::edges::pair(cap, w)).collect(),
                },
            );
            id += 1;
        }
        if matches!(c.kind, CurveKind::Circle | CurveKind::Arc) {
            let n = walls.len();
            let m = if c.closed { n } else { n - 1 };
            for i in 0..m {
                let key = crate::edges::pair(walls[i], walls[(i + 1) % n]);
                obj.set_edge_flags(key, crate::edges::SOFT | crate::edges::SMOOTH, 0);
            }
        }
    }
    obj.settle_edges();
}

/// Lane 23: a round curve an offset starts from ([`Model::offset_rounds`]).
#[derive(Debug, Clone, Copy, PartialEq)]
struct OffsetRound {
    kind: crate::curve::CurveKind,
    centre: DVec3,
    points: usize,
    closed: bool,
    segments: u32,
}

/// Lane 23: a stroke's curve, checked ([`stroke_curve`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct StrokeCurve {
    pub kind: crate::curve::CurveKind,
    /// World centre (round kinds).
    pub centre: Option<DVec3>,
    /// Segments as drawn: a circle's or polygon's points, an arc's or polyline's edges.
    pub segments: u32,
}

/// Lane 23: check what an op says a stroke is: a circle or a polygon closed with three
/// points or more, an arc open with two or more, all at one distance from the given centre
/// (within two merge distances and rounding; the tool computes them on the circle), a
/// polyline any stroke.
pub(crate) fn stroke_curve(
    c: &crate::curve::CurveSpec,
    pts: &[DVec3],
    closed: bool,
) -> Result<StrokeCurve, SketchError> {
    let kind = c.kind;
    let bad = |why: &'static str| SketchError::NotACurve {
        kind: kind.word(),
        why,
    };
    if let Some(want) = kind.closes()
        && want != closed
    {
        return Err(bad(if want {
            "a circle or a polygon is a closed stroke"
        } else {
            "an arc is an open stroke"
        }));
    }
    let need = if closed { 3 } else { 2 };
    if pts.len() < need {
        return Err(bad("too few points"));
    }
    let n = pts.len() as u32;
    let segments = if closed { n } else { n - 1 };
    if !kind.is_round() {
        return Ok(StrokeCurve {
            kind,
            centre: None,
            segments,
        });
    }
    let centre = c
        .centre
        .map(DVec3::from_array)
        .filter(|v| v.is_finite())
        .ok_or_else(|| bad("it has no centre"))?;
    let (lo, hi) = pts.iter().fold((f64::INFINITY, 0.0f64), |(lo, hi), p| {
        let d = p.distance(centre);
        (lo.min(d), hi.max(d))
    });
    let slack = 2.0 * crate::tolerance::SKETCH_MERGE + 1e-9 * (centre.length() + hi);
    if !(hi.is_finite() && lo > 0.0 && hi - lo <= slack) {
        return Err(bad("its points are not on one circle about its centre"));
    }
    Ok(StrokeCurve {
        kind,
        centre: Some(centre),
        segments,
    })
}

/// Counts for the UI and tests.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct ModelStats {
    pub sketches: usize,
    pub regions: usize,
    pub objects: usize,
    pub faces: usize,
    pub edges: usize,
    pub vertices: usize,
    pub holes: usize,
    pub shells: usize,
    /// Every object watertight (vacuously true with no objects).
    pub watertight: bool,
    pub euler: i64,
    pub genus: i64,
    pub volume_m3: f64,
    pub hash: String,
}

impl Model {
    pub fn new() -> Model {
        Model::default()
    }

    /// A model with one sketch on the ground plane (public id 1), as the UI starts.
    pub fn with_ground_sketch() -> Model {
        let mut m = Model::new();
        m.apply(&Op::NewSketch {
            origin: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
        })
        .expect("ground sketch");
        m
    }

    /// Sketches in creation order, with their public ids.
    pub fn sketches(&self) -> Vec<(u64, &Sketch)> {
        self.sketch_order
            .iter()
            .map(|&k| (to_public(k), &self.sketches[k]))
            .collect()
    }

    /// Objects in creation order, with their public ids.
    pub fn objects(&self) -> Vec<(u64, &Object)> {
        self.object_order
            .iter()
            .map(|&k| (to_public(k), &self.objects[k]))
            .collect()
    }

    pub fn sketch(&self, id: u64) -> Option<&Sketch> {
        self.sketches.get(from_public::<SketchId>(id))
    }

    pub fn object(&self, id: u64) -> Option<&Object> {
        self.objects.get(from_public::<ObjectId>(id))
    }

    /// Public ids of the objects, in creation order.
    pub fn object_ids(&self) -> Vec<u64> {
        self.object_order.iter().map(|&k| to_public(k)).collect()
    }

    /// Guides with their public ids, in id order.
    pub fn guides(&self) -> Vec<(u64, &Guide)> {
        self.guides.iter().map(|(k, g)| (to_public(k), g)).collect()
    }

    /// Dimensions with their public ids, in id order.
    pub fn dimensions(&self) -> Vec<(u64, &Dimension)> {
        self.dimensions
            .iter()
            .map(|(k, d)| (to_public(k), d))
            .collect()
    }

    /// Groups (not instances) with their public ids, in id order: name ("Group" when it
    /// has none) and the solids directly in each.
    pub fn groups(&self) -> Vec<(u64, Group)> {
        self.nodes
            .iter()
            .filter(|(_, n)| n.kind == NodeKind::Group)
            .map(|(k, n)| {
                (
                    to_public(k),
                    Group {
                        name: n.name.clone().unwrap_or_else(|| "Group".into()),
                        objects: self
                            .object_order
                            .iter()
                            .filter(|&&o| self.objects[o].member.parent == Some(k))
                            .map(|&o| to_public(o))
                            .collect(),
                    },
                )
            })
            .collect()
    }

    /// Anything lane 5 added beyond lane 2's flat, named groups of solids: instances,
    /// definitions, nested or non-default nodes, hidden / locked items, sketches and meshes
    /// in groups, the editing context, tag colours, scene contexts. Hashed only when
    /// present, so older documents keep their hashes.
    pub(crate) fn has_structure(&self) -> bool {
        !self.defs.is_empty()
            || self.context.is_some()
            || !self.tag_colors.is_empty()
            || self.nodes.values().any(|n| {
                n.kind != NodeKind::Group
                    || n.name.is_none()
                    || n.tag.is_some()
                    || n.member != crate::nodes::Membership::default()
                    || n.xform != Xform::IDENTITY
                    || !n.part.is_default()
            })
            || self
                .objects
                .values()
                .any(|o| o.member.hidden || o.member.locked || o.member.slot.is_some())
            || self
                .meshes
                .values()
                .any(|m| m.member != crate::nodes::Membership::default())
            || self.sketches.values().any(|s| s.parent.is_some())
            || self.scenes.iter().any(|s| s.context.is_some())
    }

    /// Tags and their visibility, by name.
    pub fn tags(&self) -> &BTreeMap<String, bool> {
        &self.tags
    }

    /// True when the object is shown: not hidden, untagged or on a visible tag, and every
    /// group and instance around it shown.
    pub fn is_visible(&self, object: u64) -> bool {
        self.object(object)
            .is_some_and(|o| self.member_shown(&o.member, o.tag()))
    }

    /// Imported non-solid meshes in import order, with their public ids.
    pub fn meshes(&self) -> Vec<(u64, &LooseMesh)> {
        self.mesh_order
            .iter()
            .map(|&k| (to_public(k), &self.meshes[k]))
            .collect()
    }

    pub fn mesh(&self, id: u64) -> Option<&LooseMesh> {
        self.meshes.get(from_public::<MeshId>(id))
    }

    /// True when the mesh is shown (see [`Model::is_visible`]).
    pub fn is_mesh_visible(&self, mesh: u64) -> bool {
        self.mesh(mesh)
            .is_some_and(|m| self.member_shown(&m.member, m.tag.as_deref()))
    }

    /// Materials by name.
    pub fn materials(&self) -> &BTreeMap<String, Material> {
        &self.materials
    }

    /// Saved views, in order.
    pub fn scenes(&self) -> &[SceneDef] {
        &self.scenes
    }

    /// The stock library and nesting settings (the built-in ones until someone edits them).
    pub fn stock_library(&self) -> std::borrow::Cow<'_, crate::stock::StockLibrary> {
        match &self.stock {
            Some(l) => std::borrow::Cow::Borrowed(l),
            None => std::borrow::Cow::Owned(crate::stock::StockLibrary::builtin()),
        }
    }

    /// The library as stored: `None` when it is the built-in one.
    pub fn stored_stock_library(&self) -> Option<&crate::stock::StockLibrary> {
        self.stock.as_ref()
    }

    /// Edit the library; a library edited back to the built-in one is stored as `None`.
    pub(crate) fn edit_stock(
        &mut self,
        f: impl FnOnce(&mut crate::stock::StockLibrary) -> Result<(), crate::error::StockError>,
    ) -> Result<OpOutcome, KernelError> {
        let builtin = crate::stock::StockLibrary::builtin();
        let mut lib = self.stock.clone().unwrap_or_else(|| builtin.clone());
        f(&mut lib)?;
        lib.check()?;
        self.stock = (lib != builtin).then_some(lib);
        Ok(OpOutcome::StockSet)
    }

    /// Public ids of the sketches, in creation order.
    pub fn sketch_ids(&self) -> Vec<u64> {
        self.sketch_order.iter().map(|&k| to_public(k)).collect()
    }

    /// Apply an op transactionally.
    pub fn apply(&mut self, op: &Op) -> Result<OpOutcome, KernelError> {
        let (next, outcome) = self.applied(op)?;
        *self = next;
        Ok(outcome)
    }

    /// The model `op` makes of this one, validated, and the op's outcome; this model is
    /// left as it is (what [`crate::Document`] keeps as the undo snapshot). Lane 16: the
    /// result is also held to the model-wide limits ([`crate::limits`]).
    pub fn applied(&self, op: &Op) -> Result<(Model, OpOutcome), KernelError> {
        let r = self.applied_as_given(op);
        // Lane 53 (#66): a push/pull, extrude or offset at the shortest distance can place
        // corners that round to a wall or a gap no longer than the merge distance, which the
        // kernel cannot accept (a kernel bug from the validator, or a refusal that names
        // degenerate geometry or a path crossing the face's boundary). When that is how it
        // failed and the same op twice as long goes through, the distance is too short here,
        // and it is refused as such ([`about_the_minimum`]; inside a batch,
        // [`Self::apply_in_batch`]). Anything that went through still does, unchanged.
        if let Err(e) = &r
            && let Some((longer, too_short)) = at_the_minimum(op)
            && *e != too_short
            && about_the_minimum(e)
            && self.applied_as_given(&longer).is_ok()
        {
            return Err(too_short);
        }
        r
    }

    fn applied_as_given(&self, op: &Op) -> Result<(Model, OpOutcome), KernelError> {
        self.applied_by(op, true)
    }

    /// [`Self::applied_as_given`]; with `early`, a stroke certain to break the sketch edge
    /// limit is not inserted (`unbuilt_stroke`) and the limit check refuses the edit.
    fn applied_by(&self, op: &Op, early: bool) -> Result<(Model, OpOutcome), KernelError> {
        let mut next = self.clone();
        next.dirty = Dirty::default();
        next.last_op = early;
        let outcome = next.apply_in_place(op)?;
        next.last_op = false;
        let unbuilt = next.unbuilt_stroke.take();
        next.tidy()?;
        crate::limits::check_model_with(&next, unbuilt)?;
        if unbuilt.is_some() {
            // Unreachable as the code stands: a stroke is left out only when its count is
            // over the limit, so the check above refuses the edit. Kept so that a model whose
            // stroke was left out is never committed whatever changes: the edit is done again
            // with the stroke inserted. (`tests::the_redo_with_the_stroke_drawn` forces it.)
            #[cfg(test)]
            REDONE.with(|r| r.set(r.get() + 1));
            return self.applied_by(op, false);
        }
        next.validate()?;
        Ok((next, outcome))
    }

    /// Bytes this model holds that `after` does not share (estimated): what an undo
    /// snapshot of it costs while `after` follows it (lane 16, [`crate::document::UNDO_BYTES`]).
    pub(crate) fn history_cost(&self, after: &Model) -> usize {
        let mut n = 16 * 1024;
        for (k, o) in &self.objects {
            n += 512 + o.paint.len() * 96 + o.edge_flags.len() * 48;
            if !after
                .objects
                .get(k)
                .is_some_and(|a| a.brep.shares_with(&o.brep))
            {
                n += o.brep.approx_bytes();
            }
        }
        for (k, m) in &self.meshes {
            n += 256;
            if !after.meshes.get(k).is_some_and(|a| a.shares_with(m)) {
                n += m.approx_bytes();
            }
        }
        for (_, s) in &self.sketches {
            n += s.approx_bytes();
        }
        for (k, d) in &self.defs {
            let other = after.defs.get(k);
            for (slot, mem) in &d.members {
                n += 256;
                let theirs = other.and_then(|o| o.members.get(slot)).map(|m| &m.kind);
                n += match (&mem.kind, theirs) {
                    (
                        crate::nodes::MemberKind::Solid(a),
                        Some(crate::nodes::MemberKind::Solid(b)),
                    ) if a.brep.shares_with(&b.brep) => 0,
                    (crate::nodes::MemberKind::Solid(a), _) => a.brep.approx_bytes(),
                    (
                        crate::nodes::MemberKind::Mesh(a),
                        Some(crate::nodes::MemberKind::Mesh(b)),
                    ) if a.shares_with(b) => 0,
                    (crate::nodes::MemberKind::Mesh(a), _) => a.approx_bytes(),
                    _ => 0,
                };
            }
        }
        n += (self.nodes.len() + self.guides.len() + self.dimensions.len()) * 256;
        n
    }

    /// Replay a sequence from an empty model, stopping at the first error.
    pub fn replay(ops: &[Op]) -> Result<Model, (usize, KernelError)> {
        let mut m = Model::new();
        for (i, op) in ops.iter().enumerate() {
            m.apply(op).map_err(|e| (i, e))?;
        }
        Ok(m)
    }

    fn sketch_key(&self, id: u64) -> Result<SketchId, SketchError> {
        let k = from_public::<SketchId>(id);
        if self.sketches.contains_key(k) {
            Ok(k)
        } else {
            Err(SketchError::NoSuchSketch(id))
        }
    }

    fn object_key(&self, id: u64) -> Result<ObjectId, PushError> {
        let k = from_public::<ObjectId>(id);
        if self.objects.contains_key(k) {
            Ok(k)
        } else {
            Err(PushError::NoSuchObject(id))
        }
    }

    fn edited(regions: Vec<RegionId>) -> OpOutcome {
        OpOutcome::SketchEdited {
            regions: regions.into_iter().map(to_public).collect(),
        }
    }

    /// Lane 53 (#66): one op of a batch, as [`Self::applied`] takes one op: a push/pull,
    /// extrude or offset at the shortest distance that fails for the reasons the minimum can
    /// cause ([`about_the_minimum`]) while the same op twice as long would go through (on the
    /// model as it stands before it, to the end: tidied, within the limits, valid) is refused
    /// as too short here. The batch is refused either way, so nothing of it is kept. (A
    /// failure the validator finds only when the whole batch is done is not traced to one op:
    /// it stays a kernel bug.)
    fn apply_in_batch(&mut self, op: &Op) -> Result<OpOutcome, KernelError> {
        let Some((longer, too_short)) = at_the_minimum(op) else {
            return self.apply_in_place(op);
        };
        let before = self.clone();
        match self.apply_in_place(op) {
            Err(e)
                if e != too_short && about_the_minimum(&e) && before.would_go_through(&longer) =>
            {
                Err(too_short)
            }
            // Its result checked at once, as the edit's would be: a solid the validator
            // refuses for the minimum's reasons is refused here as too short, not left for
            // the batch's last check to call a kernel bug.
            Ok(out) => match self.validate().map_err(KernelError::from) {
                Err(e) if about_the_minimum(&e) && before.would_go_through(&longer) => {
                    Err(too_short)
                }
                _ => Ok(out),
            },
            r => r,
        }
    }

    /// Lane 53 (#66): whether `op` on a copy of this model goes through to the end.
    fn would_go_through(&self, op: &Op) -> bool {
        let mut p = self.clone();
        p.last_op = false;
        p.apply_in_place(op).is_ok()
            && p.tidy().is_ok()
            && crate::limits::check_model(&p).is_ok()
            && p.validate().is_ok()
    }

    /// One op, then propagation of what it changed into component definitions.
    fn apply_in_place(&mut self, op: &Op) -> Result<OpOutcome, KernelError> {
        let out = self.apply_op(op)?;
        self.propagate()?;
        Ok(out)
    }

    fn apply_op(&mut self, op: &Op) -> Result<OpOutcome, KernelError> {
        match op {
            Op::NewSketch { origin, normal } => {
                let plane = Plane::new(DVec3::from_array(*origin), DVec3::from_array(*normal))
                    .map_err(SketchError::from)?;
                let mut sk = Sketch::new(plane);
                sk.parent = self.context;
                let id = self.sketches.insert(sk);
                self.sketch_order.push(id);
                Ok(OpOutcome::SketchCreated {
                    sketch: to_public(id),
                })
            }
            Op::AddRectangle { sketch, x, y, w, h } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let r = self.sketches[sid].add_rectangle(DVec2::new(*x, *y), DVec2::new(*w, *h))?;
                Ok(Self::edited(r))
            }
            Op::InsertSegment { sketch, a, b } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let r = self.sketches[sid]
                    .insert_segment(DVec2::from_array(*a), DVec2::from_array(*b))?;
                Ok(Self::edited(r))
            }
            Op::EraseSegment { sketch, a, b } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let r = self.sketches[sid]
                    .erase_segment(DVec2::from_array(*a), DVec2::from_array(*b))?;
                Ok(Self::edited(r))
            }
            Op::AddPolyline {
                sketch,
                points,
                closed,
            } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let pts: Vec<DVec2> = points.iter().map(|p| DVec2::from_array(*p)).collect();
                if self.leave_out_sure_overflow(sid, &pts, *closed) {
                    // Never seen: the limit check after the op refuses the edit (or it is
                    // done again with the stroke drawn, `Self::applied_by`).
                    return Ok(Self::edited(Vec::new()));
                }
                let r = self.sketches[sid].insert_polyline(&pts, *closed)?;
                Ok(Self::edited(r))
            }
            Op::Extrude {
                sketch,
                region,
                distance,
            } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let rid = from_public::<RegionId>(*region);
                let s = &self.sketches[sid];
                let r = s.region(rid).ok_or(SketchError::NoSuchRegion(*region))?;
                if r.consumed {
                    return Err(SketchError::RegionConsumed(*region).into());
                }
                if r.pinched {
                    return Err(SketchError::PinchedRegion(*region).into());
                }
                let (outer, holes) = s.region_rings(r);
                let mut obj = Object::extrude(s.plane(), &outer, &holes, *distance)?;
                // Lane 23: the region's curves become the rims of both caps.
                rim_curves(&mut obj, s, r);
                obj.source = Some(Provenance { sketch: sid });
                obj.member.parent = self.context;
                self.sketches[sid].consume_region(rid)?;
                let id = self.insert_object(obj);
                self.mark(self.context);
                Ok(OpOutcome::ObjectCreated {
                    object: to_public(id),
                })
            }
            Op::PushPull {
                object,
                face,
                distance,
            }
            | Op::PushPullNewFace {
                object,
                face,
                distance,
            } => {
                let oid = self.object_key(*object)?;
                self.editable_object(oid)?;
                let o = &mut self.objects[oid];
                let pushed = match op {
                    Op::PushPullNewFace { .. } => o.push_pull_new_face(*face, *distance)?,
                    _ => o.push_pull_id(*face, *distance)?,
                };
                match pushed {
                    PushOutcome::Moved => {
                        self.touch(oid);
                        let pieces = self.sever(oid);
                        if pieces.len() > 1 {
                            Ok(OpOutcome::Severed {
                                objects: pieces.into_iter().map(to_public).collect(),
                            })
                        } else {
                            Ok(OpOutcome::FaceMoved)
                        }
                    }
                    PushOutcome::Collapsed { footprint } => {
                        let removed = self.objects.remove(oid).expect("object exists");
                        self.object_order.retain(|&k| k != oid);
                        self.mark(removed.member.parent);
                        let regions = match removed.source.clone() {
                            Some(src) => {
                                let r = self.return_footprint(src.sketch, &footprint)?;
                                self.return_rims(src.sketch, &removed);
                                r
                            }
                            None => Vec::new(),
                        };
                        Ok(OpOutcome::ObjectCollapsed {
                            regions: regions.into_iter().map(to_public).collect(),
                        })
                    }
                }
            }
            Op::SplitFace { object, face, path } => {
                let pts: Vec<DVec3> = path.iter().map(|p| DVec3::from_array(*p)).collect();
                let (o, fi) = self.face_edit_target(*object, *face)?;
                let faces = o.split_face(fi, &pts)?;
                Ok(OpOutcome::FaceSplit {
                    faces: faces.to_vec(),
                })
            }
            Op::Imprint {
                object,
                face,
                points,
            } => {
                let pts: Vec<DVec3> = points.iter().map(|p| DVec3::from_array(*p)).collect();
                let (o, fi) = self.face_edit_target(*object, *face)?;
                let sub = o.imprint(fi, &pts)?;
                Ok(OpOutcome::Imprinted { face: sub })
            }
            Op::Draw {
                target,
                points,
                closed,
                curve,
            } => {
                let pts: Vec<DVec3> = points.iter().map(|p| DVec3::from_array(*p)).collect();
                let spec = match curve {
                    Some(c) => Some(stroke_curve(c, &pts, *closed)?),
                    None => None,
                };
                let spec = spec.as_ref();
                match target {
                    // A stroke on a face or sketch outside the open context goes into the
                    // context's own sketch on that plane (strokes never stick across
                    // contexts).
                    DrawTarget::Face { object, face }
                        if self
                            .objects
                            .get(from_public::<ObjectId>(*object))
                            .is_some_and(|_| {
                                !self.object_is_editable(from_public::<ObjectId>(*object))
                            }) =>
                    {
                        let o = &self.objects[from_public::<ObjectId>(*object)];
                        let fi = o
                            .face_ordinal(*face)
                            .ok_or(FaceEditError::NoSuchFace(*face))?;
                        let plane = o
                            .brep()
                            .to_polys()
                            .plane(fi)
                            .ok_or(FaceEditError::Degenerate)?;
                        self.draw_on_plane(&plane, &pts, *closed, spec)
                    }
                    DrawTarget::Sketch { sketch }
                        if self
                            .sketches
                            .get(from_public::<SketchId>(*sketch))
                            .is_some_and(|_| {
                                !self.sketch_is_editable(from_public::<SketchId>(*sketch))
                            }) =>
                    {
                        let plane = *self.sketches[from_public::<SketchId>(*sketch)].plane();
                        self.draw_on_plane(&plane, &pts, *closed, spec)
                    }
                    DrawTarget::Face { object, face } => {
                        let (o, fi) = self.face_edit_target(*object, *face)?;
                        let (faces, inside) = o.draw(fi, &pts, *closed)?;
                        Ok(OpOutcome::Drawn {
                            sketch: None,
                            regions: Vec::new(),
                            faces,
                            inside,
                            curve: None,
                        })
                    }
                    DrawTarget::Sketch { sketch } => {
                        let sid = self.sketch_edit_key(*sketch)?;
                        if spec.is_none() && self.leave_out_sure_overflow_world(sid, &pts, *closed)
                        {
                            // A placeholder (a draw answers `Drawn`), never seen: the limit
                            // check after the op refuses the edit, or it is done again with
                            // the stroke drawn (`Self::applied_by`).
                            return Ok(Self::edited(Vec::new()));
                        }
                        let regions = self.draw_in_sketch(sid, &pts, *closed)?;
                        let curve = match spec {
                            Some(c) => self.stroke_to_curve(sid, &pts, *closed, c)?,
                            None => None,
                        };
                        Ok(OpOutcome::Drawn {
                            sketch: Some(*sketch),
                            regions: regions.into_iter().map(to_public).collect(),
                            faces: Vec::new(),
                            inside: self.region_inside(sid, &pts, *closed),
                            curve,
                        })
                    }
                    DrawTarget::Plane { origin, normal } => {
                        let plane =
                            Plane::new(DVec3::from_array(*origin), DVec3::from_array(*normal))
                                .map_err(SketchError::from)?;
                        self.draw_on_plane(&plane, &pts, *closed, spec)
                    }
                }
            }
            Op::EraseEdge { object, a, b } => {
                let k = from_public::<ObjectId>(*object);
                if !self.objects.contains_key(k) {
                    return Err(FaceEditError::NoSuchObject(*object).into());
                }
                self.editable_object(k)?;
                let face =
                    self.objects[k].erase_edge(DVec3::from_array(*a), DVec3::from_array(*b))?;
                self.touch(k);
                Ok(OpOutcome::FacesMerged { face })
            }
            Op::EraseFace { object, face } => {
                let (o, fi) = self.face_edit_target(*object, *face)?;
                let face = o.erase_face(fi)?;
                Ok(OpOutcome::FacesMerged { face })
            }
            Op::EraseRegion { sketch, region } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let r = self.sketches[sid].erase_region(from_public::<RegionId>(*region))?;
                Ok(Self::edited(r))
            }
            Op::EraseCurve { sketch, curve } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let r = self.sketches[sid].erase_curve(*curve)?;
                Ok(Self::edited(r))
            }
            Op::ExplodeCurve {
                sketch,
                object,
                curve,
            } => match (sketch, object) {
                (Some(s), None) => {
                    let sid = self.sketch_edit_key(*s)?;
                    let n = self.sketches[sid].explode_curve(*curve)?;
                    Ok(OpOutcome::CurveExploded { edges: n as u64 })
                }
                (None, Some(o)) => {
                    let k = from_public::<ObjectId>(*o);
                    if !self.objects.contains_key(k) {
                        return Err(DocumentError::NoSuchObject(*o).into());
                    }
                    self.editable_object(k)?;
                    let c = self.objects[k]
                        .curves
                        .remove(curve)
                        .ok_or(SketchError::NoSuchCurve(*curve))?;
                    self.touch(k);
                    Ok(OpOutcome::CurveExploded {
                        edges: c.keys.len() as u64,
                    })
                }
                _ => Err(SketchError::NoSuchCurve(*curve).into()),
            },
            Op::Batch { ops } => {
                if ops.is_empty() {
                    return Err(DocumentError::EmptyBatch.into());
                }
                let mut outcomes = Vec::with_capacity(ops.len());
                // Lane 23b: the curves the batch's EraseCurve ops name that are there before
                // it runs. An earlier op of the same batch may take one with it (erasing the
                // region a circle bounds erases the circle): that curve is then already
                // erased, as an edge already gone is for EraseSegment, and the op does
                // nothing. A curve that was never there is still refused.
                let curve_there = |m: &Self, sketch: u64, curve: u64| {
                    m.sketch_key(sketch)
                        .is_ok_and(|k| m.sketches[k].curve(curve).is_some())
                };
                let named: Vec<bool> = ops
                    .iter()
                    .map(|op| match op {
                        Op::EraseCurve { sketch, curve } => curve_there(self, *sketch, *curve),
                        _ => false,
                    })
                    .collect();
                let last = std::mem::replace(&mut self.last_op, false);
                for (k, (op, was_there)) in ops.iter().zip(named).enumerate() {
                    self.last_op = last && k + 1 == ops.len();
                    if let Op::EraseCurve { sketch, curve } = op
                        && was_there
                        && !curve_there(self, *sketch, *curve)
                    {
                        outcomes.push(Self::edited(Vec::new()));
                        continue;
                    }
                    if matches!(
                        op,
                        Op::Undo
                            | Op::Redo
                            | Op::Batch { .. }
                            | Op::SetContext { .. }
                            | Op::LeaveContext
                    ) {
                        return Err(DocumentError::NotInBatch.into());
                    }
                    outcomes.push(self.apply_in_batch(op)?);
                }
                self.last_op = false;
                Ok(OpOutcome::Batch { outcomes })
            }
            Op::AddGuide { point, dir } => {
                let p = finite3(point).ok_or(DocumentError::BadAnnotation)?;
                let d = match dir {
                    Some(d) => {
                        let v = finite3(d).ok_or(DocumentError::BadAnnotation)?;
                        if v.length() < 1e-12 {
                            return Err(DocumentError::BadAnnotation.into());
                        }
                        v.normalize()
                    }
                    None => DVec3::ZERO,
                };
                let id = self.guides.insert(Guide { point: p, dir: d });
                Ok(OpOutcome::GuideAdded {
                    guide: to_public(id),
                })
            }
            Op::EraseGuide { guide } => {
                self.guides
                    .remove(from_public::<GuideId>(*guide))
                    .ok_or(DocumentError::NoSuchAnnotation(*guide))?;
                Ok(OpOutcome::AnnotationErased)
            }
            Op::DeleteGuides => {
                if self.guides.is_empty() {
                    return Err(DocumentError::NoGuides.into());
                }
                self.guides.clear();
                Ok(OpOutcome::AnnotationErased)
            }
            Op::AddDimension { a, b, offset } => {
                let (a, b, o) = (
                    finite3(a).ok_or(DocumentError::BadAnnotation)?,
                    finite3(b).ok_or(DocumentError::BadAnnotation)?,
                    finite3(offset).ok_or(DocumentError::BadAnnotation)?,
                );
                if a.distance(b) <= crate::tolerance::POINT_MERGE {
                    return Err(DocumentError::BadAnnotation.into());
                }
                let id = self.dimensions.insert(Dimension { a, b, offset: o });
                Ok(OpOutcome::DimensionAdded {
                    dimension: to_public(id),
                })
            }
            Op::EraseDimension { dimension } => {
                self.dimensions
                    .remove(from_public::<DimensionId>(*dimension))
                    .ok_or(DocumentError::NoSuchAnnotation(*dimension))?;
                Ok(OpOutcome::AnnotationErased)
            }
            Op::Transform {
                objects,
                xform,
                nodes,
            } => {
                let (keys, nkeys) = self.transform_targets(objects, nodes)?;
                xform.check()?;
                for &k in &keys {
                    let next = self.objects[k].transformed(xform)?;
                    self.objects[k] = next;
                    self.touch(k);
                }
                for &n in &nkeys {
                    self.transform_node(n, xform)?;
                }
                Ok(OpOutcome::Transformed {
                    objects: keys.into_iter().map(to_public).collect(),
                    nodes: nkeys.into_iter().map(to_public).collect(),
                })
            }
            Op::Copy {
                objects,
                xform,
                nodes,
            } => {
                let (keys, nkeys) = self.transform_targets(objects, nodes)?;
                let (objects, nodes) = self.copies(&keys, &nkeys, xform, 1)?;
                Ok(OpOutcome::Copied { objects, nodes })
            }
            Op::Array {
                objects,
                xform,
                count,
                nodes,
            } => {
                if *count == 0 || *count > MAX_ARRAY {
                    return Err(TransformError::BadCount.into());
                }
                let (keys, nkeys) = self.transform_targets(objects, nodes)?;
                let (objects, nodes) = self.copies(&keys, &nkeys, xform, *count)?;
                Ok(OpOutcome::Copied { objects, nodes })
            }
            Op::TransformGeometry {
                object,
                points,
                xform,
                autofold,
            } => {
                let k = from_public::<ObjectId>(*object);
                let o = self
                    .objects
                    .get(k)
                    .ok_or(TransformError::NoSuchObject(*object))?;
                self.editable_object(k)?;
                let pts: Vec<DVec3> = points.iter().map(|p| DVec3::from_array(*p)).collect();
                let (next, faces) = o.transform_geometry(&pts, xform, *autofold)?;
                self.objects[k] = next;
                self.touch(k);
                Ok(OpOutcome::GeometryMoved { faces })
            }
            Op::TransformSketch {
                sketch,
                points,
                edges,
                regions,
                curves,
                xform,
                copy,
                count,
            } => self.transform_sketch(
                *sketch, points, edges, regions, curves, xform, *copy, *count,
            ),
            Op::Offset { source, distance } => self.offset(source, *distance),
            Op::FollowMe {
                profile,
                path,
                closed,
            } => self.follow_me(profile, path, *closed),
            Op::DeleteObject { object } => {
                let k = from_public::<ObjectId>(*object);
                if !self.objects.contains_key(k) {
                    return Err(DocumentError::NoSuchObject(*object).into());
                }
                self.editable_object(k)?;
                let removed = self.objects.remove(k).expect("checked");
                self.object_order.retain(|&o| o != k);
                self.mark(removed.member.parent);
                Ok(OpOutcome::ObjectDeleted)
            }
            Op::CreateGroup { name, objects } => {
                check_name(name)?;
                if objects.is_empty() {
                    return Err(DocumentError::EmptyGroup.into());
                }
                let items: Vec<Item> = objects.iter().map(|&o| Item::Object(o)).collect();
                self.make_group(&items, Some(name))
            }
            Op::DeleteGroup { group } => {
                let k = from_public::<NodeId>(*group);
                if self.nodes.get(k).is_none_or(|n| n.kind != NodeKind::Group) {
                    return Err(DocumentError::NoSuchGroup(*group).into());
                }
                self.explode(*group)?;
                Ok(OpOutcome::GroupDeleted)
            }
            Op::SetContext { node } => self.set_context(*node),
            Op::LeaveContext => self.leave_context(),
            Op::MakeGroup { items, name } => self.make_group(items, name.as_deref()),
            Op::MakeComponent { items, name } => self.make_component(items, name),
            Op::MakeUnique { node } => self.make_unique(*node),
            Op::Explode { node } => self.explode(*node),
            Op::ExplodeSolid { object } => self.explode_solid(*object),
            Op::SolidFromFaces { faces } => self.solid_from_faces(faces),
            Op::ExtrudeFaces { faces, vector } => self.extrude_faces(faces, *vector),
            Op::UnionSolids { objects } => self.union_solids(objects),
            Op::UnifyMesh { mesh } => self.unify_mesh(*mesh),
            Op::DissolvePoint { object, at } => {
                let oid = self.object_key(*object)?;
                self.editable_object(oid)?;
                let next = dissolved_point(&self.objects[oid], DVec3::from_array(*at))?;
                self.objects[oid] = next;
                self.touch(oid);
                Ok(OpOutcome::PointsCleaned {
                    object: *object,
                    removed: 1,
                })
            }
            Op::DissolveSketchPoint { sketch, at } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let r =
                    self.sketches[sid].dissolve_point(DVec2::from_array(*at), MERGE_TOLERANCE)?;
                Ok(Self::edited(r))
            }
            Op::CleanSketchPoints { sketch } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let n = self.sketches[sid].clean_points(MERGE_TOLERANCE);
                Ok(OpOutcome::SketchPointsCleaned {
                    sketch: *sketch,
                    removed: n as u64,
                })
            }
            Op::CleanPoints { object } => {
                let oid = self.object_key(*object)?;
                self.editable_object(oid)?;
                let (next, removed) = cleaned_points(&self.objects[oid]);
                if removed > 0 {
                    self.objects[oid] = next;
                    self.touch(oid);
                }
                Ok(OpOutcome::PointsCleaned {
                    object: *object,
                    removed,
                })
            }
            Op::MergeFaces { object, faces } => self.merge_faces(*object, faces),
            Op::AutoMergeFaces { object } => self.auto_merge_faces(*object),
            Op::DeleteNode { node } => self.delete_node(*node),
            Op::Rename { item, name } => self.rename(*item, name.as_deref()),
            Op::RenameDefinition { definition, name } => self.rename_definition(*definition, name),
            Op::SetHidden { items, hidden } => self.set_flag(items, false, *hidden),
            Op::SetLocked { items, locked } => self.set_flag(items, true, *locked),
            Op::TagItems { items, tag } => self.tag_items(items, tag.as_deref()),
            Op::CreateTag { name, color } => self.create_tag(name, *color),
            Op::RenameTag { tag, name } => self.rename_tag(tag, name),
            Op::SetTagColor { tag, color } => self.set_tag_color(tag, *color),
            Op::DeleteTag { tag } => self.delete_tag(tag),
            Op::AddScene { scene } => self.add_scene(scene),
            Op::UpdateScene { name, scene } => self.update_scene(name, scene),
            Op::DeleteScene { name } => self.delete_scene(name),
            Op::ActivateScene { name } => self.activate_scene(name),
            Op::SetTag { object, tag } => {
                let items = [Item::Object(*object)];
                if let Some(t) = tag
                    && !crate::nodes::is_untagged_name(t)
                {
                    check_name(t)?;
                }
                if self.object(*object).is_none() {
                    return Err(DocumentError::NoSuchObject(*object).into());
                }
                self.tag_items(&items, tag.as_deref())
            }
            Op::SetTagVisible { tag, visible } => {
                check_name(tag)?;
                self.tags.insert(tag.clone(), *visible);
                Ok(OpOutcome::TagSet)
            }
            Op::Import {
                items,
                strokes,
                materials,
                scenes,
                hidden_tags,
                definitions,
                nodes,
            } => self.import(
                items,
                strokes,
                materials,
                scenes,
                hidden_tags,
                definitions,
                nodes,
            ),
            Op::DefineMaterial {
                name,
                color,
                texture,
                size,
                rotation,
                offset,
                stock,
            } => {
                check_name(name)?;
                self.define_material(
                    name.trim(),
                    Material {
                        color: *color,
                        texture: texture.clone(),
                        size: *size,
                        rotation: *rotation,
                        offset: *offset,
                        stock: stock.clone(),
                    },
                )
            }
            Op::RenameMaterial { name, to } => self.rename_material(name, to),
            Op::DeleteMaterial { name } => self.delete_material(name),
            Op::AddTexture { path, mime, data } => self.add_texture(path, mime, data),
            Op::Paint { target, material } => self.paint(target, material.as_deref()),
            Op::ClearPaint { items } => self.clear_paint(items),
            Op::SetFaceUv { object, face, uv } => self.set_face_uv(*object, *face, uv.as_ref()),
            Op::SetStyle { style } => self.set_style(style.as_ref()),
            Op::SetShadows { shadows } => self.set_shadows(shadows.as_ref()),
            Op::SetMaterial { object, material } => {
                if let Some(m) = material
                    && !self.materials.contains_key(m)
                {
                    return Err(DocumentError::NoSuchMaterial(m.clone()).into());
                }
                let o = self
                    .objects
                    .get_mut(from_public::<ObjectId>(*object))
                    .ok_or(DocumentError::NoSuchObject(*object))?;
                o.material = material.clone();
                let p = o.member.parent;
                self.mark(p);
                Ok(OpOutcome::MaterialSet)
            }
            Op::RenameObject { object, name } => {
                if self.object(*object).is_none() {
                    return Err(DocumentError::NoSuchObject(*object).into());
                }
                self.rename(Item::Object(*object), name.as_deref())
            }
            Op::DeleteMesh { mesh } => {
                let k = from_public::<MeshId>(*mesh);
                if !self.meshes.contains_key(k) {
                    return Err(DocumentError::NoSuchMesh(*mesh).into());
                }
                self.editable_mesh(k)?;
                let removed = self.meshes.remove(k).expect("checked");
                self.mesh_order.retain(|&m| m != k);
                self.mark(removed.member.parent);
                Ok(OpOutcome::MeshDeleted)
            }
            Op::SetGrain { part, grain } => {
                let g = (*grain != crate::cutlist::Grain::Length).then_some(*grain);
                self.set_part(*part, |a| a.grain = g)
            }
            Op::SetStock { part, stock, sheet } => {
                let stock = match stock {
                    Some(s) if !s.trim().is_empty() => {
                        check_name(s)?;
                        Some(s.trim().to_string())
                    }
                    _ => None,
                };
                let sheet = *sheet;
                self.set_part(*part, move |a| {
                    a.stock = stock;
                    a.sheet = sheet;
                })
            }
            Op::SetInclude { part, include } => {
                let v = (!*include).then_some(false);
                self.set_part(*part, |a| a.include = v)
            }
            Op::GroupLoose { meshes } => self.group_loose(meshes),
            Op::SetEdgeFlags { edges, set, clear } => self.set_edge_flags(edges, *set, *clear),
            Op::SoftenEdges {
                objects,
                meshes,
                angle,
                soft,
                smooth,
            } => self.soften_edges(objects, meshes, *angle, *soft, *smooth),
            Op::DefineStock { stock, replace } => {
                self.edit_stock(|l| l.define(stock, replace.as_deref()))
            }
            Op::DeleteStock { name } => self.edit_stock(|l| l.delete(name)),
            Op::SetMaterialStock { material, stock } => {
                self.edit_stock(|l| l.set_default(material, stock.as_deref()))
            }
            Op::SetNestSettings { settings } => self.edit_stock(|l| {
                settings.check()?;
                l.settings = *settings;
                Ok(())
            }),
            Op::ResetStockLibrary => {
                self.stock = None;
                Ok(OpOutcome::StockSet)
            }
            Op::AddSection {
                origin,
                normal,
                name,
                active,
                hidden,
            } => self.add_section(origin, normal, name.as_deref(), *active, *hidden),
            Op::PlaceSection {
                section,
                origin,
                normal,
            } => self.place_section(*section, origin, normal),
            Op::FlipSection { section } => self.flip_section(*section),
            Op::SetSectionActive { section, active } => self.set_section_active(*section, *active),
            Op::RenameSection { section, name } => self.rename_section(*section, name),
            Op::SetSectionHidden { section, hidden } => self.set_section_hidden(*section, *hidden),
            Op::DeleteSection { section } => self.delete_section(*section),
            Op::MoveSections { sections, xform } => {
                if sections.is_empty() {
                    return Err(TransformError::NothingSelected.into());
                }
                xform.check()?;
                let keys = self.section_targets(sections)?;
                self.transform_sections(&keys, xform)?;
                Ok(OpOutcome::SectionsMoved {
                    sections: keys.into_iter().map(to_public).collect(),
                })
            }
            Op::CopySections {
                sections,
                xform,
                count,
            } => {
                if sections.is_empty() {
                    return Err(TransformError::NothingSelected.into());
                }
                if *count == 0 || *count > MAX_ARRAY {
                    return Err(TransformError::BadCount.into());
                }
                xform.check()?;
                let keys = self.section_targets(sections)?;
                let sections = self.copy_sections(&keys, xform, *count)?;
                Ok(OpOutcome::SectionsCopied { sections })
            }
            Op::SliceToGroup { section, name } => self.slice_to_group(*section, name.as_deref()),
            Op::AddSheet { setup } => self.add_sheet(setup),
            Op::SetSheet { sheet, setup } => self.set_sheet(*sheet, setup),
            Op::DeleteSheet { sheet } => self.delete_sheet(*sheet),
            Op::MoveSheet { sheet, to } => self.move_sheet(*sheet, *to),
            Op::AddSheetItem { sheet, item } => self.add_sheet_item(*sheet, item),
            Op::SetSheetItem { sheet, item, kind } => self.set_sheet_item(*sheet, *item, kind),
            Op::MoveSheetItems { sheet, items, by } => self.move_sheet_items(*sheet, items, *by),
            Op::DeleteSheetItems { sheet, items } => self.delete_sheet_items(*sheet, items),
            Op::OrderSheetItem { sheet, item, to } => self.order_sheet_item(*sheet, *item, *to),
            Op::Restore { snapshot } => {
                *self = Model::from_snapshot(snapshot)?;
                Ok(OpOutcome::Restored {
                    objects: self.object_order.len(),
                    meshes: self.mesh_order.len(),
                    sketches: self.sketch_order.len(),
                })
            }
            Op::Undo | Op::Redo => Err(DocumentError::NotAModelOp.into()),
        }
    }

    /// The loops an offset makes (world points, closed or not) and the plane they are on,
    /// plus where they go: a solid's face on that plane (object key, face ordinal) or a
    /// sketch (when they reach off the solid, or the source is a sketch region).
    pub fn offset_result(
        &self,
        source: &OffsetSource,
        distance: f64,
    ) -> Result<(Vec<Vec<DVec3>>, bool, Plane), KernelError> {
        use crate::offset::{chain_on_plane, check_distance, offset_chain, offset_rings};
        check_distance(distance)?;
        let wind = |rings: Vec<Vec<DVec2>>| -> Vec<Vec<DVec2>> {
            rings
                .into_iter()
                .enumerate()
                .map(|(k, mut r)| {
                    let a = sawhorse_tessellate::signed_area(&r);
                    if (k == 0) == (a < 0.0) {
                        r.reverse();
                    }
                    r
                })
                .collect()
        };
        let (plane, mut loops, closed) = match source {
            OffsetSource::Face { object, face } => {
                let o = self
                    .object(*object)
                    .ok_or(FaceEditError::NoSuchObject(*object))?;
                let fi = o
                    .face_ordinal(*face)
                    .ok_or(FaceEditError::NoSuchFace(*face))?;
                let p = o.brep().to_polys();
                let plane = p.plane(fi).ok_or(OffsetError::NotPlanar)?;
                let rings: Vec<Vec<DVec2>> = p.faces[fi]
                    .rings()
                    .map(|r| r.iter().map(|&i| plane.to_local(p.points[i])).collect())
                    .collect();
                (plane, offset_rings(&wind(rings), distance), true)
            }
            OffsetSource::Region { sketch, region } => {
                let s = self
                    .sketch(*sketch)
                    .ok_or(SketchError::NoSuchSketch(*sketch))?;
                let r = s
                    .region(from_public::<RegionId>(*region))
                    .ok_or(SketchError::NoSuchRegion(*region))?;
                let (o, h) = s.region_rings(r);
                let rings: Vec<Vec<DVec2>> = std::iter::once(o).chain(h).collect();
                (*s.plane(), offset_rings(&wind(rings), distance), true)
            }
            OffsetSource::Edges {
                points,
                closed,
                normal,
                ..
            } => {
                let pts: Vec<DVec3> = points.iter().map(|p| DVec3::from_array(*p)).collect();
                if pts.len() < 2 || (*closed && pts.len() < 3) {
                    return Err(OffsetError::TooFewPoints.into());
                }
                let (plane, q) = chain_on_plane(&pts, DVec3::from_array(*normal))?;
                if *closed {
                    (plane, offset_rings(&wind(vec![q]), distance), true)
                } else {
                    (plane, offset_chain(&q, distance), false)
                }
            }
        };
        // Lane 23b: a circle, an arc or a polygon on its own offsets its radius.
        if let Some(exact) = self.round_offset(source, &plane, distance)? {
            loops = vec![exact];
        }
        if loops.is_empty() {
            return Err(OffsetError::Vanishes.into());
        }
        let world = loops
            .iter()
            .map(|r| r.iter().map(|&q| plane.to_world(q)).collect())
            .collect();
        Ok((world, closed, plane))
    }

    /// Lane 23b: when an offset's source is exactly one circle, arc or polygon (a sketch's
    /// curve, or a solid's rim: a region or a face bounded by it alone, or its chain of
    /// points), the offset is that curve with its radius moved by the distance: the same
    /// centre, kind and segment count, every point moved along its radius (a woodworker
    /// offsetting a 200 mm circle by 50 mm expects a 250 mm circle, as SketchUp draws it; the
    /// polygon offset moves the edges 50 mm, which left the radius 249.57 mm for 24
    /// segments). Closed: a positive distance grows; open (an arc): positive is to the left
    /// of the chain's way, as for any chain. `None` for any other source (it offsets as a
    /// polygon); refused when the radius would be gone.
    fn round_offset(
        &self,
        source: &OffsetSource,
        plane: &Plane,
        distance: f64,
    ) -> Result<Option<Vec<DVec2>>, KernelError> {
        use crate::curve::CurveInfo;
        let tol = |p: DVec3| crate::tolerance::SKETCH_MERGE + 1e-9 * p.length();
        // The source's points (world, in order) and whether they close, and the curves
        // they might be.
        let (pts, closed, candidates): (Vec<DVec3>, bool, Vec<CurveInfo>) = match source {
            OffsetSource::Region { sketch, region } => {
                let Some(s) = self.sketch(*sketch) else {
                    return Ok(None);
                };
                let Some(r) = s.region(from_public::<RegionId>(*region)) else {
                    return Ok(None);
                };
                let (outer, holes) = s.region_rings(r);
                if !holes.is_empty() {
                    return Ok(None);
                }
                let pts = outer.iter().map(|&q| s.plane().to_world(q)).collect();
                let cs = s.curves().filter_map(|(id, _)| s.curve_info(id)).collect();
                (pts, true, cs)
            }
            OffsetSource::Face { object, face } => {
                let Some(o) = self.object(*object) else {
                    return Ok(None);
                };
                let Some(fi) = o.face_ordinal(*face) else {
                    return Ok(None);
                };
                let p = o.brep().to_polys();
                let rings: Vec<Vec<DVec3>> = p.faces[fi]
                    .rings()
                    .map(|r| r.iter().map(|&i| p.points[i]).collect())
                    .collect();
                let [ring] = &rings[..] else {
                    return Ok(None);
                };
                let cs = o
                    .curves()
                    .keys()
                    .filter_map(|&id| o.curve_info(id))
                    .collect();
                (ring.clone(), true, cs)
            }
            OffsetSource::Edges {
                points,
                closed,
                object,
                ..
            } => {
                let pts: Vec<DVec3> = points.iter().map(|p| DVec3::from_array(*p)).collect();
                let cs: Vec<CurveInfo> = match object {
                    Some(o) => match self.object(*o) {
                        Some(o) => o
                            .curves()
                            .keys()
                            .filter_map(|&id| o.curve_info(id))
                            .collect(),
                        None => Vec::new(),
                    },
                    None => self
                        .sketch_order
                        .iter()
                        .map(|&k| &self.sketches[k])
                        .filter(|s| {
                            let sp = s.plane();
                            s.parent == self.context
                                && sp.normal.dot(plane.normal).abs() >= 1.0 - 1e-12
                                && sp.signed_distance(plane.origin).abs() <= PLANARITY
                        })
                        .flat_map(|s| {
                            s.curves()
                                .filter_map(|(id, _)| s.curve_info(id))
                                .collect::<Vec<_>>()
                        })
                        .collect(),
                };
                (pts, *closed, cs)
            }
        };
        let Some((centre, radius)) = candidates.iter().find_map(|c| {
            let (centre, radius) = (c.centre?, c.radius?);
            let same = c.kind.is_round()
                && c.closed == closed
                && c.points.len() == pts.len()
                && pts.iter().all(|&p| {
                    c.points
                        .iter()
                        .any(|q| DVec3::from_array(*q).distance(p) <= tol(p))
                });
            same.then_some((DVec3::from_array(centre), radius))
        }) else {
            return Ok(None);
        };
        if radius.is_nan() || radius <= 0.0 {
            return Ok(None);
        }
        let c = plane.to_local(centre);
        let mut q: Vec<DVec2> = pts.iter().map(|&p| plane.to_local(p)).collect();
        let to = if closed {
            // Wound counter-clockwise, as the polygon offset winds an outer ring.
            if sawhorse_tessellate::signed_area(&q) < 0.0 {
                q.reverse();
            }
            radius + distance
        } else {
            // Left of the way: towards the centre when the arc turns counter-clockwise.
            let turn: f64 = q.windows(2).map(|w| (w[0] - c).perp_dot(w[1] - c)).sum();
            if turn > 0.0 {
                radius - distance
            } else {
                radius + distance
            }
        };
        if to <= crate::tolerance::SKETCH_MERGE {
            return Err(OffsetError::Vanishes.into());
        }
        Ok(Some(
            q.iter().map(|&p| c + (p - c) * (to / radius)).collect(),
        ))
    }

    /// Apply Follow Me (see [`Op::FollowMe`]).
    fn follow_me(
        &mut self,
        profile: &Profile,
        path: &[[f64; 3]],
        closed: bool,
    ) -> Result<OpOutcome, KernelError> {
        use crate::sweep::{sweep_with_direction, swept_object, wound};
        let path: Vec<DVec3> = path.iter().map(|p| DVec3::from_array(*p)).collect();
        match profile {
            Profile::Region { sketch, region } => {
                let sid = self.sketch_edit_key(*sketch)?;
                let rid = from_public::<RegionId>(*region);
                let s = &self.sketches[sid];
                let r = s.region(rid).ok_or(SketchError::NoSuchRegion(*region))?;
                if r.consumed {
                    return Err(SketchError::RegionConsumed(*region).into());
                }
                if r.pinched {
                    return Err(SketchError::PinchedRegion(*region).into());
                }
                let plane = *s.plane();
                let (o, h) = s.region_rings(r);
                let rings: Vec<Vec<DVec3>> = std::iter::once(o)
                    .chain(h)
                    .map(|ring| ring.iter().map(|&q| plane.to_world(q)).collect())
                    .collect();
                let (p, _) =
                    sweep_with_direction(&wound(&plane, &rings), plane.normal, &path, closed)?;
                let mut obj = swept_object(&p)?;
                obj.member.parent = self.context;
                self.sketches[sid].consume_region(rid)?;
                let id = self.insert_object(obj);
                self.mark(self.context);
                Ok(OpOutcome::ObjectCreated {
                    object: to_public(id),
                })
            }
            Profile::Face { object, face } => {
                let k = from_public::<ObjectId>(*object);
                let o = self
                    .objects
                    .get(k)
                    .ok_or(FaceEditError::NoSuchObject(*object))?;
                self.editable_object(k)?;
                let fi = o
                    .face_ordinal(*face)
                    .ok_or(FaceEditError::NoSuchFace(*face))?;
                let p = o.brep().to_polys();
                let plane = p.plane(fi).ok_or(crate::error::SweepError::Degenerate)?;
                let rings: Vec<Vec<DVec3>> = p.faces[fi]
                    .rings()
                    .map(|r| r.iter().map(|&i| p.points[i]).collect())
                    .collect();
                let (sw, leaves) = sweep_with_direction(&rings, plane.normal, &path, closed)?;
                swept_object(&sw)?;
                // Add or cut: where the sweep is just beside the profile. Material on the
                // face's outer side means it grows the solid; on its inner side, it cuts.
                // (A closed path passes through its profile, so its first segment's way
                // says nothing; an open one agrees with the first segment.)
                let outward = {
                    let q2: Vec<Vec<DVec2>> = rings
                        .iter()
                        .map(|r| r.iter().map(|&q| plane.to_local(q)).collect())
                        .collect();
                    let eps = 10.0 * crate::tolerance::POINT_MERGE;
                    match sawhorse_tessellate::interior_point(&q2[0], &q2[1..]) {
                        Some(c) => {
                            let q = plane.to_world(c);
                            let ahead = crate::sever::contains(&sw, q + plane.normal * eps);
                            let behind = crate::sever::contains(&sw, q - plane.normal * eps);
                            if ahead != behind { ahead } else { leaves }
                        }
                        None => leaves,
                    }
                };
                let engine = crate::boolean::default_engine()?;
                let op = if outward {
                    crate::boolean::BoolOp::Union
                } else {
                    crate::boolean::BoolOp::Difference
                };
                let q = crate::boolean::combine(&p, &sw, op, engine.as_ref())?
                    .ok_or(crate::error::SweepError::RemovesEverything)?;
                let next = o
                    .rebuilt(q)
                    .map_err(|e| crate::boolean::BooleanError::InvalidResult(e.to_string()))?;
                next.validate()
                    .map_err(|e| crate::boolean::BooleanError::InvalidResult(e.to_string()))?;
                self.objects[k] = next;
                self.touch(k);
                let pieces = self.sever(k);
                Ok(OpOutcome::Swept {
                    objects: pieces.into_iter().map(to_public).collect(),
                    cut: !outward,
                })
            }
        }
    }

    /// Apply an offset (see [`Op::Offset`]).
    fn offset(&mut self, source: &OffsetSource, distance: f64) -> Result<OpOutcome, KernelError> {
        let (loops, closed, plane) = self.offset_result(source, distance)?;
        // On a solid: the face it came from, or the given solid's face on the chain's plane.
        let solid = match source {
            OffsetSource::Face { object, face } => Some((*object, *face)),
            OffsetSource::Edges {
                object: Some(object),
                ..
            } => self.object(*object).and_then(|o| {
                let p = o.brep().to_polys();
                crate::draw::face_on(&p, &plane)
                    .and_then(|fi| o.face_id_at(fi).map(|id| (*object, id)))
            }),
            _ => None,
        };
        if let Some((object, face)) = solid {
            let o = self
                .object(object)
                .ok_or(FaceEditError::NoSuchObject(object))?;
            let p = o.brep().to_polys();
            let on_solid = loops
                .iter()
                .flatten()
                .all(|&q| crate::draw::covers(&p, &plane, plane.to_local(q)));
            if on_solid {
                let mut faces: Vec<u64> = Vec::new();
                let mut inside = None;
                for (k, ring) in loops.iter().enumerate() {
                    let (o, fi) = self.face_edit_target(object, face)?;
                    let (made, enclosed) = o.draw(fi, ring, closed)?;
                    faces.extend(made);
                    if k == 0 {
                        inside = enclosed;
                    }
                }
                faces.sort_unstable();
                faces.dedup();
                return Ok(OpOutcome::Drawn {
                    sketch: None,
                    regions: Vec::new(),
                    faces,
                    inside,
                    curve: None,
                });
            }
        }
        // Lane 23: the round curves the source is made of (before anything is drawn).
        let rounds = self.offset_rounds(source, &plane);
        // Into a sketch: the region's own, or the sketch on the plane.
        let sid = match source {
            OffsetSource::Region { sketch, .. } => self.sketch_edit_key(*sketch)?,
            _ => self.sketch_on(&plane),
        };
        let mut regions: Vec<RegionId> = Vec::new();
        for ring in &loops {
            regions.extend(self.draw_in_sketch(sid, ring, closed)?);
        }
        regions.sort_unstable();
        regions.dedup();
        let mut curve = None;
        for ring in &loops {
            for id in self.offset_curves(sid, ring, closed, &rounds) {
                curve.get_or_insert(id);
            }
        }
        let inside = self.region_inside(sid, &loops[0], closed);
        Ok(OpOutcome::Drawn {
            sketch: Some(to_public(sid)),
            regions: regions.into_iter().map(to_public).collect(),
            faces: Vec::new(),
            inside,
            curve,
        })
    }

    /// Lane 23: the round curves (circles, arcs, polygons) an offset's source is made of:
    /// a region's whole curves on its rings, or the curves of the open context's sketch on
    /// the chain's plane whose every point is a point of the chain. Each as (kind, world
    /// centre, its point count, closed, segments).
    fn offset_rounds(&self, source: &OffsetSource, plane: &Plane) -> Vec<OffsetRound> {
        let take = |s: &Sketch, keep: &dyn Fn(&crate::curve::Curve) -> bool| -> Vec<OffsetRound> {
            s.curves()
                .filter(|(_, c)| c.round.is_some() && keep(c))
                .filter_map(|(_, c)| {
                    let r = c.round?;
                    Some(OffsetRound {
                        kind: c.kind,
                        centre: s.plane().to_world(r.centre),
                        points: c.chain.len(),
                        closed: c.closed,
                        segments: c.segments,
                    })
                })
                .collect()
        };
        match source {
            OffsetSource::Region { sketch, region } => {
                let Some(s) = self.sketch(*sketch) else {
                    return Vec::new();
                };
                let Some(r) = s.region(from_public::<RegionId>(*region)) else {
                    return Vec::new();
                };
                let ring = r.boundary_edges();
                take(s, &|c| {
                    c.pairs().all(|(a, b)| ring.contains(&(a.min(b), a.max(b))))
                })
            }
            OffsetSource::Edges {
                points,
                object: None,
                ..
            } => {
                let Some(s) = self
                    .sketch_order
                    .iter()
                    .map(|&k| &self.sketches[k])
                    .find(|s| {
                        let sp = s.plane();
                        s.parent == self.context
                            && sp.normal.dot(plane.normal).abs() >= 1.0 - 1e-12
                            && sp.signed_distance(plane.origin).abs() <= PLANARITY
                    })
                else {
                    return Vec::new();
                };
                let pts: Vec<DVec2> = points
                    .iter()
                    .map(|p| s.plane().to_local(DVec3::from_array(*p)))
                    .collect();
                take(s, &|c| {
                    c.chain.iter().all(|&v| {
                        s.vertex(v).is_some_and(|q| {
                            pts.iter()
                                .any(|p| p.distance(q) <= crate::tolerance::SKETCH_MERGE)
                        })
                    })
                })
            }
            _ => Vec::new(),
        }
    }

    /// Lane 23: the curves an offset loop holds, made in sketch `sid`: a loop of a closed
    /// round source's point count, all of its points at one distance from that source's
    /// centre, is a circle (or a polygon) there; a run of a round arc's point count (or two
    /// fewer, where the ends were cut to meet the lines beside it) at one distance from its
    /// centre is an arc. Returns the ids made.
    fn offset_curves(
        &mut self,
        sid: SketchId,
        ring: &[DVec3],
        closed: bool,
        rounds: &[OffsetRound],
    ) -> Vec<u64> {
        let mut made = Vec::new();
        let n = ring.len();
        if n < 2 {
            return made;
        }
        let tol = |c: DVec3, r: f64| 0.5 * crate::tolerance::CURVE_FIT + 1e-9 * (c.length() + r);
        for src in rounds {
            let d: Vec<f64> = ring.iter().map(|p| p.distance(src.centre)).collect();
            let even =
                |i: usize, j: usize| (d[i % n] - d[j % n]).abs() <= tol(src.centre, d[i % n]);
            if src.closed {
                if closed && n == src.points && (1..n).all(|i| even(0, i)) {
                    let c = StrokeCurve {
                        kind: src.kind,
                        centre: Some(src.centre),
                        segments: src.segments,
                    };
                    if let Ok(Some(id)) = self.stroke_to_curve(sid, ring, true, &c) {
                        made.push(id);
                    }
                }
                continue;
            }
            // An arc: the longest run of even points, of about the arc's length.
            let mut best: Option<(usize, usize)> = None;
            for i in 0..n {
                let mut len = 1;
                while len < n && (if closed { true } else { i + len < n }) && even(i, i + len) {
                    len += 1;
                }
                if best.is_none_or(|(_, l)| len > l) {
                    best = Some((i, len));
                }
            }
            let Some((i, len)) = best else { continue };
            if len < 2 || len > src.points || len + 2 < src.points {
                continue;
            }
            let run: Vec<DVec3> = (0..len).map(|k| ring[(i + k) % n]).collect();
            let c = StrokeCurve {
                kind: crate::curve::CurveKind::Arc,
                centre: Some(src.centre),
                segments: (len - 1) as u32,
            };
            if let Ok(Some(id)) = self.stroke_to_curve(sid, &run, false, &c) {
                made.push(id);
            }
        }
        made
    }

    /// The live objects and nodes named (duplicates dropped, order kept), each directly in
    /// the open context and unlocked.
    fn transform_targets(
        &self,
        objects: &[u64],
        nodes: &[u64],
    ) -> Result<(Vec<ObjectId>, Vec<NodeId>), KernelError> {
        if objects.is_empty() && nodes.is_empty() {
            return Err(TransformError::NothingSelected.into());
        }
        // Duplicates dropped through a set (lane 16: a list search per id was quadratic).
        let mut keys: Vec<ObjectId> = Vec::with_capacity(objects.len().min(self.objects.len()));
        let mut seen = std::collections::BTreeSet::new();
        for &o in objects {
            let k = from_public::<ObjectId>(o);
            if !self.objects.contains_key(k) {
                return Err(TransformError::NoSuchObject(o).into());
            }
            self.editable_object(k)?;
            if seen.insert(k) {
                keys.push(k);
            }
        }
        let mut nkeys: Vec<NodeId> = Vec::with_capacity(nodes.len().min(self.nodes.len()));
        let mut nseen = std::collections::BTreeSet::new();
        for &n in nodes {
            let k = self.node_key(n)?;
            self.editable_node(k)?;
            if nseen.insert(k) {
                nkeys.push(k);
            }
        }
        Ok((keys, nkeys))
    }

    /// `count` copies of each object and node, copy k mapped by `xform` k times. Returns the
    /// new ids (solids, nodes). Solids stay plain solids; an instance's copy is an instance.
    fn copies(
        &mut self,
        keys: &[ObjectId],
        nkeys: &[NodeId],
        xform: &Xform,
        count: u32,
    ) -> Result<(Vec<u64>, Vec<u64>), KernelError> {
        xform.check()?;
        // Lane 16: what the copies add, counted (component copies by what they derive)
        // with checked arithmetic before anything is allocated or copied.
        let kids = (!nkeys.is_empty()).then(|| self.kids());
        // Per copy: the solids named, and for each group or instance named, itself and
        // everything in it (solids and meshes; groups and instances).
        let (mut things, mut nodes_per) = (keys.len(), 0usize);
        for &n in nkeys {
            nodes_per = nodes_per.saturating_add(1);
            for t in kids
                .as_ref()
                .map(|k| self.subtree_things(k, n))
                .unwrap_or_default()
            {
                match t {
                    Thing::Node(_) => nodes_per = nodes_per.saturating_add(1),
                    _ => things = things.saturating_add(1),
                }
            }
        }
        let count_us = count as usize;
        let objects = (self.objects.len() + self.meshes.len())
            .saturating_add(things.saturating_mul(count_us));
        if objects > crate::limits::MAX_OBJECTS {
            return Err(crate::limits::LimitError::Objects(objects).into());
        }
        let nodes_after = self
            .nodes
            .len()
            .saturating_add(nodes_per.saturating_mul(count_us));
        if nodes_after > crate::limits::MAX_NODES {
            return Err(crate::limits::LimitError::Nodes(nodes_after).into());
        }
        let mut made = Vec::with_capacity(keys.len().saturating_mul(count as usize));
        let mut made_nodes = Vec::with_capacity(nkeys.len().saturating_mul(count as usize));
        for k in 1..=count {
            let x = xform.power(k);
            for &key in keys {
                let mut copy = self.objects[key].transformed(&x)?;
                copy.source = None;
                copy.member.slot = None;
                copy.member.locked = false;
                made.push(to_public(self.insert_object(copy)));
            }
            for &n in nkeys {
                made_nodes.push(to_public(self.copy_node(n, &x, self.context)?));
            }
        }
        self.mark(self.context);
        Ok((made, made_nodes))
    }

    /// Split an object that has come apart into one object per piece (see
    /// [`crate::sever`]). Returns every piece's key, the original first; the new pieces join
    /// the original's groups.
    fn sever(&mut self, oid: ObjectId) -> Vec<ObjectId> {
        let Some(pieces) = self.objects[oid].split_shells() else {
            return vec![oid];
        };
        let mut it = pieces.into_iter();
        let first = it.next().expect("at least two pieces");
        self.objects[oid] = first;
        let mut keys = vec![oid];
        for piece in it {
            keys.push(self.insert_object(piece));
        }
        // Inside a shared component, a cut that comes apart makes that copy unique.
        self.mark_severed(self.objects[oid].member.parent);
        keys
    }

    /// Add a new object at the end of the creation order with the next display number.
    pub(crate) fn insert_object(&mut self, mut obj: Object) -> ObjectId {
        self.last_number += 1;
        obj.number = self.last_number;
        let id = self.objects.insert(obj);
        self.object_order.push(id);
        id
    }

    /// Lane 16b: an import's materials (those the model does not hold yet), scenes and
    /// hidden tags, counted against the model's limits before any is added.
    fn import_counts(
        &self,
        materials: &[MaterialDef],
        scenes: &[SceneDef],
        hidden_tags: &[String],
    ) -> Result<(), crate::limits::LimitError> {
        use crate::limits as l;
        l::count("tags hidden by the import", hidden_tags.len(), l::MAX_TAGS)?;
        l::count(
            "scenes",
            self.scenes.len().saturating_add(scenes.len()),
            l::MAX_SCENES,
        )?;
        for sc in scenes {
            l::count(
                "tags hidden by one scene",
                sc.hidden_tags.len(),
                l::MAX_TAGS,
            )?;
        }
        l::count("materials", materials.len(), l::MAX_MATERIALS)?;
        let new: std::collections::BTreeSet<&str> = materials
            .iter()
            .map(|m| m.name.as_str())
            .filter(|n| !self.materials.contains_key(*n))
            .collect();
        l::count(
            "materials",
            self.materials.len() + new.len(),
            l::MAX_MATERIALS,
        )
    }

    /// [`Op::Import`]: everything or nothing (the caller validates and commits). Everything
    /// goes into the open context.
    #[allow(clippy::too_many_arguments)]
    fn import(
        &mut self,
        items: &[ImportItem],
        strokes: &[ImportStroke],
        materials: &[MaterialDef],
        scenes: &[SceneDef],
        hidden_tags: &[String],
        definitions: &[ImportDefinition],
        nodes: &[ImportNode],
    ) -> Result<OpOutcome, KernelError> {
        // Lane 16: what the import expands to (component copies included), counted before
        // anything is built, whoever made the op (a reader, openskp, a plugin).
        import_budget(items, strokes, definitions, nodes)?;
        // Lane 16b: and its materials, scenes and hidden tags against the model's limits.
        self.import_counts(materials, scenes, hidden_tags)?;
        let mut notes = Vec::new();
        for m in materials {
            let mut mat = m.material();
            mat.rotation = mat.rotation.rem_euclid(360.0);
            if Self::check_material_fields(&mat).is_err() {
                notes.push(format!(
                    "{}: texture size, rotation or offset out of range, left at the defaults",
                    m.name
                ));
                mat = Material {
                    stock: None,
                    ..Material::plain(m.color, m.texture.clone())
                };
            }
            self.materials.insert(clip_name(&m.name, "Material"), mat);
        }
        let (mut objects, mut meshes) = (Vec::new(), Vec::new());
        // Lane 25: what became of the curves the items and strokes carry, said once.
        // Lane 27 (issue #17): counted per placed copy. The curves made, by kind, are read
        // from the rims of every solid the import placed (`placed`), each as the view reads
        // it where it lands; the rest of a component's tally counts once per copy.
        let mut curve_tally = crate::meshbuild::CurveTally::default();
        let mut placed: Vec<ObjectId> = Vec::new();
        let mut groups: Vec<(String, Vec<Thing>)> = Vec::new();
        // Group name -> index in `groups` (lane 16: a search per item was quadratic).
        let mut group_index: BTreeMap<String, usize> = BTreeMap::new();
        let ctx = self.context;
        for item in items {
            let built = build_item(item)?;
            let sources = built.face_sources.clone();
            notes.extend(built.notes);
            curve_tally.add(&built.curves.unmade());
            let tag = self.import_tag(item.tag.as_deref());
            let material = match item.material.as_deref().map(|m| clip_name(m, "Material")) {
                Some(m) if self.materials.contains_key(&m) => Some(m),
                Some(m) => {
                    notes.push(format!(
                        "{}: material {m:?} is not defined, left unpainted",
                        item.name
                    ));
                    None
                }
                None => None,
            };
            let base = clip_name(&item.name, "Imported");
            let n = built.solids.len();
            let mut made = Vec::new();
            for (k, mut o) in built.solids.into_iter().enumerate() {
                o.name = Some(if n == 1 {
                    base.clone()
                } else {
                    clip_name(&format!("{base} {}", k + 1), "Imported")
                });
                o.tag = tag.clone();
                o.material = material.clone();
                notes.extend(self.import_face_paint(&mut o, item, sources.get(k)));
                o.member.parent = ctx;
                let id = self.insert_object(o);
                made.push(Thing::Object(id));
                objects.push(to_public(id));
                placed.push(id);
            }
            if let Some(mut m) = built.loose {
                m.name = base.clone();
                m.tag = tag.clone();
                m.material = material.clone();
                m.member.parent = ctx;
                let id = self.meshes.insert(m);
                self.mesh_order.push(id);
                meshes.push(to_public(id));
                made.push(Thing::Mesh(id));
            }
            if let Some(g) = &item.group
                && !made.is_empty()
            {
                let g = clip_name(g, "Group");
                match group_index.get(&g) {
                    Some(&i) => groups[i].1.extend(&made),
                    None => {
                        group_index.insert(g.clone(), groups.len());
                        groups.push((g, made.clone()));
                    }
                }
            }
        }
        let mut group_ids = Vec::new();
        for (name, members) in groups {
            let g = self.nodes.insert(Node {
                name: Some(name),
                kind: NodeKind::Group,
                xform: Xform::IDENTITY,
                tag: None,
                member: crate::nodes::Membership {
                    parent: ctx,
                    ..Default::default()
                },
                part: Default::default(),
                material: None,
            });
            for t in members {
                match t {
                    Thing::Object(k) => self.objects[k].member.parent = Some(g),
                    Thing::Mesh(k) => self.meshes[k].member.parent = Some(g),
                    Thing::Node(_) => {}
                }
            }
            group_ids.push(to_public(g));
        }
        // Groups and components (a `.skp`'s structure). Each definition's own tally (its
        // template items' curves, once) is counted per world copy below.
        let mut def_ids: Vec<DefId> = Vec::with_capacity(definitions.len());
        let mut def_tallies: Vec<crate::meshbuild::CurveTally> =
            Vec::with_capacity(definitions.len());
        for d in definitions {
            let mut tally = crate::meshbuild::CurveTally::default();
            let mut def = Definition {
                name: clip_name(&d.name, crate::nodes::DEFAULT_COMPONENT_NAME),
                members: BTreeMap::new(),
                next_slot: 1,
                part: Default::default(),
            };
            self.template_items(&mut def, None, &d.items, &mut notes, &mut tally)?;
            self.template_nodes(
                &mut def,
                None,
                &d.children,
                &def_ids,
                &mut notes,
                &mut tally,
            )?;
            def_ids.push(self.defs.insert(def));
            def_tallies.push(tally);
        }
        let mut made_nodes = Vec::new();
        let def_sections: Vec<&[ImportSection]> =
            definitions.iter().map(|d| d.sections.as_slice()).collect();
        for n in nodes {
            made_nodes.push(self.import_node(
                n,
                ctx,
                &def_ids,
                &def_sections,
                &mut notes,
                &mut meshes,
                &mut curve_tally,
            )?);
        }
        if !made_nodes.is_empty() {
            let kids = self.kids();
            for &n in &made_nodes {
                for t in self.subtree_things(&kids, n) {
                    if let Thing::Object(k) = t {
                        objects.push(to_public(k));
                        placed.push(k);
                    }
                }
            }
            group_ids.extend(made_nodes.iter().map(|&n| to_public(n)));
        }
        // Lane 27: each definition's tally once per world copy of it (the definitions are
        // new, so every instance of one is a copy this import placed, nested ones included).
        if def_tallies.iter().any(|t| !t.is_empty()) {
            let mut copies: BTreeMap<DefId, usize> = BTreeMap::new();
            for (_, node) in self.nodes.iter() {
                if let NodeKind::Instance(d) = node.kind {
                    *copies.entry(d).or_default() += 1;
                }
            }
            for (d, t) in def_ids.iter().zip(&def_tallies) {
                curve_tally.add_times(t, copies.get(d).copied().unwrap_or(0));
            }
        }
        // Lane 27: the rims every placed solid holds, each read where it landed (what the
        // view shows): a component copy placed with an uneven scale reads its circles and
        // arcs as polylines, counted as such and as stretched.
        for &k in &placed {
            let o = &self.objects[k];
            for (&cid, rim) in o.curves() {
                let Some(info) = o.curve_info(cid) else {
                    continue;
                };
                curve_tally.made_one(info.kind);
                if rim.kind.is_round() && info.kind == crate::curve::CurveKind::Polyline {
                    curve_tally.stretched += 1;
                }
            }
        }
        self.mark(ctx);
        let mut sketches = Vec::new();
        let mut skipped = 0usize;
        // Lane 16 (C3): the strokes are sorted into their sketches first, then each sketch
        // takes all of its strokes at once (one repair, one region pass); only a sketch that
        // refuses that takes them one at a time, skipping the ones it refuses.
        // (sketch, made for these strokes, its strokes in its own coordinates and closed)
        type Strokes = Vec<(Vec<DVec2>, bool)>;
        let mut by_sketch: Vec<(SketchId, bool, Strokes)> = Vec::new();
        // Lane 23: strokes that are curves in the file, made curves once drawn.
        let mut curved: Vec<(SketchId, Vec<DVec3>, bool, StrokeCurve)> = Vec::new();
        for st in strokes {
            let pts: Vec<DVec3> = st.points.iter().map(|p| DVec3::from_array(*p)).collect();
            if pts.len() < 2 || pts.iter().any(|p| !p.is_finite()) {
                skipped += 1;
                continue;
            }
            let plane = if pts.len() == 2 {
                // A single segment lies on many planes: prefer the horizontal one.
                Plane::new(pts[0], DVec3::Z).ok()
            } else {
                Plane::fit(&pts).ok().map(|p| {
                    // Strokes on the ground face up, like drawn ones.
                    if p.normal.z < 0.0 {
                        Plane::new(p.origin, -p.normal).unwrap_or(p)
                    } else {
                        p
                    }
                })
            };
            let Some(plane) = plane.filter(|pl| {
                pts.iter()
                    .all(|p| pl.signed_distance(*p).abs() <= PLANARITY)
            }) else {
                skipped += 1;
                continue;
            };
            let existed = self.sketch_order.len();
            let sid = self.sketch_on(&plane);
            let sp = *self.sketches[sid].plane();
            // On the sketch's own plane, as a drawn stroke must be (its plane may sit up to
            // the tolerance off the stroke's).
            if pts.iter().any(|p| sp.signed_distance(*p).abs() > PLANARITY) {
                skipped += 1;
                if self.sketch_order.len() > existed {
                    self.sketches.remove(sid);
                    self.sketch_order.retain(|&k| k != sid);
                }
                continue;
            }
            let local: Vec<DVec2> = pts.iter().map(|&p| sp.to_local(p)).collect();
            let closed = st.closed && pts.len() > 2;
            match st.curve.as_ref().map(|c| stroke_curve(c, &pts, closed)) {
                Some(Ok(c)) => curved.push((sid, pts.clone(), closed, c)),
                // Lane 25: said to be a curve but not one (counted, drawn as plain edges).
                Some(Err(_)) => curve_tally.not_landed += 1,
                None => {}
            }
            let stroke = (local, closed);
            match by_sketch.iter_mut().find(|(k, _, _)| *k == sid) {
                Some((_, _, list)) => list.push(stroke),
                None => by_sketch.push((sid, self.sketch_order.len() > existed, vec![stroke])),
            }
        }
        for (sid, created, list) in by_sketch {
            let s = &mut self.sketches[sid];
            let mut taken = list.len();
            if s.insert_polylines(&list).is_err() {
                taken = 0;
                for (pts, closed) in &list {
                    match s.insert_polyline(pts, *closed) {
                        Ok(_) => taken += 1,
                        Err(_) => skipped += 1,
                    }
                }
            }
            if taken > 0 {
                let p = to_public(sid);
                if !sketches.contains(&p) {
                    sketches.push(p);
                }
            } else if created && self.sketches[sid].edge_count() == 0 {
                self.sketches.remove(sid);
                self.sketch_order.retain(|&k| k != sid);
            }
        }
        // Lane 23: the file's curves, as curves (those whose points landed as drawn); #11:
        // each sketch's together, in the file's order.
        let mut curves_by_sketch: Vec<(SketchId, Vec<NewCurve>)> = Vec::new();
        for (sid, pts, closed, c) in &curved {
            let Some(nc) = self
                .sketches
                .contains_key(*sid)
                .then(|| self.curve_input(*sid, pts, *closed, c).ok())
                .flatten()
            else {
                curve_tally.not_landed += 1;
                continue;
            };
            match curves_by_sketch.iter_mut().find(|(k, _)| k == sid) {
                Some((_, list)) => list.push(nc),
                None => curves_by_sketch.push((*sid, vec![nc])),
            }
        }
        for (sid, list) in curves_by_sketch {
            let made = self.sketches[sid].add_curves(&list);
            for (nc, id) in list.iter().zip(made) {
                if id.is_some() {
                    curve_tally.made_one(nc.kind);
                } else {
                    curve_tally.not_landed += 1;
                }
            }
        }
        // Lane 25: one count for the strokes' curves and the items' rims together.
        notes.extend(curve_tally.notes());
        if skipped > 0 {
            notes.push(format!(
                "skipped {skipped} stroke{} that were not flat or would not go into a sketch",
                if skipped == 1 { "" } else { "s" }
            ));
        }
        for sc in scenes {
            let mut sc = sc.clone();
            sc.name = clip_name(&sc.name, "Scene");
            let finite = sc
                .eye
                .iter()
                .chain(&sc.target)
                .chain(&sc.up)
                .all(|v| v.is_finite());
            if finite && sc.fov.is_finite() {
                self.scenes.push(sc);
            } else {
                notes.push(format!("scene {:?} has a broken camera, skipped", sc.name));
            }
        }
        for t in hidden_tags {
            if !crate::nodes::is_untagged_name(t) {
                self.tags.insert(clip_name(t, "Tag"), false);
            }
        }
        if objects.is_empty()
            && group_ids.is_empty()
            && meshes.is_empty()
            && sketches.is_empty()
            && materials.is_empty()
            && scenes.is_empty()
        {
            return Err(DocumentError::NothingImported.into());
        }
        Ok(OpOutcome::Imported {
            objects,
            meshes,
            groups: group_ids,
            sketches,
            notes,
        })
    }

    /// A tag from a file: "", "Untagged" and "Layer0"-style defaults mean none; otherwise the
    /// tag is listed (visible unless it already is listed hidden).
    fn import_tag(&mut self, tag: Option<&str>) -> Option<String> {
        let t = tag?;
        if crate::nodes::is_untagged_name(t) {
            return None;
        }
        let t = clip_name(t, "Tag");
        self.tags.entry(t.clone()).or_insert(true);
        Some(t)
    }

    /// Import items as template members of `def` (definition coordinates).
    fn template_items(
        &mut self,
        def: &mut Definition,
        parent: Option<u32>,
        items: &[ImportItem],
        notes: &mut Vec<String>,
        curves: &mut crate::meshbuild::CurveTally,
    ) -> Result<(), KernelError> {
        use crate::nodes::{Member, MemberKind};
        for item in items {
            let built = build_item(item)?;
            let sources = built.face_sources.clone();
            notes.extend(built.notes);
            curves.add(&built.curves.unmade());
            let tag = self.import_tag(item.tag.as_deref());
            let material = item
                .material
                .as_deref()
                .map(|m| clip_name(m, "Material"))
                .filter(|m| self.materials.contains_key(m));
            let base = clip_name(&item.name, "Imported");
            let n = built.solids.len();
            for (k, mut o) in built.solids.into_iter().enumerate() {
                o.material = material.clone();
                notes.extend(self.import_face_paint(&mut o, item, sources.get(k)));
                let name = if n == 1 {
                    base.clone()
                } else {
                    clip_name(&format!("{base} {}", k + 1), "Imported")
                };
                let slot = def.next_slot;
                def.next_slot += 1;
                def.members.insert(
                    slot,
                    Member {
                        parent,
                        name: Some(name),
                        tag: tag.clone(),
                        hidden: false,
                        locked: false,
                        part: Default::default(),
                        kind: MemberKind::Solid(Box::new(o)),
                        material: None,
                    },
                );
            }
            if let Some(mut m) = built.loose {
                m.name = base.clone();
                m.material = material.clone();
                let slot = def.next_slot;
                def.next_slot += 1;
                def.members.insert(
                    slot,
                    Member {
                        parent,
                        name: Some(base.clone()),
                        tag: tag.clone(),
                        hidden: false,
                        locked: false,
                        part: Default::default(),
                        kind: MemberKind::Mesh(Box::new(m)),
                        material: None,
                    },
                );
            }
        }
        Ok(())
    }

    /// Import nested groups and instances as template members of `def`.
    fn template_nodes(
        &mut self,
        def: &mut Definition,
        parent: Option<u32>,
        nodes: &[ImportNode],
        def_ids: &[DefId],
        notes: &mut Vec<String>,
        curves: &mut crate::meshbuild::CurveTally,
    ) -> Result<(), KernelError> {
        use crate::nodes::{Member, MemberKind};
        for n in nodes {
            let slot = def.next_slot;
            def.next_slot += 1;
            let tag = self.import_tag(n.tag.as_deref());
            let name = n.name.as_deref().map(|x| clip_name(x, "Group"));
            match &n.kind {
                ImportNodeKind::Group { items, children } => {
                    def.members.insert(
                        slot,
                        Member {
                            parent,
                            name,
                            tag,
                            hidden: n.hidden,
                            locked: false,
                            part: Default::default(),
                            kind: MemberKind::Group(Xform::IDENTITY),
                            material: None,
                        },
                    );
                    self.template_items(def, Some(slot), items, notes, curves)?;
                    self.template_nodes(def, Some(slot), children, def_ids, notes, curves)?;
                }
                ImportNodeKind::Instance { definition, xform } => {
                    let d = *def_ids.get(*definition).ok_or_else(|| {
                        crate::error::StructureError::BadStructure(
                            "a definition places one that is not listed before it".into(),
                        )
                    })?;
                    xform.check()?;
                    def.members.insert(
                        slot,
                        Member {
                            parent,
                            name,
                            tag,
                            hidden: n.hidden,
                            locked: false,
                            part: Default::default(),
                            kind: MemberKind::Instance(d, *xform),
                            material: None,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    /// Import a group or instance into the world under `parent`.
    #[allow(clippy::too_many_arguments)]
    fn import_node(
        &mut self,
        n: &ImportNode,
        parent: Option<NodeId>,
        def_ids: &[DefId],
        def_sections: &[&[ImportSection]],
        notes: &mut Vec<String>,
        meshes: &mut Vec<u64>,
        curves: &mut crate::meshbuild::CurveTally,
    ) -> Result<NodeId, KernelError> {
        let node =
            self.import_node_inner(n, parent, def_ids, def_sections, notes, meshes, curves)?;
        // Section planes (lane 12): the group's own, in the enclosing frame; an instance's
        // come from its definition, placed with it. All inactive.
        let mut planes: Vec<(ImportSection, Option<Xform>)> =
            n.sections.iter().map(|s| (s.clone(), None)).collect();
        if let ImportNodeKind::Instance { definition, xform } = &n.kind {
            planes.extend(
                def_sections
                    .get(*definition)
                    .into_iter()
                    .flat_map(|x| x.iter())
                    .map(|s| (s.clone(), Some(*xform))),
            );
        }
        for (s, x) in planes {
            let Ok((origin, normal)) = crate::section::plane_from(&s.origin, &s.normal) else {
                notes.push("a section plane without a usable position was left out".into());
                continue;
            };
            let mut plane = crate::section::SectionPlane {
                name: s
                    .name
                    .as_deref()
                    .map_or_else(|| "Section".to_string(), |x| clip_name(x, "Section")),
                origin,
                normal,
                parent: Some(node),
                active: false,
                hidden: s.hidden,
            };
            if let Some(x) = x {
                plane = plane.mapped(&x)?;
            }
            self.sections.insert(plane);
        }
        Ok(node)
    }

    #[allow(clippy::too_many_arguments)]
    fn import_node_inner(
        &mut self,
        n: &ImportNode,
        parent: Option<NodeId>,
        def_ids: &[DefId],
        def_sections: &[&[ImportSection]],
        notes: &mut Vec<String>,
        meshes: &mut Vec<u64>,
        curves: &mut crate::meshbuild::CurveTally,
    ) -> Result<NodeId, KernelError> {
        let tag = self.import_tag(n.tag.as_deref());
        let member = crate::nodes::Membership {
            parent,
            slot: None,
            hidden: n.hidden,
            locked: false,
        };
        match &n.kind {
            ImportNodeKind::Group { items, children } => {
                let g = self.nodes.insert(Node {
                    name: n.name.as_deref().map(|x| clip_name(x, "Group")),
                    kind: NodeKind::Group,
                    xform: Xform::IDENTITY,
                    tag,
                    member,
                    part: Default::default(),
                    material: None,
                });
                for item in items {
                    let built = build_item(item)?;
                    let sources = built.face_sources.clone();
                    notes.extend(built.notes);
                    curves.add(&built.curves.unmade());
                    let tag = self.import_tag(item.tag.as_deref());
                    let material = item
                        .material
                        .as_deref()
                        .map(|m| clip_name(m, "Material"))
                        .filter(|m| self.materials.contains_key(m));
                    let base = clip_name(&item.name, "Imported");
                    let count = built.solids.len();
                    for (k, mut o) in built.solids.into_iter().enumerate() {
                        o.name = Some(if count == 1 {
                            base.clone()
                        } else {
                            clip_name(&format!("{base} {}", k + 1), "Imported")
                        });
                        o.tag = tag.clone();
                        o.material = material.clone();
                        notes.extend(self.import_face_paint(&mut o, item, sources.get(k)));
                        o.member.parent = Some(g);
                        self.insert_object(o);
                    }
                    if let Some(mut m) = built.loose {
                        m.name = base.clone();
                        m.tag = tag.clone();
                        m.material = material.clone();
                        m.member.parent = Some(g);
                        let id = self.meshes.insert(m);
                        self.mesh_order.push(id);
                        meshes.push(to_public(id));
                    }
                }
                for c in children {
                    self.import_node(c, Some(g), def_ids, def_sections, notes, meshes, curves)?;
                }
                Ok(g)
            }
            ImportNodeKind::Instance { definition, xform } => {
                let d = *def_ids.get(*definition).ok_or_else(|| {
                    crate::error::StructureError::BadStructure(
                        "an instance places a definition that is not in the import".into(),
                    )
                })?;
                xform.check()?;
                let i = self.nodes.insert(Node {
                    name: n.name.as_deref().map(|x| clip_name(x, "Component")),
                    kind: NodeKind::Instance(d),
                    xform: *xform,
                    tag,
                    member,
                    part: Default::default(),
                    material: None,
                });
                self.rederive(i)?;
                let kids = self.kids();
                for t in self.subtree_things(&kids, i) {
                    if let Thing::Mesh(k) = t {
                        meshes.push(to_public(k));
                    }
                }
                Ok(i)
            }
        }
    }

    /// The open context's sketch lying on `plane` (either facing, within the planarity
    /// tolerance), or a new one on it there. Strokes only stick to strokes of the same
    /// editing context.
    pub(crate) fn sketch_on(&mut self, plane: &Plane) -> SketchId {
        let found = self.sketch_order.iter().copied().find(|&k| {
            let s = &self.sketches[k];
            let sp = s.plane();
            s.parent == self.context
                && sp.normal.dot(plane.normal).abs() >= 1.0 - 1e-12
                && sp.signed_distance(plane.origin).abs() <= PLANARITY
        });
        found.unwrap_or_else(|| {
            let mut s = Sketch::new(*plane);
            s.parent = self.context;
            let id = self.sketches.insert(s);
            self.sketch_order.push(id);
            id
        })
    }

    /// A stroke onto a plane: into the open context's sketch on it (or a new one).
    fn draw_on_plane(
        &mut self,
        plane: &Plane,
        pts: &[DVec3],
        closed: bool,
        curve: Option<&StrokeCurve>,
    ) -> Result<OpOutcome, KernelError> {
        let sid = self.sketch_on(plane);
        if curve.is_none() && self.leave_out_sure_overflow_world(sid, pts, closed) {
            // A placeholder, never seen (see the same in `apply_op`'s draw).
            return Ok(Self::edited(Vec::new()));
        }
        let (sid, regions) = self.draw_in_sketch_closing(sid, pts, closed, curve.is_none())?;
        let curve = match curve {
            Some(c) => self.stroke_to_curve(sid, pts, closed, c)?,
            None => None,
        };
        Ok(OpOutcome::Drawn {
            sketch: Some(to_public(sid)),
            regions: regions.into_iter().map(to_public).collect(),
            faces: Vec::new(),
            inside: self.region_inside(sid, pts, closed),
            curve,
        })
    }

    /// Lane 23: the stroke just drawn into sketch `sid` becomes curve `c` (see
    /// [`Sketch::add_curve`]): its centre must lie on the sketch's plane. Returns the curve's
    /// id (its first piece's), `None` when the stroke did not land as one.
    pub(crate) fn stroke_to_curve(
        &mut self,
        sid: SketchId,
        pts: &[DVec3],
        closed: bool,
        c: &StrokeCurve,
    ) -> Result<Option<u64>, KernelError> {
        let n = self.curve_input(sid, pts, closed, c)?;
        Ok(self.sketches[sid].add_curve(&n.pts, n.closed, n.kind, n.centre, n.segments))
    }

    /// [`Self::stroke_to_curve`]'s curve in sketch `sid`'s frame, refused when its centre is
    /// off the sketch's plane.
    fn curve_input(
        &self,
        sid: SketchId,
        pts: &[DVec3],
        closed: bool,
        c: &StrokeCurve,
    ) -> Result<NewCurve, KernelError> {
        let plane = *self.sketches[sid].plane();
        let centre = match c.centre {
            Some(w) => {
                if plane.signed_distance(w).abs() > PLANARITY {
                    return Err(SketchError::NotACurve {
                        kind: c.kind.word(),
                        why: "its centre is off its plane",
                    }
                    .into());
                }
                Some(plane.to_local(w))
            }
            None => None,
        };
        Ok(NewCurve {
            pts: pts.iter().map(|&p| plane.to_local(p)).collect(),
            closed,
            kind: c.kind,
            centre,
            segments: c.segments,
        })
    }

    /// A sketch that may be edited: it exists and belongs to the open context.
    pub(crate) fn sketch_edit_key(&self, id: u64) -> Result<SketchId, KernelError> {
        let k = self.sketch_key(id)?;
        self.editable_sketch(k)?;
        Ok(k)
    }

    /// The region a closed stroke encloses (holding an interior point of it), if any.
    fn region_inside(&self, sid: SketchId, pts: &[DVec3], closed: bool) -> Option<u64> {
        if !closed {
            return None;
        }
        let s = &self.sketches[sid];
        let ring: Vec<DVec2> = pts.iter().map(|&p| s.plane().to_local(p)).collect();
        let q = sawhorse_tessellate::interior_point(&ring, &[])?;
        s.region_at(q).map(to_public)
    }

    /// Lane 53 (#65): a stroke into sketch `sid` (its own frame) that is certain to take the
    /// sketch past [`crate::limits::MAX_SKETCH_EDGES`] is left out: true, with
    /// `unbuilt_stroke` set to the count, and the edit goes on to its limit check, which
    /// refuses it as it refuses the stroke inserted ([`Self::applied_by`]). Only for the last
    /// op of an edit (`last_op`), and only when the count is certain
    /// ([`Sketch::edges_after_clear_polyline`]); otherwise false and the stroke goes in as
    /// always, so every stroke accepted before still is, and every edit refused is refused
    /// with the same error.
    fn leave_out_sure_overflow(&mut self, sid: SketchId, pts: &[DVec2], closed: bool) -> bool {
        use crate::limits::MAX_SKETCH_EDGES;
        let s = &self.sketches[sid];
        let segs = if closed {
            pts.len()
        } else {
            pts.len().saturating_sub(1)
        };
        #[cfg(test)]
        let forced = FORCE_LEAVE_OUT.with(|f| f.get());
        #[cfg(not(test))]
        let forced = false;
        if !self.last_op || (!forced && s.edge_count() + segs <= MAX_SKETCH_EDGES) {
            return false;
        }
        #[cfg(test)]
        if forced && let Some(n) = s.edges_after_clear_polyline(pts, closed) {
            self.unbuilt_stroke = Some((sid, n));
            return true;
        }
        match s.edges_after_clear_polyline(pts, closed) {
            Some(n) if n > MAX_SKETCH_EDGES => {
                self.unbuilt_stroke = Some((sid, n));
                true
            }
            _ => false,
        }
    }

    /// [`Self::leave_out_sure_overflow`] for a world-space stroke, taken into the sketch's
    /// frame as [`Self::draw_in_sketch`] takes it (a stroke it refuses goes on to be refused
    /// there).
    fn leave_out_sure_overflow_world(
        &mut self,
        sid: SketchId,
        pts: &[DVec3],
        closed: bool,
    ) -> bool {
        let s = &self.sketches[sid];
        let segs = if closed {
            pts.len()
        } else {
            pts.len().saturating_sub(1)
        };
        if !self.last_op || s.edge_count() + segs <= crate::limits::MAX_SKETCH_EDGES {
            return false;
        }
        let plane = *s.plane();
        let mut local = Vec::with_capacity(pts.len());
        for p in pts {
            if !p.is_finite() || plane.signed_distance(*p).abs() > PLANARITY {
                return false;
            }
            local.push(plane.to_local(*p));
        }
        self.leave_out_sure_overflow(sid, &local, closed)
    }

    /// Insert a world-space stroke into a sketch (every point on its plane).
    fn draw_in_sketch(
        &mut self,
        sid: SketchId,
        pts: &[DVec3],
        closed: bool,
    ) -> Result<Vec<RegionId>, SketchError> {
        self.draw_in_sketch_closing(sid, pts, closed, false)
            .map(|(_, r)| r)
    }

    /// [`Self::draw_in_sketch`], closing against other sketches' edges (sketchcraft E-19):
    /// an open stroke that, with edges of other sketches of the open context lying on its
    /// plane and connected to it, closes a loop takes those edges into its sketch (edges are
    /// shared in SketchUp); otherwise nothing is added. With `relocate`, a straight stroke
    /// that closes nothing on its plane (a single line has no plane of its own) goes onto
    /// the plane through it and an edge touching it where it does close one. Returns the
    /// sketch the stroke went into and the regions it bounds.
    fn draw_in_sketch_closing(
        &mut self,
        sid: SketchId,
        pts: &[DVec3],
        closed: bool,
        relocate: bool,
    ) -> Result<(SketchId, Vec<RegionId>), SketchError> {
        let plane = *self.sketches[sid].plane();
        let mut local = Vec::with_capacity(pts.len());
        for p in pts {
            if !p.is_finite() {
                return Err(SketchError::OutOfBounds);
            }
            if plane.signed_distance(*p).abs() > PLANARITY {
                return Err(SketchError::OffPlane);
            }
            local.push(plane.to_local(*p));
        }
        if local.len() < 2 {
            return Err(SketchError::Degenerate);
        }
        let before = self.sketches[sid].clone();
        let mut regions = self.sketches[sid].insert_polyline(&local, closed)?;
        if closed {
            return Ok((sid, regions));
        }
        let parent = self.sketches[sid].parent;
        let mut foreign = self.edges_on_plane(&plane, parent, &[sid]);
        foreign.extend(self.solid_closers(&plane, parent, pts));
        let mut s = self.sketches[sid].clone();
        let more = import_connected(&mut s, &plane, pts, &foreign);
        if !more.is_empty() {
            self.sketches[sid] = s;
            regions.extend(more);
            regions.sort();
            regions.dedup();
        }
        if !(relocate && straight(pts)) {
            return Ok((sid, regions));
        }
        // A straight line may close loops on other planes too (an edge closing a roof and
        // a wall at once makes both faces, as in SketchUp).
        let others = self.close_on_other_planes(sid, pts, parent, &before);
        if regions.is_empty()
            && let Some((k, rs)) = others.first().cloned()
        {
            // It closed nothing where it was drawn: it lives only where it closes.
            let emptied = before.edge_count() == 0 && self.sketch_order.first() != Some(&sid);
            self.sketches[sid] = before;
            if emptied {
                self.sketches.remove(sid);
                self.sketch_order.retain(|&x| x != sid);
            }
            return Ok((k, rs));
        }
        Ok((sid, regions))
    }

    /// sketchcraft E-19: the edges (world) of the open context's sketches other than
    /// `skip` lying on `plane`.
    fn edges_on_plane(
        &self,
        plane: &Plane,
        parent: Option<crate::ids::NodeId>,
        skip: &[SketchId],
    ) -> Vec<(DVec3, DVec3)> {
        let mut out = Vec::new();
        for &k in &self.sketch_order {
            if skip.contains(&k) {
                continue;
            }
            let s = &self.sketches[k];
            if s.parent != parent {
                continue;
            }
            let sp = *s.plane();
            for (_, [va, vb]) in s.edges() {
                let (Some(a), Some(b)) = (s.vertex(va), s.vertex(vb)) else {
                    continue;
                };
                let (a, b) = (sp.to_world(a), sp.to_world(b));
                if plane.signed_distance(a).abs() <= PLANARITY
                    && plane.signed_distance(b).abs() <= PLANARITY
                {
                    out.push((a, b));
                }
            }
        }
        out
    }

    /// sketchcraft E-25 (H-20261117-3): the pieces of the context's solid edges on
    /// `plane` that close the stroke `pts` against the solid: the part of one edge between
    /// the stroke's two ends, or the two parts meeting at a corner when each end lies on
    /// one of two edges sharing it. Only these are taken, never a whole face outline (the
    /// solid's face is already a face).
    fn solid_closers(
        &self,
        plane: &Plane,
        parent: Option<crate::ids::NodeId>,
        pts: &[DVec3],
    ) -> Vec<(DVec3, DVec3)> {
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        if a.distance(b) <= PLANARITY {
            return Vec::new();
        }
        let segs: Vec<(DVec3, DVec3)> = self
            .solid_segments(parent)
            .into_iter()
            .filter(|(p, q)| {
                plane.signed_distance(*p).abs() <= PLANARITY
                    && plane.signed_distance(*q).abs() <= PLANARITY
            })
            .collect();
        let on = |x: DVec3, (p, q): (DVec3, DVec3)| {
            let d = q - p;
            let t = ((x - p).dot(d) / d.length_squared()).clamp(0.0, 1.0);
            (p + d * t).distance(x) <= PLANARITY
        };
        if segs.iter().any(|&sg| on(a, sg) && on(b, sg)) {
            return vec![(a, b)];
        }
        for &s1 in segs.iter().filter(|&&sg| on(a, sg)) {
            for &s2 in segs.iter().filter(|&&sg| on(b, sg)) {
                for c in [s1.0, s1.1] {
                    if (c.distance(s2.0) <= PLANARITY || c.distance(s2.1) <= PLANARITY)
                        && c.distance(a) > PLANARITY
                        && c.distance(b) > PLANARITY
                    {
                        return vec![(a, c), (c, b)];
                    }
                }
            }
        }
        Vec::new()
    }

    /// sketchcraft E-28 (H-20261117-5): see [`Op::ExplodeSolid`].
    fn explode_solid(&mut self, object: u64) -> Result<OpOutcome, KernelError> {
        let k = from_public::<ObjectId>(object);
        if !self.objects.contains_key(k) {
            return Err(DocumentError::NoSuchObject(object).into());
        }
        self.editable_object(k)?;
        let polys = self.objects[k].brep().to_polys();
        let mut faces = 0u64;
        for f in &polys.faces {
            let world: Vec<Vec<DVec3>> = f
                .rings()
                .map(|r| r.iter().map(|&i| polys.points[i]).collect())
                .collect();
            let plane = match f.plane {
                Some(p) => p,
                None => {
                    let Some(n) = newell(&world[0]) else { continue };
                    let Ok(p) = Plane::new(world[0][0], n) else {
                        continue;
                    };
                    p
                }
            };
            let sid = self.sketch_on(&plane);
            let sp = *self.sketches[sid].plane();
            let local: Vec<Vec<DVec2>> = world
                .iter()
                .map(|r| r.iter().map(|&p| sp.to_local(p)).collect())
                .collect();
            let before: Vec<RegionId> = self.sketches[sid].all_regions().map(|(r, _)| r).collect();
            // The face's rings drawn, and what is inside the outer ring and outside the
            // holes live again (a footprint this very solid used up comes back).
            self.sketches[sid].release_footprint(&local)?;
            // A hole stays a hole: a region new inside one is used up (not a face).
            let s = &self.sketches[sid];
            let in_holes: Vec<RegionId> = s
                .regions()
                .filter(|(r, reg)| {
                    !before.contains(r)
                        && local[1..]
                            .iter()
                            .any(|h| crate::sketch::inside_rings(reg.anchor, h, &[]))
                })
                .map(|(r, _)| r)
                .collect();
            for r in in_holes {
                self.sketches[sid].consume_region(r)?;
            }
            faces += 1;
        }
        let removed = self.objects.remove(k).expect("checked");
        self.object_order.retain(|&o| o != k);
        self.mark(removed.member.parent);
        Ok(OpOutcome::SolidExploded { face_count: faces })
    }
    /// sketchcraft E-28 (H-20261117-5): see [`Op::SolidFromFaces`].
    fn solid_from_faces(&mut self, faces: &[[u64; 2]]) -> Result<OpOutcome, KernelError> {
        if faces.len() < 4 {
            return Err(not_a_volume(
                "a volume needs at least 4 faces: select the faces that close it",
            ));
        }
        let (rings_of, used) = self.face_rings(faces)?;
        let mut obj = solid_from_rings(&rings_of)?;
        obj.member.parent = self.context;
        for &(sid, rid) in &used {
            self.sketches[sid].consume_region(rid)?;
        }
        let id = self.insert_object(obj);
        self.mark(self.context);
        Ok(OpOutcome::ObjectCreated {
            object: to_public(id),
        })
    }

    /// sketchcraft E-30 (H-20261117-7): see [`Op::ExtrudeFaces`].
    fn extrude_faces(
        &mut self,
        faces: &[[u64; 2]],
        vector: [f64; 3],
    ) -> Result<OpOutcome, KernelError> {
        let v = DVec3::from_array(vector);
        if !v.is_finite() || v.length() <= PLANARITY {
            return Err(not_a_volume("give a distance to extrude along"));
        }
        if faces.is_empty() {
            return Err(not_a_volume("select the faces to extrude"));
        }
        let (bottom, _) = self.face_rings(faces)?;
        for f in &bottom {
            if newell(&f[0]).is_none_or(|n| n.dot(v).abs() <= 1e-9 * v.length()) {
                return Err(not_a_volume(
                    "a face lies along that direction: it would make no volume",
                ));
            }
        }
        let (points, polys) = weld_rings(&bottom);
        let mut uses: BTreeMap<(usize, usize), Vec<(usize, usize)>> = BTreeMap::new();
        for f in &polys {
            for r in f {
                for i in 0..r.len() {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    uses.entry((a.min(b), a.max(b))).or_default().push((a, b));
                }
            }
        }
        if uses.values().any(|u| u.len() > 2) {
            return Err(not_a_volume("an edge joins more than two of these faces"));
        }
        // The faces, the faces moved along `v`, and a side along every edge on the rim.
        let mut rings: Vec<Vec<Vec<DVec3>>> = bottom.clone();
        rings.extend(bottom.iter().map(|f| {
            f.iter()
                .map(|r| r.iter().map(|&p| p + v).collect())
                .collect()
        }));
        // Review (constat 2): faces apart make one solid each (the pieces: faces joined
        // across shared edges, with their tops and sides).
        let groups = face_groups(&polys);
        let n = bottom.len();
        let mut objs = Vec::new();
        for g in &groups {
            let mut piece: Vec<Vec<Vec<DVec3>>> = Vec::new();
            for &fi in g {
                piece.push(rings[fi].clone());
                piece.push(rings[n + fi].clone());
            }
            for u in uses.values().filter(|u| u.len() == 1) {
                let (a, b) = u[0];
                if g.iter()
                    .any(|&fi| polys[fi].iter().any(|r| r.contains(&a) && r.contains(&b)))
                {
                    let (pa, pb) = (points[a], points[b]);
                    piece.push(vec![vec![pa, pb, pb + v, pa + v]]);
                }
            }
            let mut obj = solid_from_rings(&piece)?;
            obj.member.parent = self.context;
            objs.push(obj);
        }
        let ids: Vec<u64> = objs
            .into_iter()
            .map(|o| to_public(self.insert_object(o)))
            .collect();
        self.mark(self.context);
        Ok(if ids.len() == 1 {
            OpOutcome::ObjectCreated { object: ids[0] }
        } else {
            OpOutcome::Severed { objects: ids }
        })
    }

    /// sketchcraft E-41 (H-20261117-18): see [`Op::MergeFaces`].
    fn merge_faces(&mut self, object: u64, faces: &[u64]) -> Result<OpOutcome, KernelError> {
        let oid = self.object_key(object)?;
        self.editable_object(oid)?;
        let (next, face) = merged_object(&self.objects[oid], faces)?;
        self.objects[oid] = next;
        self.touch(oid);
        Ok(OpOutcome::FacesMerged { face })
    }

    /// sketchcraft E-42 (H-20261117-19): see [`Op::AutoMergeFaces`].
    fn auto_merge_faces(&mut self, object: u64) -> Result<OpOutcome, KernelError> {
        let oid = self.object_key(object)?;
        self.editable_object(oid)?;
        let (next, merged) = auto_merged(&self.objects[oid]);
        if merged > 0 {
            self.objects[oid] = next;
            self.touch(oid);
        }
        Ok(OpOutcome::AutoMerged { object, merged })
    }

    /// sketchcraft E-40 (H-20261117-17): see [`Op::UnifyMesh`].
    fn unify_mesh(&mut self, mesh: u64) -> Result<OpOutcome, KernelError> {
        let k = from_public::<MeshId>(mesh);
        if !self.meshes.contains_key(k) {
            return Err(DocumentError::NoSuchMesh(mesh).into());
        }
        self.editable_mesh(k)?;
        let m = self.meshes[k].clone();
        let (mut obj, notes) = unify_25d(&m.positions, &m.faces, UNIFY_GAP)?;
        obj.member.parent = m.member.parent;
        obj.name = Some(m.name.clone());
        obj.tag = m.tag.clone();
        self.meshes.remove(k);
        self.mesh_order.retain(|&x| x != k);
        let id = self.insert_object(obj);
        self.mark(m.member.parent);
        Ok(OpOutcome::MeshUnified {
            object: to_public(id),
            notes,
        })
    }

    /// sketchcraft E-04 (H-20261117-14): see [`Op::UnionSolids`].
    fn union_solids(&mut self, objects: &[u64]) -> Result<OpOutcome, KernelError> {
        let mut keys: Vec<ObjectId> = Vec::new();
        for &o in objects {
            let k = from_public::<ObjectId>(o);
            if !self.objects.contains_key(k) {
                return Err(DocumentError::NoSuchObject(o).into());
            }
            self.editable_object(k)?;
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        if keys.len() < 2 {
            return Err(not_a_volume("select at least two solids to join"));
        }
        let first = keys[0];
        let mut acc = self.objects[first].clone();
        for &k in &keys[1..] {
            // Review (constat 11): solids meeting only along an edge or at a corner make no
            // valid solid: say so plainly.
            acc = acc
                .combine(&self.objects[k], crate::boolean::BoolOp::Union)
                .map_err(|_| {
                    not_a_volume(
                        "these solids cannot be joined: they meet only along an edge or at a corner (or the result is not a valid solid)",
                    )
                })?
                .ok_or_else(|| not_a_volume("the union is empty"))?;
        }
        self.objects[first] = acc;
        for &k in &keys[1..] {
            let removed = self.objects.remove(k).expect("checked");
            self.object_order.retain(|&o| o != k);
            self.mark(removed.member.parent);
        }
        self.touch(first);
        let pieces = self.sever(first);
        if pieces.len() > 1 {
            return Err(not_a_volume(
                "these solids neither touch nor overlap: they stay apart",
            ));
        }
        Ok(OpOutcome::ObjectCreated {
            object: to_public(first),
        })
    }

    /// sketchcraft E-36 (H-20261117-14): the volume (m³) the faces `[sketch, region]`
    /// enclose when they close one, without making a solid; `None` otherwise.
    pub fn closed_faces_volume(&self, faces: &[[u64; 2]]) -> Option<f64> {
        if faces.len() < 4 {
            return None;
        }
        let mut rings: Vec<Vec<Vec<DVec3>>> = Vec::new();
        let mut seen: Vec<[u64; 2]> = Vec::new();
        for &f in faces {
            if seen.contains(&f) {
                continue;
            }
            seen.push(f);
            rings.push(self.region_world_rings(f[0], f[1])?);
        }
        solid_from_rings(&rings).ok().map(|o| o.volume())
    }

    /// sketchcraft E-37 (H-20261117-14): the live faces of sketches in the same context as
    /// `[sketch, region]` reached from it across shared edges (itself included), as
    /// `[sketch, region]` pairs: SketchUp's "select all connected" for loose faces.
    pub fn connected_faces(&self, sketch: u64, region: u64) -> Vec<[u64; 2]> {
        let Some(s0) = self.sketches.get(from_public::<SketchId>(sketch)) else {
            return Vec::new();
        };
        let parent = s0.parent;
        let mut keys: Vec<[u64; 2]> = Vec::new();
        let mut rings: Vec<Vec<Vec<DVec3>>> = Vec::new();
        for &k in &self.sketch_order {
            let s = &self.sketches[k];
            if s.parent != parent {
                continue;
            }
            for (rid, _) in s.regions() {
                let key = [to_public(k), to_public(rid)];
                if let Some(r) = self.region_world_rings(key[0], key[1]) {
                    keys.push(key);
                    rings.push(r);
                }
            }
        }
        let Some(start) = keys.iter().position(|&k| k == [sketch, region]) else {
            return Vec::new();
        };
        let (_, polys) = weld_rings(&rings);
        let group = face_groups(&polys)
            .into_iter()
            .find(|g| g.contains(&start))
            .unwrap_or_default();
        let mut out: Vec<[u64; 2]> = group.into_iter().map(|i| keys[i]).collect();
        out.sort();
        out
    }

    /// The world rings (outer first) of a live region, read only.
    fn region_world_rings(&self, sketch: u64, region: u64) -> Option<Vec<Vec<DVec3>>> {
        let s = self.sketches.get(from_public::<SketchId>(sketch))?;
        let r = s.region(from_public::<RegionId>(region))?;
        if r.consumed {
            return None;
        }
        let sp = *s.plane();
        let (o, h) = s.region_rings(r);
        Some(
            std::iter::once(&o)
                .chain(h.iter())
                .map(|ring| ring.iter().map(|&p| sp.to_world(p)).collect())
                .collect(),
        )
    }

    /// sketchcraft E-28: the world rings (outer first) of live regions `[sketch, region]`,
    /// each once, with their keys.
    #[allow(clippy::type_complexity)]
    fn face_rings(
        &mut self,
        faces: &[[u64; 2]],
    ) -> Result<(Vec<Vec<Vec<DVec3>>>, Vec<(SketchId, RegionId)>), KernelError> {
        let mut rings_of: Vec<Vec<Vec<DVec3>>> = Vec::new();
        let mut used: Vec<(SketchId, RegionId)> = Vec::new();
        for &[sk, rg] in faces {
            let sid = self.sketch_edit_key(sk)?;
            let rid = from_public::<RegionId>(rg);
            let s = &self.sketches[sid];
            let r = s.region(rid).ok_or(SketchError::NoSuchRegion(rg))?;
            if r.consumed {
                return Err(SketchError::RegionConsumed(rg).into());
            }
            if used.contains(&(sid, rid)) {
                continue;
            }
            let sp = *s.plane();
            let (o, h) = s.region_rings(r);
            let w = |ring: &Vec<DVec2>| ring.iter().map(|&p| sp.to_world(p)).collect::<Vec<_>>();
            rings_of.push(std::iter::once(&o).chain(h.iter()).map(w).collect());
            used.push((sid, rid));
        }
        Ok((rings_of, used))
    }

    /// sketchcraft E-25 (H-20261117-3): every edge (world) of the solids directly in the
    /// node `parent` (the open context), once per face loop side.
    fn solid_segments(&self, parent: Option<crate::ids::NodeId>) -> Vec<(DVec3, DVec3)> {
        let mut out = Vec::new();
        for (_, o) in self.objects.iter() {
            if o.membership().parent != parent {
                continue;
            }
            let polys = o.brep().to_polys();
            for f in &polys.faces {
                for ring in std::iter::once(&f.outer).chain(f.holes.iter()) {
                    for i in 0..ring.len() {
                        let (a, b) = (
                            polys.points[ring[i]],
                            polys.points[ring[(i + 1) % ring.len()]],
                        );
                        if a.distance(b) > PLANARITY {
                            out.push((a, b));
                        }
                    }
                }
            }
        }
        out
    }

    /// sketchcraft E-19: a straight stroke: the planes through it and an edge of another
    /// sketch touching one of its ends, in turn; on each where it closes a loop (with the
    /// edges there connected to it), it is drawn into that plane's sketch (made if need
    /// be). `sid` is left out (`before` is what it holds without the stroke). Returns
    /// those sketches and the regions closed there.
    fn close_on_other_planes(
        &mut self,
        sid: SketchId,
        pts: &[DVec3],
        parent: Option<crate::ids::NodeId>,
        before: &Sketch,
    ) -> Vec<(SketchId, Vec<RegionId>)> {
        let mut closed_on = Vec::new();
        let (a, b) = (pts[0], pts[pts.len() - 1]);
        let d = (b - a).normalize_or_zero();
        if d == DVec3::ZERO {
            return closed_on;
        }
        let own = *self.sketches[sid].plane();
        let mut tried: Vec<DVec3> = vec![own.normal];
        let mut touching: Vec<(DVec3, DVec3)> = Vec::new();
        for &k in &self.sketch_order {
            let s = if k == sid { before } else { &self.sketches[k] };
            if s.parent != parent {
                continue;
            }
            let sp = *s.plane();
            for (_, [va, vb]) in s.edges() {
                let (Some(p), Some(q)) = (s.vertex(va), s.vertex(vb)) else {
                    continue;
                };
                let (p, q) = (sp.to_world(p), sp.to_world(q));
                if [a, b]
                    .iter()
                    .any(|e| e.distance(p) <= PLANARITY || e.distance(q) <= PLANARITY)
                {
                    touching.push((p, q));
                }
            }
        }
        touching.extend(self.solid_segments(parent).into_iter().filter(|(p, q)| {
            [a, b]
                .iter()
                .any(|e| e.distance(*p) <= PLANARITY || e.distance(*q) <= PLANARITY)
        }));
        for (p, q) in touching {
            let n = d.cross(q - p);
            if n.length() <= 1e-9 * (q - p).length().max(1e-12) {
                continue;
            }
            let n = n.normalize();
            if tried.iter().any(|t| t.dot(n).abs() >= 1.0 - 1e-12) {
                continue;
            }
            tried.push(n);
            let Ok(plane) = Plane::new(a, n) else {
                continue;
            };
            let existing = self.sketch_order.iter().copied().find(|&k| {
                let sp = self.sketches[k].plane();
                k != sid
                    && self.sketches[k].parent == parent
                    && sp.normal.dot(plane.normal).abs() >= 1.0 - 1e-12
                    && sp.signed_distance(plane.origin).abs() <= PLANARITY
            });
            let mut trial = match existing {
                Some(k) => self.sketches[k].clone(),
                None => {
                    let mut s = Sketch::new(plane);
                    s.parent = parent;
                    s
                }
            };
            let tp = *trial.plane();
            let local: Vec<_> = pts.iter().map(|&p| tp.to_local(p)).collect();
            let Ok(mut rs) = trial.insert_polyline(&local, false) else {
                continue;
            };
            let mut skip = vec![sid];
            skip.extend(existing);
            let mut foreign = self.edges_on_plane(&tp, parent, &skip);
            foreign.extend(self.solid_closers(&tp, parent, pts));
            // The sketch the stroke was drawn into, without the stroke.
            let bp = *before.plane();
            for (_, [va, vb]) in before.edges() {
                if let (Some(x), Some(y)) = (before.vertex(va), before.vertex(vb)) {
                    let (x, y) = (bp.to_world(x), bp.to_world(y));
                    if tp.signed_distance(x).abs() <= PLANARITY
                        && tp.signed_distance(y).abs() <= PLANARITY
                    {
                        foreign.push((x, y));
                    }
                }
            }
            rs.extend(import_connected(&mut trial, &tp, pts, &foreign));
            if rs.is_empty() {
                continue;
            }
            rs.sort();
            rs.dedup();
            let k = match existing {
                Some(k) => {
                    self.sketches[k] = trial;
                    k
                }
                None => {
                    let k = self.sketches.insert(trial);
                    self.sketch_order.push(k);
                    k
                }
            };
            closed_on.push((k, rs));
        }
        closed_on
    }

    /// The object and the ordinal of its face with persistent id `face`.
    fn face_edit_target(
        &mut self,
        object: u64,
        face: u64,
    ) -> Result<(&mut Object, usize), KernelError> {
        let k = from_public::<ObjectId>(object);
        if !self.objects.contains_key(k) {
            return Err(FaceEditError::NoSuchObject(object).into());
        }
        self.editable_object(k)?;
        self.touch(k);
        let o = &mut self.objects[k];
        let fi = o
            .face_ordinal(face)
            .ok_or(FaceEditError::NoSuchFace(face))?;
        Ok((o, fi))
    }

    /// A collapsed solid's flat footprint goes back to its source sketch when it lies on
    /// that sketch's plane (the usual case: a box pushed back down onto its base).
    fn return_footprint(
        &mut self,
        sketch: SketchId,
        rings: &[Vec<DVec3>],
    ) -> Result<Vec<RegionId>, SketchError> {
        let Some(s) = self.sketches.get_mut(sketch) else {
            return Ok(Vec::new());
        };
        let plane = *s.plane();
        if rings
            .iter()
            .flatten()
            .any(|p| plane.signed_distance(*p).abs() > PLANARITY)
        {
            return Ok(Vec::new());
        }
        let rings2: Vec<Vec<DVec2>> = rings
            .iter()
            .map(|ring| ring.iter().map(|&p| plane.to_local(p)).collect())
            .collect();
        s.release_footprint(&rings2)
    }

    /// Lane 23: a solid pushed flat onto its source sketch gives back the curves of its rims
    /// that lie on the sketch's plane (the base's), as curves of the sketch again.
    fn return_rims(&mut self, sketch: SketchId, o: &Object) {
        let Some(s) = self.sketches.get_mut(sketch) else {
            return;
        };
        let plane = *s.plane();
        for &id in o.curves().keys() {
            let Some(info) = o.curve_info(id) else {
                continue;
            };
            let pts: Vec<DVec3> = info.points.iter().map(|p| DVec3::from_array(*p)).collect();
            if pts
                .iter()
                .any(|p| plane.signed_distance(*p).abs() > PLANARITY)
            {
                continue;
            }
            let local: Vec<DVec2> = pts.iter().map(|&p| plane.to_local(p)).collect();
            let centre = info.centre.map(|c| plane.to_local(DVec3::from_array(c)));
            let kind = if centre.is_some() || !info.kind.is_round() {
                info.kind
            } else {
                crate::curve::CurveKind::Polyline
            };
            if kind == crate::curve::CurveKind::Polyline && local.len() < 3 {
                continue;
            }
            s.add_curve(&local, info.closed, kind, centre, info.segments);
        }
    }

    /// Validate every object (topology + watertight), every sketch graph, and the
    /// document bookkeeping (groups name live objects, every object tag is listed).
    pub fn validate(&self) -> Result<(), TopologyError> {
        let dangling = |what: &'static str| TopologyError {
            violations: vec![Violation::DanglingKey(what)],
            report: Default::default(),
        };
        if let Err(e) = self
            .check_structure()
            .and_then(|_| self.check_looks())
            .and_then(|_| self.check_sections())
            .and_then(|_| self.check_sheets())
        {
            return Err(TopologyError {
                violations: vec![Violation::Structure(e)],
                report: Default::default(),
            });
        }
        for (_, o) in self.objects() {
            if o.tag().is_some_and(|t| !self.tags.contains_key(t)) {
                return Err(dangling("object tag is not listed"));
            }
            if o.material()
                .is_some_and(|m| !self.materials.contains_key(m))
            {
                return Err(dangling("object material is not defined"));
            }
        }
        for (_, m) in self.meshes() {
            if m.tag.as_deref().is_some_and(|t| !self.tags.contains_key(t))
                || m.material
                    .as_deref()
                    .is_some_and(|x| !self.materials.contains_key(x))
            {
                return Err(dangling("mesh tag or material is not listed"));
            }
            if let Err(e) = m.check() {
                return Err(dangling(e));
            }
        }
        for (_, o) in self.objects() {
            o.validate()?;
        }
        for (_, s) in self.sketches() {
            if let Err(e) = s.check() {
                return Err(TopologyError {
                    violations: vec![Violation::SketchInvalid(e)],
                    report: Default::default(),
                });
            }
        }
        Ok(())
    }

    pub fn hash(&self) -> u64 {
        crate::hash::model_hash(self)
    }

    pub fn hash_hex(&self) -> String {
        format!("{:016x}", self.hash())
    }

    pub fn stats(&self) -> ModelStats {
        let mut s = ModelStats {
            sketches: self.sketch_order.len(),
            regions: self.sketches().iter().map(|(_, s)| s.region_count()).sum(),
            objects: self.object_order.len(),
            watertight: true,
            hash: self.hash_hex(),
            ..Default::default()
        };
        for (_, o) in self.objects() {
            match o.validate() {
                Ok(r) => {
                    s.faces += r.faces;
                    s.edges += r.edges;
                    s.vertices += r.vertices;
                    s.holes += r.holes;
                    s.shells += r.shells.len();
                    s.watertight &= r.watertight;
                    for sh in &r.shells {
                        s.euler += sh.euler;
                        s.genus += sh.genus.unwrap_or(0);
                    }
                }
                Err(_) => s.watertight = false,
            }
            s.volume_m3 += o.volume();
        }
        s
    }
}

/// sketchcraft E-19: whether the points lie on one straight line.
fn straight(pts: &[DVec3]) -> bool {
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let d = b - a;
    d.length() > PLANARITY
        && pts
            .iter()
            .all(|&p| d.cross(p - a).length() <= PLANARITY * d.length())
}

/// sketchcraft E-19: into `s` (on `plane`), the `foreign` edges connected to the stroke
/// `pts` (through shared ends, edge after edge), kept only when they close a loop: the
/// regions their insertion bounds; `s` is left as it was when there are none.
/// sketchcraft E-30 / E-37: the faces (indices into `polys`) joined across shared sides,
/// group by group, each in ascending order.
fn face_groups(polys: &[Vec<Vec<usize>>]) -> Vec<Vec<usize>> {
    let mut by_edge: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (fi, f) in polys.iter().enumerate() {
        for r in f {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                by_edge.entry((a.min(b), a.max(b))).or_default().push(fi);
            }
        }
    }
    let mut seen = vec![false; polys.len()];
    let mut groups = Vec::new();
    for s in 0..polys.len() {
        if seen[s] {
            continue;
        }
        seen[s] = true;
        let (mut stack, mut g) = (vec![s], Vec::new());
        while let Some(fi) = stack.pop() {
            g.push(fi);
            for r in &polys[fi] {
                for i in 0..r.len() {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    for &o in &by_edge[&(a.min(b), a.max(b))] {
                        if !seen[o] {
                            seen[o] = true;
                            stack.push(o);
                        }
                    }
                }
            }
        }
        g.sort();
        groups.push(g);
    }
    groups
}

/// sketchcraft E-50: `o` without its point at `at`: the point must join exactly two other
/// points across all its faces (two edges) and lie at most [`MERGE_TOLERANCE`] off the line
/// between them; it leaves every ring it is in, and those faces refit their planes.
fn dissolved_point(o: &Object, at: DVec3) -> Result<Object, KernelError> {
    let refuse = |m: &str| not_a_volume(&format!("that point stays: {m}"));
    let mut p = o.brep().to_polys();
    let Some(v) = (0..p.points.len())
        .filter(|&i| p.points[i].distance(at) <= 1e-4)
        .min_by(|&a, &b| {
            p.points[a]
                .distance(at)
                .total_cmp(&p.points[b].distance(at))
        })
    else {
        return Err(refuse("there is no point of this solid there"));
    };
    let mut around: BTreeSet<usize> = BTreeSet::new();
    for f in &p.faces {
        for r in std::iter::once(&f.outer).chain(f.holes.iter()) {
            if let Some(k) = r.iter().position(|&x| x == v) {
                around.insert(r[(k + r.len() - 1) % r.len()]);
                around.insert(r[(k + 1) % r.len()]);
            }
        }
    }
    let [u, w] = around.iter().copied().collect::<Vec<_>>()[..] else {
        return Err(refuse(
            "it is a corner where three edges or more meet (only a point between two edges can go)",
        ));
    };
    let (pu, pv, pw) = (p.points[u], p.points[v], p.points[w]);
    let d = pw - pu;
    let t = (pv - pu).dot(d) / d.length_squared().max(1e-18);
    let off = (pv - (pu + d * t)).length();
    if !(0.0..=1.0).contains(&t) || off > MERGE_TOLERANCE {
        return Err(refuse(&format!(
            "its two edges are not in line ({:.0} cm off the line, at most {:.0} cm)",
            off * 100.0,
            MERGE_TOLERANCE * 100.0
        )));
    }
    for f in p.faces.iter_mut() {
        let mut hit = false;
        for r in std::iter::once(&mut f.outer).chain(f.holes.iter_mut()) {
            if let Some(k) = r.iter().position(|&x| x == v) {
                r.remove(k);
                hit = true;
            }
        }
        if hit {
            f.plane = None;
        }
    }
    if p.faces
        .iter()
        .any(|f| f.outer.len() < 3 || f.holes.iter().any(|h| h.len() < 3))
    {
        return Err(refuse("a face would be left with two corners"));
    }
    p.user_edges.retain(|&(a, b)| a != v && b != v);
    // The point leaves the list too (an unused point would be an isolated vertex).
    p.points.remove(v);
    let down = |x: usize| if x > v { x - 1 } else { x };
    for f in p.faces.iter_mut() {
        for r in std::iter::once(&mut f.outer).chain(f.holes.iter_mut()) {
            for x in r.iter_mut() {
                *x = down(*x);
            }
        }
    }
    p.user_edges = p
        .user_edges
        .iter()
        .map(|&(a, b)| (down(a), down(b)))
        .collect();
    let next = o.rebuilt(p).map_err(|e| refuse(&e.to_string()))?;
    next.validate()
        .map_err(|e| refuse(&format!("removing it would bend a face beside it ({e})")))?;
    Ok(next)
}

/// sketchcraft E-50: `o` with every removable point ([`dissolved_point`]) gone, the straightest
/// first, over again while any goes. Returns the solid and how many points went.
fn cleaned_points(o: &Object) -> (Object, u64) {
    let mut obj = o.clone();
    let mut removed = 0u64;
    let mut stuck: BTreeSet<[u64; 3]> = BTreeSet::new();
    let key = |q: DVec3| [q.x.to_bits(), q.y.to_bits(), q.z.to_bits()];
    for _ in 0..100_000 {
        let p = obj.brep().to_polys();
        let mut pair: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for f in &p.faces {
            for r in std::iter::once(&f.outer).chain(f.holes.iter()) {
                for k in 0..r.len() {
                    let (a, b) = (r[k], r[(k + 1) % r.len()]);
                    pair.entry(a).or_default().insert(b);
                    pair.entry(b).or_default().insert(a);
                }
            }
        }
        let mut cands: Vec<(f64, DVec3)> = pair
            .iter()
            .filter(|(_, n)| n.len() == 2)
            .filter_map(|(&v, n)| {
                let mut it = n.iter();
                let (pu, pw, pv) = (p.points[*it.next()?], p.points[*it.next()?], p.points[v]);
                let d = pw - pu;
                let t = (pv - pu).dot(d) / d.length_squared().max(1e-18);
                let off = (pv - (pu + d * t)).length();
                ((0.0..=1.0).contains(&t) && off <= MERGE_TOLERANCE && !stuck.contains(&key(pv)))
                    .then_some((off, pv))
            })
            .collect();
        cands.sort_by(|a, b| a.0.total_cmp(&b.0));
        let Some(&(_, at)) = cands.first() else { break };
        match dissolved_point(&obj, at) {
            Ok(next) => {
                obj = next;
                removed += 1;
            }
            Err(_) => {
                stuck.insert(key(at));
            }
        }
    }
    (obj, removed)
}

fn merged_object(o: &Object, faces: &[u64]) -> Result<(Object, u64), KernelError> {
    let refuse = |m: &str| not_a_volume(&format!("these faces cannot be merged: {m}"));
    let mut p = o.brep().to_polys();
    let sel: Vec<usize> = (0..p.faces.len())
        .filter(|&i| p.faces[i].id.is_some_and(|id| faces.contains(&id)))
        .collect();
    if sel.len() < 2 {
        return Err(refuse("select two faces of one solid or more"));
    }
    if sel.iter().any(|&i| !p.faces[i].holes.is_empty()) {
        return Err(refuse("a face with a hole"));
    }
    let corners: BTreeSet<usize> = sel
        .iter()
        .flat_map(|&i| p.faces[i].outer.iter().copied())
        .collect();
    let pts: Vec<DVec3> = corners.iter().map(|&v| p.points[v]).collect();
    let plane = Plane::fit(&pts).map_err(|_| refuse("their corners fit no plane"))?;
    let off = pts
        .iter()
        .map(|&q| plane.signed_distance(q).abs())
        .fold(0.0, f64::max);
    if off > MERGE_TOLERANCE {
        return Err(refuse(&format!(
            "they are {:.0} cm out of one plane (at most {:.0} cm)",
            off * 100.0,
            MERGE_TOLERANCE * 100.0
        )));
    }
    // Onto the plane: straight up or down when it is not upright (walls stay upright).
    let n = plane.normal;
    for &v in &corners {
        let q = p.points[v];
        let d = plane.signed_distance(q);
        p.points[v] = if n.z.abs() > 0.1 {
            q - DVec3::Z * (d / n.z)
        } else {
            q - n * d
        };
    }
    // The merged outline: the selected faces' sides not shared within the selection.
    let mut inner: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut sides: Vec<(usize, usize)> = Vec::new();
    for &i in &sel {
        let r = &p.faces[i].outer;
        for k in 0..r.len() {
            sides.push((r[k], r[(k + 1) % r.len()]));
        }
    }
    let set: BTreeSet<(usize, usize)> = sides.iter().copied().collect();
    for &(a, b) in &sides {
        if set.contains(&(b, a)) {
            inner.insert((a, b));
        }
    }
    let mut rim: Vec<(usize, usize)> = sides.into_iter().filter(|e| !inner.contains(e)).collect();
    if inner.is_empty() {
        return Err(refuse("they do not touch along an edge"));
    }
    let mut loops: Vec<Vec<usize>> = Vec::new();
    while let Some((a, b)) = rim.pop() {
        let mut lp = vec![a];
        let mut at = b;
        while at != a {
            let Some(k) = rim.iter().position(|e| e.0 == at) else {
                return Err(refuse("their outline does not close"));
            };
            lp.push(at);
            at = rim.swap_remove(k).1;
        }
        loops.push(lp);
    }
    let area = |r: &[usize], pts: &[DVec3]| -> f64 {
        let mut a = DVec3::ZERO;
        for k in 0..r.len() {
            a += pts[r[k]].cross(pts[r[(k + 1) % r.len()]]);
        }
        a.dot(n).abs() / 2.0
    };
    loops.sort_by(|x, y| area(y, &p.points).total_cmp(&area(x, &p.points)));
    let outer = loops.remove(0);
    let id = p.faces[sel[0]].id;
    // Faces that kept a moved corner fit their plane afresh (and are refused if bent).
    let moved: BTreeSet<usize> = corners;
    for f in p.faces.iter_mut() {
        if f.outer
            .iter()
            .chain(f.holes.iter().flatten())
            .any(|v| moved.contains(v))
        {
            f.plane = None;
        }
    }
    let mut merged = crate::brep::PolyFace::new(outer, loops);
    merged.id = id;
    let mut keep: Vec<crate::brep::PolyFace> = p
        .faces
        .iter()
        .enumerate()
        .filter(|(i, _)| !sel.contains(i))
        .map(|(_, f)| f.clone())
        .collect();
    keep.push(merged);
    p.faces = keep;
    p.user_edges.clear();
    let next = o.rebuilt(p).map_err(|e| refuse(&e.to_string()))?;
    next.validate().map_err(|_| {
        refuse("putting them in one plane would bend a face beside them (merge it too, or split it first)")
    })?;
    Ok((next, id.unwrap_or(0)))
}

/// sketchcraft E-42 (H-20261117-19): `o` with its faces merged pan by pan: from each face
/// (largest first), its neighbours across an edge join while they face within 5° of it and
/// all their corners stay within [`MERGE_TOLERANCE`] of one plane; a group of two or more is
/// merged ([`merged_object`]) unless that would bend a face beside it. Over again until
/// nothing merges. Returns the solid and how many faces went.
fn auto_merged(o: &Object) -> (Object, u64) {
    let cos_max = 5f64.to_radians().cos();
    let mut obj = o.clone();
    let mut gone = 0u64;
    let mut refused: BTreeSet<Vec<u64>> = BTreeSet::new();
    'again: for _ in 0..10_000 {
        let p = obj.brep().to_polys();
        let n = p.faces.len();
        let planes: Vec<Option<Plane>> = (0..n).map(|i| p.plane(i)).collect();
        let mut by_side: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for (i, f) in p.faces.iter().enumerate() {
            for r in std::iter::once(&f.outer).chain(f.holes.iter()) {
                for k in 0..r.len() {
                    by_side.insert((r[k], r[(k + 1) % r.len()]), i);
                }
            }
        }
        let neighbours = |i: usize| -> BTreeSet<usize> {
            let f = &p.faces[i];
            let mut out = BTreeSet::new();
            for r in std::iter::once(&f.outer).chain(f.holes.iter()) {
                for k in 0..r.len() {
                    if let Some(&j) = by_side.get(&(r[(k + 1) % r.len()], r[k]))
                        && j != i
                    {
                        out.insert(j);
                    }
                }
            }
            out
        };
        let area = |i: usize| -> f64 {
            let r = &p.faces[i].outer;
            let mut a = DVec3::ZERO;
            for k in 0..r.len() {
                a += p.points[r[k]].cross(p.points[r[(k + 1) % r.len()]]);
            }
            a.length() / 2.0
        };
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| area(b).total_cmp(&area(a)));
        for &seed in &order {
            let Some(sp) = planes[seed] else { continue };
            if !p.faces[seed].holes.is_empty() {
                continue;
            }
            let mut group: Vec<usize> = vec![seed];
            let mut corners: Vec<DVec3> =
                p.faces[seed].outer.iter().map(|&v| p.points[v]).collect();
            let mut grew = true;
            while grew {
                grew = false;
                let around: BTreeSet<usize> = group
                    .iter()
                    .flat_map(|&g| neighbours(g))
                    .filter(|j| !group.contains(j))
                    .collect();
                for j in around {
                    let Some(pj) = planes[j] else { continue };
                    if pj.normal.dot(sp.normal) < cos_max || !p.faces[j].holes.is_empty() {
                        continue;
                    }
                    let mut trial = corners.clone();
                    trial.extend(p.faces[j].outer.iter().map(|&v| p.points[v]));
                    let fits = Plane::fit(&trial).is_ok_and(|pl| {
                        trial
                            .iter()
                            .all(|&q| pl.signed_distance(q).abs() <= MERGE_TOLERANCE)
                    });
                    if fits {
                        group.push(j);
                        corners = trial;
                        grew = true;
                    }
                }
            }
            if group.len() < 2 {
                continue;
            }
            let mut ids: Vec<u64> = group.iter().filter_map(|&g| p.faces[g].id).collect();
            ids.sort();
            if refused.contains(&ids) {
                continue;
            }
            match merged_object(&obj, &ids) {
                Ok((next, _)) => {
                    gone += (ids.len() - 1) as u64;
                    obj = next;
                    continue 'again;
                }
                Err(_) => {
                    refused.insert(ids);
                }
            }
        }
        break;
    }
    (obj, gone)
}

/// sketchcraft E-40: how far apart (metres) the pieces of an imported roof may be and still
/// be sewn together (GeoHVAC's are 1 to 4 cm apart: 3 cm left building A2CDD7E9 open).
pub const UNIFY_GAP: f64 = 0.05;

/// sketchcraft E-41: how far (metres) faces may lie out of one plane and still be merged.
pub const MERGE_TOLERANCE: f64 = 0.05;

/// sketchcraft E-40 (H-20261117-17): a 2.5D building (walls upright, under a roof) rebuilt
/// from an imported mesh's roof: its faces facing up or down by more than about 12° and
/// above the floor band, welded within `gap` (and their corners brought to one plan
/// position within `gap`), each outline edge of the roof gets a vertical wall: down to the
/// floor, or, where another roof edge lies under or over it in plan (a step between two roof
/// parts), up or down to that edge; the floor is flat at the walls' foot (the median height
/// of the lowest half metre of points, or the ground, z = 0, for a roof alone). The mesh's own walls and floor are not used.
fn unify_25d(
    positions: &[DVec3],
    faces: &[Vec<Vec<u32>>],
    gap: f64,
) -> Result<(Object, Vec<String>), KernelError> {
    let refuse = |m: &str| not_a_volume(&format!("this mesh cannot be unified: {m}"));
    if positions.is_empty() || faces.is_empty() {
        return Err(refuse("it is empty"));
    }
    // The floor: the median of the points within half a metre of the lowest.
    let zmin = positions.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
    let mut low: Vec<f64> = positions
        .iter()
        .map(|p| p.z)
        .filter(|&z| z <= zmin + 0.5)
        .collect();
    low.sort_by(f64::total_cmp);
    // A roof alone (nothing near the ground): the floor is the ground, z = 0.
    let z0 = if zmin > 0.5 { 0.0 } else { low[low.len() / 2] };
    // The roof faces (outer rings only).
    let mut roof_pts: Vec<DVec3> = Vec::new();
    let mut roof_rings: Vec<Vec<usize>> = Vec::new();
    for f in faces {
        let Some(outer) = f.first() else { continue };
        let ring: Vec<DVec3> = outer
            .iter()
            .filter_map(|&i| positions.get(i as usize).copied())
            .collect();
        if ring.len() < 3 {
            continue;
        }
        let Some(n) = newell(&ring) else { continue };
        let c = ring.iter().fold(DVec3::ZERO, |a, &p| a + p) / ring.len() as f64;
        if n.z.abs() <= 0.2 || c.z <= z0 + 0.3 {
            continue;
        }
        let base = roof_pts.len();
        roof_pts.extend(ring.iter().copied());
        roof_rings.push((base..roof_pts.len()).collect());
    }
    if roof_rings.is_empty() {
        return Err(refuse("no roof faces found above the floor"));
    }
    // Weld in space, then bring corners close in plan to one plan position.
    let (mut pts, map) = {
        let (p, m) = crate::meshbuild::weld_points(&roof_pts, gap);
        (p, m)
    };
    for i in 0..pts.len() {
        for j in 0..i {
            if pts[j].truncate().distance(pts[i].truncate()) <= gap {
                pts[i].x = pts[j].x;
                pts[i].y = pts[j].y;
                break;
            }
        }
    }
    let mut roof: Vec<Vec<usize>> = Vec::new();
    for r in &roof_rings {
        let mut q: Vec<usize> = Vec::new();
        for &i in r {
            let j = map[i] as usize;
            if q.last() != Some(&j) {
                q.push(j);
            }
        }
        while q.len() > 1 && q.first() == q.last() {
            q.pop();
        }
        if q.len() < 3 {
            continue;
        }
        let w: Vec<DVec3> = q.iter().map(|&i| pts[i]).collect();
        let Some(n) = newell(&w) else { continue };
        if n.z < 0.0 {
            q.reverse();
        }
        roof.push(q);
    }
    let mut uses: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for r in &roof {
        for k in 0..r.len() {
            let (a, b) = (r[k], r[(k + 1) % r.len()]);
            *uses.entry((a.min(b), a.max(b))).or_default() += 1;
        }
    }
    if uses.values().any(|&n| n > 2) {
        return Err(refuse(
            "its roof faces overlap (an edge shared by three faces)",
        ));
    }
    // The roof's outline, in its faces' direction (anticlockwise seen from above).
    let mut rim: Vec<(DVec3, DVec3)> = Vec::new();
    for r in &roof {
        for k in 0..r.len() {
            let (a, b) = (r[k], r[(k + 1) % r.len()]);
            if uses[&(a.min(b), a.max(b))] == 1 {
                rim.push((pts[a], pts[b]));
            }
        }
    }
    // Cut each outline edge where another's corner lies over or under it in plan (its height
    // there is the edge's own, interpolated), so steps pair edge for edge.
    let corners: Vec<DVec3> = rim.iter().flat_map(|&(a, b)| [a, b]).collect();
    let mut cut: Vec<(DVec3, DVec3)> = Vec::new();
    for &(a, b) in &rim {
        let (a2, d2) = (a.truncate(), (b - a).truncate());
        let len2 = d2.length_squared();
        let mut on: Vec<f64> = Vec::new();
        if len2 > 0.0 {
            for c in &corners {
                let t = (c.truncate() - a2).dot(d2) / len2;
                if t > 1e-9 && t < 1.0 - 1e-9 && (a2 + d2 * t).distance(c.truncate()) <= 1e-9 {
                    on.push(t);
                }
            }
        }
        on.sort_by(f64::total_cmp);
        on.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
        let mut prev = a;
        for t in on {
            let p = a + (b - a) * t;
            cut.push((prev, p));
            prev = p;
        }
        cut.push((prev, b));
    }
    let same_xy = |p: DVec3, q: DVec3| p.truncate().distance(q.truncate()) <= 1e-9;
    let mut paired = vec![false; cut.len()];
    let mut rings: Vec<Vec<Vec<DVec3>>> = roof
        .iter()
        .map(|r| vec![r.iter().map(|&i| pts[i]).collect()])
        .collect();
    let mut steps = 0usize;
    for i in 0..cut.len() {
        if paired[i] {
            continue;
        }
        let (a, b) = cut[i];
        if let Some(j) =
            (i + 1..cut.len()).find(|&j| !paired[j] && same_xy(a, cut[j].1) && same_xy(b, cut[j].0))
        {
            paired[i] = true;
            paired[j] = true;
            let (c, d) = cut[j];
            let mut ring: Vec<DVec3> = Vec::new();
            for p in [a, b, c, d] {
                if ring.last().is_none_or(|q: &DVec3| q.distance(p) > 1e-12) {
                    ring.push(p);
                }
            }
            if ring.len() > 1 && ring[0].distance(ring[ring.len() - 1]) <= 1e-12 {
                ring.pop();
            }
            if ring.len() >= 3 {
                rings.push(vec![ring]);
                steps += 1;
            }
        }
    }
    // Outer walls, down to the floor, and the floor's outline.
    let foot = |p: DVec3| DVec3::new(p.x, p.y, z0);
    let outer: Vec<(DVec3, DVec3)> = (0..cut.len())
        .filter(|&i| !paired[i])
        .map(|i| cut[i])
        .collect();
    for &(a, b) in &outer {
        if a.z <= z0 + 1e-6 || b.z <= z0 + 1e-6 {
            return Err(refuse("a roof edge is at or below the floor"));
        }
        rings.push(vec![vec![a, b, foot(b), foot(a)]]);
    }
    // Floor loops: outline edges chained end to start in plan.
    let mut left: Vec<(DVec3, DVec3)> = outer.clone();
    let mut loops: Vec<Vec<DVec3>> = Vec::new();
    while let Some(first) = left.pop() {
        let mut lp = vec![foot(first.0)];
        let mut at = first.1;
        let mut guard = 0;
        while !same_xy(at, first.0) {
            let Some(k) = left.iter().position(|e| same_xy(e.0, at)) else {
                return Err(refuse("its roof outline does not close in plan"));
            };
            lp.push(foot(at));
            at = left.swap_remove(k).1;
            guard += 1;
            if guard > outer.len() {
                return Err(refuse("its roof outline does not close in plan"));
            }
        }
        loops.push(lp);
    }
    let area = |r: &[DVec3]| -> f64 {
        (0..r.len())
            .map(|i| {
                let (p, q) = (r[i], r[(i + 1) % r.len()]);
                p.x * q.y - q.x * p.y
            })
            .sum::<f64>()
            / 2.0
    };
    let inside = |p: DVec3, r: &[DVec3]| -> bool {
        let mut c = false;
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                c = !c;
            }
        }
        c
    };
    let (outs, holes): (Vec<Vec<DVec3>>, Vec<Vec<DVec3>>) =
        loops.into_iter().partition(|l| area(l) > 0.0);
    if outs.is_empty() {
        return Err(refuse("its roof has no outline"));
    }
    let mut floors: Vec<Vec<Vec<DVec3>>> = outs.iter().map(|o| vec![o.clone()]).collect();
    for h in holes {
        if let Some(f) = floors.iter_mut().find(|f| inside(h[0], &f[0])) {
            f.push(h);
        } else {
            return Err(refuse("a hole in its outline lies outside it"));
        }
    }
    rings.extend(floors);
    let obj = solid_from_rings(&rings)
        .map_err(|e| refuse(&e.to_string().replace("these faces ", "the rebuilt faces ")))?;
    // sketchcraft E-42: the roof's triangles merged pan by pan, the walls' seams too.
    let before = obj.brep().face_count();
    let (obj, merged) = auto_merged(&obj);
    let mut notes = vec![format!(
        "rebuilt from its roof: {} roof face{}, {} step wall{} between roof parts, floor at {:.2} m; roof pieces within {:.0} cm sewn together; its own walls and floor were left out",
        roof.len(),
        if roof.len() == 1 { "" } else { "s" },
        steps,
        if steps == 1 { "" } else { "s" },
        z0,
        gap * 100.0
    )];
    if merged > 0 {
        notes.push(format!(
            "{merged} of its {before} faces merged into their neighbours (faces within 5° and 5 cm of one plane made one: corners moved 5 cm at most); {} faces now",
            obj.brep().face_count()
        ));
    }
    Ok((obj, notes))
}

fn not_a_volume(m: &str) -> KernelError {
    DocumentError::NotAVolume(m.into()).into()
}

/// sketchcraft E-28: rings (world) welded to shared corners, each side split at the corners
/// lying on it (T-junctions): the points and, per face, its rings as indices.
#[allow(clippy::type_complexity)]
fn weld_rings(rings_of: &[Vec<Vec<DVec3>>]) -> (Vec<DVec3>, Vec<Vec<Vec<usize>>>) {
    // Review of 17 nov. 2026 (constat 5): corners found through a grid of cells twice the
    // weld distance (the 27 cells around), not by a search through every point.
    let cell = 2.0 * PLANARITY;
    let key = |p: DVec3| {
        (
            (p.x / cell).floor() as i64,
            (p.y / cell).floor() as i64,
            (p.z / cell).floor() as i64,
        )
    };
    let mut grid: std::collections::HashMap<(i64, i64, i64), Vec<usize>> =
        std::collections::HashMap::new();
    let mut points: Vec<DVec3> = Vec::new();
    let mut idx: Vec<Vec<Vec<usize>>> = Vec::new();
    for f in rings_of {
        let mut fr = Vec::new();
        for r in f {
            let mut ri = Vec::new();
            for &p in r {
                let (kx, ky, kz) = key(p);
                let mut found = None;
                'near: for dx in -1..=1 {
                    for dy in -1..=1 {
                        for dz in -1..=1 {
                            if let Some(v) = grid.get(&(kx + dx, ky + dy, kz + dz))
                                && let Some(&i) =
                                    v.iter().find(|&&i| points[i].distance(p) <= PLANARITY)
                            {
                                found = Some(i);
                                break 'near;
                            }
                        }
                    }
                }
                let i = match found {
                    Some(i) => i,
                    None => {
                        points.push(p);
                        grid.entry((kx, ky, kz)).or_default().push(points.len() - 1);
                        points.len() - 1
                    }
                };
                if ri.last() != Some(&i) {
                    ri.push(i);
                }
            }
            fr.push(ri);
        }
        idx.push(fr);
    }
    // The corners lying on a side: only those within its x range are looked at.
    let mut by_x: Vec<usize> = (0..points.len()).collect();
    by_x.sort_by(|&i, &j| points[i].x.total_cmp(&points[j].x));
    let split = |ring: &Vec<usize>| -> Vec<usize> {
        let mut out = Vec::new();
        for i in 0..ring.len() {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            out.push(a);
            let (pa, d) = (points[a], points[b] - points[a]);
            let (lo, hi) = (
                pa.x.min(points[b].x) - PLANARITY,
                pa.x.max(points[b].x) + PLANARITY,
            );
            let from = by_x.partition_point(|&j| points[j].x < lo);
            let to = by_x.partition_point(|&j| points[j].x <= hi);
            let mut on: Vec<(f64, usize)> = by_x[from..to]
                .iter()
                .copied()
                .filter(|&j| j != a && j != b)
                .filter_map(|j| {
                    let t = (points[j] - pa).dot(d) / d.length_squared();
                    (t > 0.0 && t < 1.0 && (pa + d * t).distance(points[j]) <= PLANARITY)
                        .then_some((t, j))
                })
                .collect();
            on.sort_by(|x, y| x.0.total_cmp(&y.0));
            out.extend(on.into_iter().map(|(_, j)| j));
        }
        out
    };
    let polys = idx.iter().map(|f| f.iter().map(&split).collect()).collect();
    (points, polys)
}

/// sketchcraft E-28: a solid from face rings (world, outer first) that close a volume:
/// welded, turned to face outward, built and checked; else why not.
fn solid_from_rings(rings_of: &[Vec<Vec<DVec3>>]) -> Result<Object, KernelError> {
    let refuse = not_a_volume;
    let (points, mut polys) = weld_rings(rings_of);
    // Every side shared by exactly two faces, run once each way (turn faces to agree).
    let mut uses: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (fi, f) in polys.iter().enumerate() {
        for r in f {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                uses.entry((a.min(b), a.max(b))).or_default().push(fi);
            }
        }
    }
    if uses.values().any(|v| v.len() != 2) {
        return Err(refuse(
            "these faces do not close a volume: every edge must join exactly two of them",
        ));
    }
    let dir = |f: &Vec<Vec<usize>>, a: usize, b: usize| -> bool {
        f.iter()
            .any(|r| (0..r.len()).any(|i| r[i] == a && r[(i + 1) % r.len()] == b))
    };
    let mut flip: Vec<Option<bool>> = vec![None; polys.len()];
    let mut stack = vec![0usize];
    flip[0] = Some(false);
    while let Some(fi) = stack.pop() {
        for (&(a, b), v) in &uses {
            if !v.contains(&fi) {
                continue;
            }
            let other = if v[0] == fi { v[1] } else { v[0] };
            if other == fi {
                return Err(refuse("a face meets itself along an edge"));
            }
            let mine = dir(&polys[fi], a, b) != flip[fi].unwrap_or(false);
            let theirs = dir(&polys[other], a, b);
            let want = mine == theirs; // same way: the other must turn
            match flip[other] {
                None => {
                    flip[other] = Some(want);
                    stack.push(other);
                }
                Some(f) if f != want => {
                    return Err(refuse(
                        "these faces cannot all face outward (a twisted shell)",
                    ));
                }
                _ => {}
            }
        }
    }
    if flip.iter().any(Option::is_none) {
        return Err(refuse(
            "these faces make more than one piece: select one closed volume",
        ));
    }
    for (f, fl) in polys.iter_mut().zip(&flip) {
        if *fl == Some(true) {
            for r in f.iter_mut() {
                r.reverse();
            }
        }
    }
    let volume: f64 = polys
        .iter()
        .flatten()
        .map(|r| {
            (1..r.len().saturating_sub(1))
                .map(|i| points[r[0]].dot(points[r[i]].cross(points[r[i + 1]])))
                .sum::<f64>()
        })
        .sum::<f64>()
        / 6.0;
    // Review (constat 4): a volume, not a length, decides: PLANARITY³ (1e-18 m³).
    if volume.abs() <= PLANARITY * PLANARITY * PLANARITY {
        return Err(refuse("these faces enclose no volume"));
    }
    if volume < 0.0 {
        for r in polys.iter_mut().flatten() {
            r.reverse();
        }
    }
    let pfaces: Vec<crate::brep::PolyFace> = polys
        .into_iter()
        .map(|mut f| {
            let outer = f.remove(0);
            crate::brep::PolyFace::new(outer, f)
        })
        .collect();
    let brep = crate::brep::Brep::build(&crate::brep::Polys {
        points,
        faces: pfaces,
        user_edges: Default::default(),
    })
    .map_err(|e| refuse(&format!("these faces do not make a solid: {e}")))?;
    let obj = Object::from_brep(brep);
    if !obj.watertight || obj.validate().is_err() {
        return Err(refuse("these faces do not make a closed solid"));
    }
    Ok(obj)
}

/// sketchcraft E-28: the normal of a planar ring (Newell), if it has an area.
fn newell(ring: &[DVec3]) -> Option<DVec3> {
    let mut n = DVec3::ZERO;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        n += DVec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    (n.length() > 1e-12).then(|| n.normalize())
}

fn import_connected(
    s: &mut Sketch,
    plane: &Plane,
    pts: &[DVec3],
    foreign: &[(DVec3, DVec3)],
) -> Vec<RegionId> {
    let near = |p: DVec3, q: DVec3| p.distance(q) <= PLANARITY;
    let mut reach: Vec<DVec3> = pts.to_vec();
    let mut used = vec![false; foreign.len()];
    let mut picked = Vec::new();
    loop {
        let mut grew = false;
        for (i, &(a, b)) in foreign.iter().enumerate() {
            if used[i] || !reach.iter().any(|&r| near(r, a) || near(r, b)) {
                continue;
            }
            used[i] = true;
            picked.push((a, b));
            reach.push(a);
            reach.push(b);
            grew = true;
        }
        if !grew {
            break;
        }
    }
    if picked.is_empty() {
        return Vec::new();
    }
    let mut trial = s.clone();
    let mut regions = Vec::new();
    for (a, b) in picked {
        let (la, lb) = (plane.to_local(a), plane.to_local(b));
        if let (Some(va), Some(vb)) = (trial.vertex_at(la), trial.vertex_at(lb))
            && (va == vb || trial.has_edge(va, vb))
        {
            continue;
        }
        if let Ok(more) = trial.insert_segment(la, lb) {
            regions.extend(more);
        }
    }
    if regions.is_empty() {
        return Vec::new();
    }
    *s = trial;
    regions.sort();
    regions.dedup();
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUND: u64 = 1;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Op {
        Op::AddRectangle {
            sketch: GROUND,
            x,
            y,
            w,
            h,
        }
    }

    fn first_region(m: &Model) -> u64 {
        to_public(m.sketch(GROUND).unwrap().regions().next().unwrap().0)
    }

    #[test]
    fn spike_box() {
        let mut m = Model::with_ground_sketch();
        let r = m.apply(&rect(0.0, 0.0, 0.6096, 0.4064)).unwrap();
        assert_eq!(r, OpOutcome::SketchEdited { regions: vec![1] });
        assert_eq!(
            m.apply(&Op::Extrude {
                sketch: GROUND,
                region: 1,
                distance: 0.3048
            })
            .unwrap(),
            OpOutcome::ObjectCreated { object: 1 }
        );
        let st = m.stats();
        assert!(st.watertight);
        assert_eq!(st.regions, 0, "the extruded region left the sketch");
        assert_eq!(
            (st.faces, st.edges, st.vertices, st.euler, st.genus),
            (6, 12, 8, 2, 0)
        );
        assert!((st.volume_m3 - 0.6096 * 0.4064 * 0.3048).abs() < 1e-12);
    }

    #[test]
    fn refused_ops_roll_back() {
        let mut m = Model::with_ground_sketch();
        m.apply(&rect(0.0, 0.0, 1.0, 1.0)).unwrap();
        let before = m.hash();
        for op in [
            Op::Extrude {
                sketch: GROUND,
                region: 1,
                distance: 0.0,
            },
            Op::Extrude {
                sketch: GROUND,
                region: 7,
                distance: 1.0,
            },
            Op::Extrude {
                sketch: 3,
                region: 1,
                distance: 1.0,
            },
            Op::PushPull {
                object: 1,
                face: 0,
                distance: 1.0,
            },
            rect(0.5, 0.5, 0.0, 1.0),
            Op::InsertSegment {
                sketch: GROUND,
                a: [0.0, 0.0],
                b: [f64::NAN, 1.0],
            },
            Op::NewSketch {
                origin: [0.0; 3],
                normal: [0.0; 3],
            },
        ] {
            assert!(m.apply(&op).is_err(), "{op:?} should be refused");
            assert_eq!(m.hash(), before, "{op:?} must not change the model");
        }
    }

    #[test]
    fn extrude_then_push_back_restores_the_sketch() {
        let mut m = Model::with_ground_sketch();
        m.apply(&rect(0.0, 0.0, 1.0, 2.0)).unwrap();
        let sketch_only = m.hash();
        m.apply(&Op::Extrude {
            sketch: GROUND,
            region: 1,
            distance: 0.5,
        })
        .unwrap();
        assert!(
            m.apply(&Op::Extrude {
                sketch: GROUND,
                region: 1,
                distance: 0.5
            })
            .is_err(),
            "region already consumed"
        );
        let out = m
            .apply(&Op::PushPull {
                object: 1,
                face: 2,
                distance: -0.5,
            })
            .unwrap();
        assert!(matches!(out, OpOutcome::ObjectCollapsed { ref regions } if regions.len() == 1));
        assert_eq!(m.hash(), sketch_only);
        assert!(m.objects().is_empty());
    }

    #[test]
    fn crossing_rectangles_extrude_one_piece() {
        let mut m = Model::with_ground_sketch();
        m.apply(&rect(0.0, 0.0, 2.0, 2.0)).unwrap();
        let out = m.apply(&rect(1.0, 1.0, 2.0, 2.0)).unwrap();
        let OpOutcome::SketchEdited { regions } = out else {
            panic!("{out:?}")
        };
        assert_eq!(regions.len(), 2);
        assert_eq!(m.stats().regions, 3);
        // Extrude the overlap square (area 1).
        let s = m.sketch(GROUND).unwrap();
        let overlap = s
            .regions()
            .find(|(_, r)| (s.region_area(r) - 1.0).abs() < 1e-12)
            .map(|(id, _)| to_public(id))
            .unwrap();
        m.apply(&Op::Extrude {
            sketch: GROUND,
            region: overlap,
            distance: 1.0,
        })
        .unwrap();
        let st = m.stats();
        assert_eq!(st.regions, 2, "the L-shaped neighbours stay");
        assert!((st.volume_m3 - 1.0).abs() < 1e-12);
        assert!(
            m.sketch(GROUND)
                .unwrap()
                .region(from_public(first_region(&m)))
                .is_some()
        );
    }

    #[test]
    fn display_numbers_are_never_reused() {
        let mut m = Model::with_ground_sketch();
        m.apply(&rect(0.0, 0.0, 1.0, 1.0)).unwrap();
        m.apply(&Op::Extrude {
            sketch: GROUND,
            region: first_region(&m),
            distance: 1.0,
        })
        .unwrap();
        m.apply(&Op::DeleteObject { object: 1 }).unwrap();
        m.apply(&rect(3.0, 0.0, 1.0, 1.0)).unwrap();
        let out = m
            .apply(&Op::Extrude {
                sketch: GROUND,
                region: first_region(&m),
                distance: 1.0,
            })
            .unwrap();
        let OpOutcome::ObjectCreated { object } = out else {
            panic!("{out:?}")
        };
        assert_eq!(
            object,
            (1 << 32) | 1,
            "the slot is reused, the public id says so"
        );
        assert_eq!(
            m.object(object).unwrap().number(),
            2,
            "the display number is not"
        );
        let (lo, hi) = m.object(object).unwrap().bounds();
        assert_eq!(
            (lo, hi),
            (DVec3::new(3.0, 0.0, 0.0), DVec3::new(4.0, 1.0, 1.0))
        );
    }

    #[test]
    fn display_numbers_survive_a_save_and_reopen() {
        let mut m = Model::with_ground_sketch();
        for x in [0.0, 3.0, 6.0] {
            m.apply(&rect(x, 0.0, 1.0, 1.0)).unwrap();
            m.apply(&Op::Extrude {
                sketch: GROUND,
                region: first_region(&m),
                distance: 1.0,
            })
            .unwrap();
        }
        // Delete the last one: its number (3) must not come back after a reopen.
        let last = *m.object_order.last().unwrap();
        m.apply(&Op::DeleteObject {
            object: to_public(last),
        })
        .unwrap();
        let snap = m.snapshot();
        assert_eq!(snap.last_number, 3);
        let mut back = Model::from_snapshot(&snap).unwrap();
        let numbers: Vec<u64> = back
            .object_order
            .iter()
            .map(|&k| back.objects[k].number())
            .collect();
        assert_eq!(numbers, vec![1, 2]);
        back.apply(&rect(9.0, 0.0, 1.0, 1.0)).unwrap();
        let out = back
            .apply(&Op::Extrude {
                sketch: GROUND,
                region: first_region(&back),
                distance: 1.0,
            })
            .unwrap();
        let OpOutcome::ObjectCreated { object } = out else {
            panic!("{out:?}")
        };
        assert_eq!(back.object(object).unwrap().number(), 4);

        // A file without numbers (an older writer) is numbered in saved order.
        let mut bare = m.snapshot();
        for o in &mut bare.objects {
            o.number = None;
        }
        bare.last_number = 0;
        let back = Model::from_snapshot(&bare).unwrap();
        let numbers: Vec<u64> = back
            .object_order
            .iter()
            .map(|&k| back.objects[k].number())
            .collect();
        assert_eq!(numbers, vec![1, 2]);
    }

    #[test]
    fn ops_round_trip_through_json() {
        let ops = vec![
            Op::NewSketch {
                origin: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            },
            rect(0.0, 0.0, 1.0, 1.0),
            Op::InsertSegment {
                sketch: GROUND,
                a: [0.5, 0.0],
                b: [0.5, 1.0],
            },
            Op::AddPolyline {
                sketch: GROUND,
                points: vec![[2.0, 0.0], [3.0, 0.0], [2.5, 1.0]],
                closed: true,
            },
            Op::Extrude {
                sketch: GROUND,
                region: 1,
                distance: 0.25,
            },
            Op::PushPull {
                object: 1,
                face: 2,
                distance: 0.1,
            },
        ];
        let json = serde_json::to_string(&ops).unwrap();
        assert!(json.contains(r#""op":"add_rectangle""#));
        assert!(json.contains(r#""op":"insert_segment""#));
        let back: Vec<Op> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ops);
        let m = Model::replay(&ops).unwrap();
        assert_eq!(m.hash(), Model::replay(&back).unwrap().hash());
        assert_eq!(
            m.stats().regions,
            2,
            "one half of the split square and the triangle"
        );
    }

    /// Lane 53 (#66): the retry at the minimum relabels only the failures the minimum can
    /// cause; every other failure keeps its own message (and a kernel bug of another kind
    /// stays a kernel bug, so the fuzzer still sees it).
    #[test]
    fn only_the_minimum_s_own_failures_are_relabelled() {
        use crate::boolean::BooleanError;
        use crate::validate::{TopologyReport, Violation};
        let topo = |v: Vec<Violation>| {
            KernelError::Topology(TopologyError {
                violations: v,
                report: TopologyReport::default(),
            })
        };
        let hit = crate::intersect::SelfHit::VertexOnFace { vertex: 0, face: 1 };
        for e in [
            topo(vec![Violation::ZeroLengthEdge(Default::default())]),
            topo(vec![Violation::SelfIntersectingSolid(hit)]),
            KernelError::Push(PushError::Boolean(BooleanError::Degenerate("x".into()))),
            KernelError::FaceEdit(FaceEditError::CrossesBoundary),
            KernelError::FaceEdit(FaceEditError::NotInside),
            KernelError::FaceEdit(FaceEditError::NotOnBoundary),
            KernelError::FaceEdit(FaceEditError::Dangling),
            KernelError::FaceEdit(FaceEditError::TooClose),
        ] {
            assert!(about_the_minimum(&e), "{e}");
        }
        for e in [
            topo(vec![]),
            topo(vec![Violation::DanglingKey("object is not watertight")]),
            topo(vec![
                Violation::ZeroLengthEdge(Default::default()),
                Violation::DanglingKey("object is not watertight"),
            ]),
            KernelError::Push(PushError::NoSuchFace(3)),
            KernelError::Push(PushError::OutOfBounds),
            KernelError::Offset(OffsetError::Vanishes),
            KernelError::FaceEdit(FaceEditError::NoSuchFace(3)),
            KernelError::Sketch(SketchError::Degenerate),
            KernelError::Limit(crate::limits::LimitError::SketchEdges(20_001)),
        ] {
            assert!(!about_the_minimum(&e), "{e}");
        }
    }

    /// Lane 53 (#65): opening a file does not hold its sketches to the edge limit, so a
    /// model can already hold a sketch past it. A stroke certain to break the limit in
    /// another sketch is then refused for the first sketch the check comes to, as it is with
    /// the stroke drawn (and as every edit to such a model is).
    #[test]
    fn a_file_already_over_the_sketch_limit() {
        use crate::limits::{LimitError, MAX_SKETCH_EDGES};
        let zig = |n: usize, y: f64| -> Vec<DVec2> {
            (0..n)
                .map(|k| DVec2::new(0.001 * k as f64, y + 0.0005 * (k % 2) as f64))
                .collect()
        };
        let mut m = Model::with_ground_sketch();
        let b = match m
            .apply(&Op::NewSketch {
                origin: [0.0, 0.0, 1.0],
                normal: [0.0, 0.0, 1.0],
            })
            .unwrap()
        {
            OpOutcome::SketchCreated { sketch } => sketch,
            o => panic!("{o:?}"),
        };
        let a = m.sketch_order[0];
        m.sketches[a]
            .insert_polylines(&[(zig(MAX_SKETCH_EDGES + 2, 0.0), false)])
            .unwrap();
        assert_eq!(m.sketches[a].edge_count(), MAX_SKETCH_EDGES + 1);
        let opened = Model::from_snapshot(&m.snapshot()).expect("a file may hold it");
        let want = Err(KernelError::Limit(LimitError::SketchEdges(
            MAX_SKETCH_EDGES + 1,
        )));
        let pts: Vec<[f64; 2]> = zig(MAX_SKETCH_EDGES + 5, 5.0)
            .iter()
            .map(|p| [p.x, p.y])
            .collect();
        let stroke = Op::AddPolyline {
            sketch: b,
            points: pts,
            closed: false,
        };
        let guide = Op::AddGuide {
            point: [0.0, 0.0, 0.0],
            dir: None,
        };
        // Every edit to this model is refused, for A; the stroke into B too, counted or drawn.
        assert_eq!(opened.clone().apply(&guide).map(|_| ()), want);
        assert_eq!(opened.clone().apply(&stroke).map(|_| ()), want);
        assert_eq!(
            opened
                .clone()
                .apply(&Op::Batch {
                    ops: vec![stroke, guide]
                })
                .map(|_| ()),
            want
        );
    }

    /// Lane 53 (#65): a zigzag of `n` points in the sketch's frame, every segment clear.
    fn clear_zigzag(n: usize, y: f64) -> Vec<[f64; 2]> {
        (0..n)
            .map(|k| [0.001 * k as f64, y + 0.0005 * (k % 2) as f64])
            .collect()
    }

    /// Lane 53 (#65): a stroke that makes exactly MAX_SKETCH_EDGES edges is drawn and
    /// accepted (not left out and done again), and one more edge is refused with its count.
    #[test]
    fn a_stroke_of_exactly_the_limit_is_drawn() {
        use crate::limits::{LimitError, MAX_SKETCH_EDGES};
        let redone = || REDONE.with(|r| r.get());
        let m = Model::with_ground_sketch();
        let before = redone();
        let (next, _) = m
            .applied(&Op::AddPolyline {
                sketch: 1,
                points: clear_zigzag(MAX_SKETCH_EDGES + 1, 0.0),
                closed: false,
            })
            .unwrap();
        assert_eq!(
            next.sketches[next.sketch_order[0]].edge_count(),
            MAX_SKETCH_EDGES
        );
        assert_eq!(
            redone(),
            before,
            "drawn at once, not left out and done again"
        );
        let e = m
            .applied(&Op::AddPolyline {
                sketch: 1,
                points: clear_zigzag(MAX_SKETCH_EDGES + 2, 0.0),
                closed: false,
            })
            .map(|_| ())
            .unwrap_err();
        assert_eq!(
            e,
            KernelError::Limit(LimitError::SketchEdges(MAX_SKETCH_EDGES + 1))
        );
        assert_eq!(redone(), before);
    }

    /// Lane 53 (#65): the path that does an edit again with a left-out stroke drawn cannot be
    /// reached as the code stands (a stroke is left out only when its count is over the
    /// limit). Forced here (every certain stroke left out): the edit is done again, once,
    /// and comes out as the edit with the stroke drawn, model and outcome.
    #[test]
    fn the_redo_with_the_stroke_drawn() {
        let mut m = Model::with_ground_sketch();
        m.apply(&Op::AddRectangle {
            sketch: 1,
            x: -1.0,
            y: -1.0,
            w: 0.4,
            h: 0.1,
        })
        .unwrap();
        let op = Op::AddPolyline {
            sketch: 1,
            points: clear_zigzag(300, 0.0),
            closed: false,
        };
        let (want, want_out) = m.applied(&op).unwrap();
        let before = REDONE.with(|r| r.get());
        FORCE_LEAVE_OUT.with(|f| f.set(true));
        let got = m.applied(&op);
        FORCE_LEAVE_OUT.with(|f| f.set(false));
        let (got, got_out) = got.unwrap();
        assert_eq!(REDONE.with(|r| r.get()), before + 1);
        assert_eq!(got.hash(), want.hash());
        assert_eq!(got_out, want_out);
    }
}
