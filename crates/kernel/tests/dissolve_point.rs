//! sketchcraft E-50 (JC, 19 nov. 2026, 12 h 55 : « en mode point on voit bien les anomalies ,
//! des points inutile, un autre soucis " il n'est pas possible de supprimer un point entre
//! deux segments »): a point between two edges nearly in line can be removed, from a solid
//! or a sketch, one at a time or all at once.

use sawhorse_kernel::meshbuild::ImportItem;
use sawhorse_kernel::model::{Model, Op, OpOutcome};

/// A 1 m cube whose bottom front edge has a point at its middle and, when `two`, the top
/// back edge too.
fn cube_with_extra_points(two: bool) -> Model {
    let mut p = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 0.0, 1.0],
        [1.0, 1.0, 1.0],
        [0.0, 1.0, 1.0],
        [0.5, 0.0, 0.0], // 8: on the bottom front edge
    ];
    let top: Vec<u32> = if two {
        p.push([0.5, 1.0, 1.0]); // 9: on the top back edge
        vec![4, 5, 6, 9, 7]
    } else {
        vec![4, 5, 6, 7]
    };
    let back: Vec<u32> = if two {
        vec![3, 2, 6, 9, 7]
    } else {
        vec![3, 2, 6, 7]
    };
    // The extra point is on both the bottom and the front face's rings.
    let faces: Vec<Vec<u32>> = vec![
        vec![0, 3, 2, 1, 8], // bottom (z = 0), seen from below
        vec![0, 8, 1, 5, 4], // front (y = 0)
        vec![1, 2, 6, 5],    // right
        back,                // back
        vec![3, 0, 4, 7],    // left
        top,                 // top
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
    assert_eq!(m.stats().objects, 1, "imported as a solid");
    m
}

fn points(m: &Model) -> usize {
    m.objects()[0].1.brep().vertex_count()
}

#[test]
fn a_point_between_two_edges_in_line_goes() {
    let mut m = cube_with_extra_points(false);
    assert_eq!(points(&m), 9);
    let out = m
        .apply(&Op::DissolvePoint {
            object: 1,
            at: [0.5, 0.0, 0.0],
        })
        .unwrap();
    assert_eq!(
        out,
        OpOutcome::PointsCleaned {
            object: 1,
            removed: 1
        }
    );
    assert_eq!(points(&m), 8);
    assert!(m.stats().watertight);
    assert!((m.stats().volume_m3 - 1.0).abs() < 1e-12);
}

#[test]
fn a_corner_stays_and_says_why() {
    let mut m = cube_with_extra_points(false);
    let h = m.hash();
    let e = m
        .apply(&Op::DissolvePoint {
            object: 1,
            at: [0.0, 0.0, 0.0],
        })
        .unwrap_err()
        .to_string();
    assert!(e.contains("three edges or more"), "{e}");
    assert_eq!(m.hash(), h, "nothing changed");
}

#[test]
fn clean_points_removes_them_all_at_once() {
    let mut m = cube_with_extra_points(true);
    assert_eq!(points(&m), 10);
    let out = m.apply(&Op::CleanPoints { object: 1 }).unwrap();
    assert_eq!(
        out,
        OpOutcome::PointsCleaned {
            object: 1,
            removed: 2
        }
    );
    assert_eq!(points(&m), 8);
    assert!((m.stats().volume_m3 - 1.0).abs() < 1e-12);
    // Nothing left to clean: nothing changes.
    let h = m.hash();
    m.apply(&Op::CleanPoints { object: 1 }).unwrap();
    assert_eq!(m.hash(), h);
}

#[test]
fn a_sketch_point_between_two_segments_goes() {
    let mut m = Model::with_ground_sketch();
    m.apply(&Op::AddPolyline {
        sketch: 1,
        points: vec![[0.0, 0.0], [0.5, 0.01], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        closed: true,
    })
    .unwrap();
    let edges = |m: &Model| m.sketch(1).unwrap().edge_count();
    assert_eq!(edges(&m), 5);
    // 1 cm off the line: within 5 cm, it goes.
    m.apply(&Op::DissolveSketchPoint {
        sketch: 1,
        at: [0.5, 0.01],
    })
    .unwrap();
    assert_eq!(edges(&m), 4);
    assert_eq!(m.sketch(1).unwrap().regions().count(), 1);
    // A corner stays.
    let e = m
        .apply(&Op::DissolveSketchPoint {
            sketch: 1,
            at: [1.0, 1.0],
        })
        .unwrap_err()
        .to_string();
    assert!(e.contains("not in line"), "{e}");
    assert_eq!(edges(&m), 4);
}

#[test]
fn clean_sketch_points_removes_every_point_in_line() {
    let mut m = Model::with_ground_sketch();
    m.apply(&Op::AddPolyline {
        sketch: 1,
        points: vec![
            [0.0, 0.0],
            [0.3, 0.02],
            [0.6, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ],
        closed: true,
    })
    .unwrap();
    assert_eq!(m.sketch(1).unwrap().edge_count(), 6);
    let out = m.apply(&Op::CleanSketchPoints { sketch: 1 }).unwrap();
    assert_eq!(
        out,
        OpOutcome::SketchPointsCleaned {
            sketch: 1,
            removed: 2
        }
    );
    assert_eq!(m.sketch(1).unwrap().edge_count(), 4);
    assert_eq!(m.sketch(1).unwrap().regions().count(), 1);
}

#[test]
fn the_loose_end_of_a_line_takes_the_line_with_it() {
    // sketchcraft E-54: a square with a stub line sticking out of one side.
    let mut m = Model::with_ground_sketch();
    m.apply(&Op::AddRectangle {
        sketch: 1,
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    })
    .unwrap();
    m.apply(&Op::InsertSegment {
        sketch: 1,
        a: [1.0, 0.5],
        b: [1.3, 0.5],
    })
    .unwrap();
    let edges = |m: &Model| m.sketch(1).unwrap().edge_count();
    assert_eq!(edges(&m), 6, "the side split at the stub, and the stub");
    m.apply(&Op::DissolveSketchPoint {
        sketch: 1,
        at: [1.3, 0.5],
    })
    .unwrap();
    assert_eq!(edges(&m), 4, "the stub went, and the side healed");
    assert_eq!(m.sketch(1).unwrap().regions().count(), 1);
}
