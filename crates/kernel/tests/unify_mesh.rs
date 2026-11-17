//! sketchcraft E-40 (JC, 17 nov. 2026, 21 h 21 : « il faudrait une fonction unify import »):
//! an imported building mesh that is not a solid, as GeoHVAC's .dae are (roof pieces a couple
//! of centimetres apart, a step between a high and a low part, walls not sewn, no floor),
//! rebuilt as one solid from its roof.

use sawhorse_kernel::meshbuild::ImportItem;
use sawhorse_kernel::model::{Model, Op, OpOutcome};

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
