//! sketchcraft E-40 (JC, 17 nov. 2026, 21 h 21 : « il faudrait une fonction unify import »):
//! an imported building mesh that is not a solid, as GeoHVAC's .dae are (roof pieces a couple
//! of centimetres apart, a step between a high and a low part, walls not sewn, no floor),
//! rebuilt as one solid from its roof.

use sawhorse_kernel::meshbuild::ImportItem;
use sawhorse_kernel::model::{DrawTarget, Model, Op, OpOutcome};

fn item(positions: Vec<[f64; 3]>, tris: &[[u32; 3]]) -> ImportItem {
    ImportItem {
        name: "Bâtiment".into(),
        group: None,
        tag: None,
        material: None,
        positions,
        faces: tris.iter().map(|t| vec![t.to_vec()]).collect(),
        lines: vec![],
        merge_coplanar: false,
        face_labels: vec![],
        face_materials: vec![],
        face_uvs: vec![],
        edge_flags: vec![],
        curves: vec![],
    }
}

fn imported(it: ImportItem) -> Model {
    let mut m = Model::with_ground_sketch();
    m.apply(&Op::Import {
        items: vec![it],
        strokes: vec![],
        materials: vec![],
        scenes: vec![],
        hidden_tags: vec![],
        definitions: vec![],
        nodes: vec![],
    })
    .unwrap();
    assert_eq!(m.meshes().len(), 1, "kept as a mesh (not a solid)");
    m
}

