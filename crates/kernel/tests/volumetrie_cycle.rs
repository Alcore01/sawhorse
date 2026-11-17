//! sketchcraft consolidation (JC, 17 nov. 2026, 16 h 41 : « on va faire 1 et 2 »): the whole
//! cycle on a document, with undo, redo, save and reopen at each step: a box, a gable closed
//! against it (E-25), exploded into faces (E-28), its top erased, a roof drawn, the faces made
//! one solid again, a roof slab pushed up (E-30), the slab joined to the house (E-04); and the
//! read-only queries for loose faces (E-36 volume, E-37 connected faces).

use sawhorse_kernel::Document;
use sawhorse_kernel::model::{DrawTarget, Model, Op, OpOutcome};

fn ok(d: &mut Document, op: Op) -> OpOutcome {
    d.apply_owned(op.clone())
        .unwrap_or_else(|e| panic!("{op:?}: {e}"))
}

/// Saved and opened again: the same model.
fn reopen(d: &Document) {
    let json = serde_json::to_string(&d.model().snapshot()).unwrap();
    let back: sawhorse_kernel::snapshot::Snapshot = serde_json::from_str(&json).unwrap();
    let m = Model::from_snapshot(&back).unwrap();
    assert_eq!(
        m.hash(),
        d.model().hash(),
        "saved and reopened, the model is the same"
    );
}

/// Undo then redo: back to the same model.
fn undo_redo(d: &mut Document) {
    let h = d.model().hash();
    d.undo().unwrap();
    assert_ne!(d.model().hash(), h);
    d.redo().unwrap();
    assert_eq!(d.model().hash(), h, "undo then redo gives the same model");
}

fn step(d: &mut Document, op: Op) -> OpOutcome {
    let out = ok(d, op);
    undo_redo(d);
    reopen(d);
    out
}

/// Every live region: `[sketch, region]` and the height of its centroid.
fn faces(m: &Model) -> Vec<([u64; 2], f64)> {
    let mut out = Vec::new();
    for (sid, s) in m.sketches() {
        for (rid, r) in s.regions() {
            let (o, _) = s.region_rings(r);
            let c = o.iter().fold(glam::DVec2::ZERO, |a, &p| a + p) / o.len() as f64;
            out.push((
                [sid, sawhorse_kernel::ids::to_public(rid)],
                s.plane().to_world(c).z,
            ));
        }
    }
    out
}

fn closed(points: Vec<[f64; 3]>, origin: [f64; 3], normal: [f64; 3]) -> Op {
    Op::Draw {
        target: DrawTarget::Plane { origin, normal },
        points,
        closed: true,
        curve: None,
    }
}

