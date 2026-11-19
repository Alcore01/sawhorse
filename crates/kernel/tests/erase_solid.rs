//! sketchcraft E-52 (JC, 19 nov. 2026, 14 h 03 : « je ne sais supprimer aucunes lignes ni
//! surface »): the Eraser and Delete on a solid's edge or face, as in SketchUp.

use sawhorse_kernel::meshbuild::ImportItem;
use sawhorse_kernel::model::{Model, Op, OpOutcome};

/// A 1 m cube whose top is two faces (split along x = 0.5), imported as one solid.
fn cube_split_top() -> Model {
    let p = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.5, 0.0, 1.0], // 8
        [0.5, 1.0, 1.0], // 9
    ];
    let faces: Vec<Vec<u32>> = vec![
        vec![0, 3, 2, 1],
        vec![0, 1, 5, 8, 4],
        vec![1, 2, 6, 5],
        vec![3, 7, 9, 6, 2],
        vec![3, 0, 4, 7],
        vec![4, 8, 9, 7],
        vec![8, 5, 6, 9],
    ];
    let it = ImportItem {
        name: "Cube".into(),
        group: None,
        tag: None,
        material: None,
        positions: p,
        faces: faces.into_iter().map(|r| vec![r]).collect(),
        lines: vec![],
        merge_coplanar: false,
        face_labels: vec![],
        face_materials: vec![],
        face_uvs: vec![],
        edge_flags: vec![],
        curves: vec![],
    };
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
    assert_eq!(m.stats().objects, 1);
    assert_eq!(m.stats().faces, 7);
    m
}

#[test]
fn an_edge_between_two_faces_in_one_plane_goes_and_the_solid_stays() {
    let mut m = cube_split_top();
    let out = m
        .apply(&Op::EraseSolidEdges {
            object: 1,
            edges: vec![[[0.5, 0.0, 1.0], [0.5, 1.0, 1.0]]],
        })
        .unwrap();
    assert!(matches!(out, OpOutcome::FacesMerged { .. }), "{out:?}");
    assert_eq!(m.stats().objects, 1);
    assert_eq!(m.stats().faces, 6);
    assert!((m.stats().volume_m3 - 1.0).abs() < 1e-12);
}

#[test]
fn a_corner_edge_opens_the_solid_and_its_two_faces_go() {
    let mut m = cube_split_top();
    let out = m
        .apply(&Op::EraseSolidEdges {
            object: 1,
            edges: vec![[[1.0, 0.0, 0.0], [1.0, 1.0, 0.0]]],
        })
        .unwrap();
    assert_eq!(
        out,
        OpOutcome::SolidOpened {
            face_count: 7,
            erased: 2
        }
    );
    assert_eq!(m.stats().objects, 0, "the solid became plain faces");
}

#[test]
fn a_face_opens_the_solid_and_only_that_face_goes() {
    let mut m = cube_split_top();
    let regions =
        |m: &Model| -> usize { m.sketches().iter().map(|(_, s)| s.regions().count()).sum() };
    let before = regions(&m);
    let front = m.objects()[0].1.brep().to_polys().faces[2].id.unwrap(); // x = 1 side
    let out = m
        .apply(&Op::EraseSolidFaces {
            object: 1,
            faces: vec![front],
        })
        .unwrap();
    assert_eq!(
        out,
        OpOutcome::SolidOpened {
            face_count: 7,
            erased: 1
        }
    );
    assert_eq!(m.stats().objects, 0);
    assert_eq!(regions(&m), before + 6, "the other six faces are there");
}

#[test]
fn two_faces_of_one_solid_go_together() {
    let mut m = cube_split_top();
    let p = m.objects()[0].1.brep().to_polys();
    let ids: Vec<u64> = [0, 2].iter().map(|&i| p.faces[i].id.unwrap()).collect();
    let out = m
        .apply(&Op::EraseSolidFaces {
            object: 1,
            faces: ids,
        })
        .unwrap();
    assert_eq!(
        out,
        OpOutcome::SolidOpened {
            face_count: 7,
            erased: 2
        }
    );
}

#[test]
fn several_edges_of_one_solid_go_in_one_step() {
    // The split of the top merges; then a bottom corner edge opens the solid.
    let mut m = cube_split_top();
    let out = m
        .apply(&Op::EraseSolidEdges {
            object: 1,
            edges: vec![
                [[0.5, 0.0, 1.0], [0.5, 1.0, 1.0]],
                [[1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            ],
        })
        .unwrap();
    assert_eq!(
        out,
        OpOutcome::SolidOpened {
            face_count: 6,
            erased: 2
        }
    );
}