/// A high part (x 0..4, roof 6 m, in two pieces 2 cm apart at two corners) beside a low
/// part (x 4..8, roof 3.3 m), one stray wall, no floor.
fn geohvac_like() -> ImportItem {
    let p = vec![
        // high roof, piece 1
        [0.0, 0.0, 6.0],
        [4.0, 0.0, 6.0],
        [4.0, 4.0, 6.0],
        // high roof, piece 2: its (0,0) and (4,4) corners 2 cm higher
        [0.0, 0.0, 6.02],
        [4.0, 4.0, 6.02],
        [0.0, 4.0, 6.0],
        // low roof
        [4.0, 0.0, 3.3],
        [8.0, 0.0, 3.3],
        [8.0, 4.0, 3.3],
        [4.0, 4.0, 3.3],
        // a wall that does not meet the roof (8 mm short) and the ground
        [0.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [4.0, 0.0, 5.992],
        [0.0, 0.0, 5.992],
    ];
    item(
        p,
        &[
            [0, 1, 2],
            [3, 4, 5],
            [6, 7, 8],
            [6, 8, 9],
            [10, 11, 12],
            [10, 12, 13],
        ],
    )
}

#[test]
fn a_geohvac_like_building_becomes_one_solid() {
    let mut m = imported(geohvac_like());
    let mesh = m.meshes()[0].0;
    let out = m.apply(&Op::UnifyMesh { mesh }).unwrap();
    let OpOutcome::MeshUnified { notes, .. } = out else {
        panic!("{out:?}")
    };
    assert!(notes[0].contains("1 step wall"), "{notes:?}");
    assert!(m.meshes().is_empty(), "the mesh goes");
    assert_eq!(m.objects().len(), 1);
    assert!(m.stats().watertight);
    // 4 x 4 x 6 + 4 x 4 x 3.3, the floor at 0.
    assert!(
        (m.stats().volume_m3 - (96.0 + 52.8)).abs() < 1e-6,
        "{}",
        m.stats().volume_m3
    );
}

#[test]
fn a_mesh_without_a_roof_is_refused_and_kept() {
    let mut m = imported(item(
        vec![
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [4.0, 0.0, 3.0],
            [0.0, 0.0, 3.0],
        ],
        &[[0, 1, 2], [0, 2, 3]],
    ));
    let mesh = m.meshes()[0].0;
    let err = m.apply(&Op::UnifyMesh { mesh }).unwrap_err();
    assert!(err.to_string().contains("no roof faces"), "{err}");
    assert_eq!(m.meshes().len(), 1);
}

/// A 4 x 4 m flat-roofed block whose roof is two triangles, one corner 2 cm high: unified,
/// its two roof triangles are merged into one flat pan by themselves (sketchcraft E-42).
#[test]
fn unify_merges_nearly_coplanar_roof_triangles_into_one_pan() {
    let it = item(
        vec![
            [0.0, 0.0, 3.0],
            [4.0, 0.0, 3.0],
            [4.0, 4.0, 3.02],
            [0.0, 4.0, 3.0],
            [0.0, 0.0, 0.0],
        ],
        &[[0, 1, 2], [0, 2, 3]],
    );
    let mut m = imported(it);
    let mesh = m.meshes()[0].0;
    let out = m.apply(&Op::UnifyMesh { mesh }).unwrap();
    let OpOutcome::MeshUnified { notes, .. } = out else {
        panic!("{out:?}")
    };
    assert!(
        notes
            .iter()
            .any(|n| n.contains("merged into their neighbours")),
        "{notes:?}"
    );
    assert_eq!(m.stats().faces, 6, "a roof, four walls, a floor");
    assert!(m.stats().watertight);
    let v = m.stats().volume_m3;
    assert!((v - 16.0 * 3.005).abs() < 0.01, "{v}");
}

/// A 1 m box whose top a line drawn across has cut in two.
fn box_with_a_cut_top() -> Model {
    let mut m = Model::with_ground_sketch();
    m.apply(&Op::AddRectangle {
        sketch: 1,
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    })
    .unwrap();
    m.apply(&Op::Extrude {
        sketch: 1,
        region: 1,
        distance: 1.0,
    })
    .unwrap();
    m.apply(&Op::Draw {
        target: DrawTarget::Face { object: 1, face: 2 },
        points: vec![[0.0, 0.5, 1.0], [1.0, 0.5, 1.0]],
        closed: false,
        curve: None,
    })
    .unwrap();
    assert_eq!(m.stats().faces, 7);
    m
}

fn top_faces(m: &Model) -> Vec<u64> {
    let (_, o) = m.objects()[0];
    let p = o.brep().to_polys();
    (0..p.faces.len())
        .filter(|&i| p.faces[i].outer.iter().all(|&v| p.points[v].z > 0.99))
        .map(|i| p.faces[i].id.unwrap())
        .collect()
}

#[test]
fn two_faces_in_one_plane_merge_back_into_one() {
    // sketchcraft E-41.
    let mut m = box_with_a_cut_top();
    let top = top_faces(&m);
    assert_eq!(top.len(), 2);
    m.apply(&Op::MergeFaces {
        object: 1,
        faces: top,
    })
    .unwrap();
    assert_eq!(m.stats().faces, 6);
    assert!(m.stats().watertight);
    assert!((m.stats().volume_m3 - 1.0).abs() < 1e-12);
}

#[test]
fn auto_merge_finds_them_by_itself_and_leaves_the_rest() {
    // sketchcraft E-42: the two halves of the top merge; the sides, at right angles, do not.
    let mut m = box_with_a_cut_top();
    let out = m.apply(&Op::AutoMergeFaces { object: 1 }).unwrap();
    assert_eq!(
        out,
        OpOutcome::AutoMerged {
            object: 1,
            merged: 1
        }
    );
    assert_eq!(m.stats().faces, 6);
    assert!((m.stats().volume_m3 - 1.0).abs() < 1e-12);
    // Nothing more to merge: nothing changes.
    let h = m.hash();
    assert_eq!(
        m.apply(&Op::AutoMergeFaces { object: 1 }).unwrap(),
        OpOutcome::AutoMerged {
            object: 1,
            merged: 0
        }
    );
    assert_eq!(m.hash(), h);
}

#[test]
fn faces_far_out_of_one_plane_are_not_merged() {
    // A box: its top and a side are at right angles, nothing changes.
    let mut m = Model::with_ground_sketch();
    m.apply(&Op::AddRectangle {
        sketch: 1,
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    })
    .unwrap();
    m.apply(&Op::Extrude {
        sketch: 1,
        region: 1,
        distance: 1.0,
    })
    .unwrap();
    let h = m.hash();
    let err = m
        .apply(&Op::MergeFaces {
            object: 1,
            faces: vec![1, 2, 3],
        })
        .unwrap_err();
    assert!(err.to_string().contains("out of one plane"), "{err}");
    assert_eq!(m.hash(), h);
}

#[test]
fn roof_pieces_a_little_over_3_cm_apart_are_sewn_too() {
    // GeoHVAC's building A2CDD7E9 (18 nov. 2026): two roof pieces whose shared corner is
    // 3.0006 cm apart were left unsewn at a 3 cm gap: "do not close a volume".
    let it = item(
        vec![
            [0.0, 0.0, 3.0],
            [4.0, 0.0, 3.0],
            [4.0, 4.0, 3.0],
            // its corner a hair over 3 cm off, across: the two pieces cross in plan
            [0.0, -0.0300001, 3.0006],
            [4.0, 4.0, 3.0],
            [0.0, 4.0, 3.0],
            [0.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
        ],
        &[[0, 1, 2], [3, 4, 5], [6, 7, 1], [6, 1, 0]],
    );
    let mut m = imported(it);
    let mesh = m.meshes()[0].0;
    m.apply(&Op::UnifyMesh { mesh }).unwrap();
    assert!(m.stats().watertight);
    assert!(
        (m.stats().volume_m3 - 48.0).abs() < 0.05,
        "{}",
        m.stats().volume_m3
    );
}