#[test]
fn the_massing_cycle_keeps_through_undo_redo_save_and_reopen() {
    let mut d = Document::with_ground_sketch();
    step(
        &mut d,
        Op::AddRectangle {
            sketch: 1,
            x: 0.0,
            y: 0.0,
            w: 2.0,
            h: 1.0,
        },
    );
    step(
        &mut d,
        Op::Extrude {
            sketch: 1,
            region: 1,
            distance: 1.0,
        },
    );
    assert!((d.model().stats().volume_m3 - 2.0).abs() < 1e-9);

    // The box into faces; the top goes.
    assert_eq!(
        step(&mut d, Op::ExplodeSolid { object: 1 }),
        OpOutcome::SolidExploded { face_count: 6 }
    );
    let top = faces(d.model())
        .into_iter()
        .find(|f| (f.1 - 1.0).abs() < 1e-9)
        .unwrap()
        .0;
    step(
        &mut d,
        Op::EraseRegion {
            sketch: top[0],
            region: top[1],
        },
    );
    assert_eq!(faces(d.model()).len(), 5);

    // A gable roof: two gables (on the end walls' planes) and two slopes.
    let s = 0.5f64.hypot(0.5);
    step(
        &mut d,
        closed(
            vec![[0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.5, 1.5]],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ),
    );
    step(
        &mut d,
        closed(
            vec![[2.0, 0.0, 1.0], [2.0, 1.0, 1.0], [2.0, 0.5, 1.5]],
            [2.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        ),
    );
    step(
        &mut d,
        closed(
            vec![
                [0.0, 0.0, 1.0],
                [2.0, 0.0, 1.0],
                [2.0, 0.5, 1.5],
                [0.0, 0.5, 1.5],
            ],
            [0.0, 0.0, 1.0],
            [0.0, -0.5 / s, 0.5 / s],
        ),
    );
    step(
        &mut d,
        closed(
            vec![
                [0.0, 1.0, 1.0],
                [2.0, 1.0, 1.0],
                [2.0, 0.5, 1.5],
                [0.0, 0.5, 1.5],
            ],
            [0.0, 1.0, 1.0],
            [0.0, 0.5 / s, 0.5 / s],
        ),
    );
    let all: Vec<[u64; 2]> = faces(d.model()).into_iter().map(|f| f.0).collect();
    assert_eq!(all.len(), 9);

    // E-36: the loose faces already tell their volume; E-37: one face reaches them all.
    assert!((d.model().closed_faces_volume(&all).unwrap() - 2.5).abs() < 1e-9);
    assert_eq!(
        d.model().closed_faces_volume(&all[..8]),
        None,
        "an open shell has no volume"
    );
    let mut reach = d.model().connected_faces(all[0][0], all[0][1]);
    reach.sort();
    let mut want = all.clone();
    want.sort();
    assert_eq!(reach, want);

    // A roof slab straight up from both slopes, before the house is closed (the slopes stay).
    let slopes: Vec<[u64; 2]> = faces(d.model())
        .into_iter()
        .filter(|f| f.1 > 1.2 && f.1 < 1.3)
        .map(|f| f.0)
        .collect();
    assert_eq!(slopes.len(), 2);
    step(
        &mut d,
        Op::ExtrudeFaces {
            faces: slopes,
            vector: [0.0, 0.0, 0.3],
        },
    );
    assert!((d.model().stats().volume_m3 - 0.6).abs() < 1e-9);

    // The house closed into a solid, then joined with the slab.
    step(&mut d, Op::SolidFromFaces { faces: all });
    assert_eq!(d.model().objects().len(), 2);
    assert!((d.model().stats().volume_m3 - 3.1).abs() < 1e-9);
    let ids: Vec<u64> = d.model().objects().iter().map(|o| o.0).collect();
    step(&mut d, Op::UnionSolids { objects: ids });
    assert_eq!(d.model().objects().len(), 1);
    assert!(d.model().stats().watertight);
    assert!(
        (d.model().stats().volume_m3 - 3.1).abs() < 1e-9,
        "{}",
        d.model().stats().volume_m3
    );
}

#[test]
fn solids_apart_are_not_joined_and_nothing_changes() {
    let mut d = Document::with_ground_sketch();
    ok(
        &mut d,
        Op::AddRectangle {
            sketch: 1,
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
    );
    ok(
        &mut d,
        Op::Extrude {
            sketch: 1,
            region: 1,
            distance: 1.0,
        },
    );
    ok(
        &mut d,
        Op::AddRectangle {
            sketch: 1,
            x: 3.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        },
    );
    let r = faces(d.model())
        .into_iter()
        .find(|f| f.1.abs() < 1e-9)
        .unwrap()
        .0;
    ok(
        &mut d,
        Op::Extrude {
            sketch: r[0],
            region: r[1],
            distance: 1.0,
        },
    );
    let h = d.model().hash();
    let ids: Vec<u64> = d.model().objects().iter().map(|o| o.0).collect();
    let err = d.apply_owned(Op::UnionSolids { objects: ids }).unwrap_err();
    assert!(
        err.to_string().contains("neither touch nor overlap"),
        "{err}"
    );
    assert_eq!(d.model().hash(), h);
}

// ---- review of 17 nov. 2026 (relecture à froid) -------------------------------------------

fn ground_rects(rects: &[(f64, f64, f64, f64)]) -> Document {
    let mut d = Document::with_ground_sketch();
    for &(x, y, w, h) in rects {
        ok(
            &mut d,
            Op::AddRectangle {
                sketch: 1,
                x,
                y,
                w,
                h,
            },
        );
    }
    d
}

#[test]
fn faces_apart_extrude_together_into_one_solid_each() {
    // Constat 2: the op said "touching or not", and refused faces apart.
    let mut d = ground_rects(&[(0.0, 0.0, 1.0, 1.0), (3.0, 0.0, 1.0, 1.0)]);
    let f: Vec<[u64; 2]> = faces(d.model()).into_iter().map(|f| f.0).collect();
    assert_eq!(f.len(), 2);
    ok(
        &mut d,
        Op::ExtrudeFaces {
            faces: f,
            vector: [0.0, 0.0, 1.0],
        },
    );
    assert_eq!(d.model().objects().len(), 2);
    assert!(d.model().stats().watertight);
    assert!((d.model().stats().volume_m3 - 2.0).abs() < 1e-9);
}

#[test]
fn small_volumes_are_volumes() {
    // Constat 4: a length tolerance was used as a volume: under about 1 cm³ was "no volume".
    let mut d = ground_rects(&[(0.0, 0.0, 0.009, 0.009)]);
    let f: Vec<[u64; 2]> = faces(d.model()).into_iter().map(|f| f.0).collect();
    ok(
        &mut d,
        Op::ExtrudeFaces {
            faces: f,
            vector: [0.0, 0.0, 0.005],
        },
    );
    assert!((d.model().stats().volume_m3 - 0.009 * 0.009 * 0.005).abs() < 1e-15);
    let object = d.model().objects()[0].0;
    ok(&mut d, Op::ExplodeSolid { object });
    let all: Vec<[u64; 2]> = faces(d.model()).into_iter().map(|f| f.0).collect();
    assert!((d.model().closed_faces_volume(&all).unwrap() - 0.009 * 0.009 * 0.005).abs() < 1e-15);
    ok(&mut d, Op::SolidFromFaces { faces: all });
    assert_eq!(d.model().objects().len(), 1);
}

#[test]
fn solids_touching_on_an_edge_only_are_refused_in_plain_words() {
    // Constat 11: the raw boolean error came through.
    let mut d = ground_rects(&[(0.0, 0.0, 1.0, 1.0), (1.0, 1.0, 1.0, 1.0)]);
    for f in faces(d.model()) {
        ok(
            &mut d,
            Op::Extrude {
                sketch: f.0[0],
                region: f.0[1],
                distance: 1.0,
            },
        );
    }
    let ids: Vec<u64> = d.model().objects().iter().map(|o| o.0).collect();
    assert_eq!(ids.len(), 2);
    let err = d.apply_owned(Op::UnionSolids { objects: ids }).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("cannot be joined"), "{msg}");
}
