//! sketchcraft E-28 (H-20261117-5): a solid exploded into plain faces, a face removed, and
//! a closed set of faces made a solid again (JC, 17 nov. 2026).

use sawhorse_kernel::model::{DrawTarget, Model, Op, OpOutcome};

fn boxed() -> Model {
    let mut m = Model::with_ground_sketch();
    m.apply(&Op::AddRectangle {
        sketch: 1,
        x: 0.0,
        y: 0.0,
        w: 2.0,
        h: 1.0,
    })
    .unwrap();
    m.apply(&Op::Extrude {
        sketch: 1,
        region: 1,
        distance: 1.0,
    })
    .unwrap();
    m
}

/// Every live region: `[sketch, region]` with its centroid height (to pick the top).
fn faces(m: &Model) -> Vec<([u64; 2], f64, f64)> {
    let mut out = Vec::new();
    for (sid, s) in m.sketches() {
        for (rid, r) in s.regions() {
            let (o, _) = s.region_rings(r);
            let c = o.iter().fold(glam::DVec2::ZERO, |a, &p| a + p) / o.len() as f64;
            let w = s.plane().to_world(c);
            out.push((
                [sid, sawhorse_kernel::ids::to_public(rid)],
                w.z,
                s.region_area(r),
            ));
        }
    }
    out
}

#[test]
fn a_box_explodes_into_six_faces() {
    let mut m = boxed();
    let out = m.apply(&Op::ExplodeSolid { object: 1 }).unwrap();
    assert_eq!(out, OpOutcome::SolidExploded { face_count: 6 });
    assert!(m.objects().is_empty(), "the solid is gone");
    let f = faces(&m);
    assert_eq!(f.len(), 6, "{f:?}");
    assert!(
        (f.iter().map(|x| x.2).sum::<f64>() - 10.0).abs() < 1e-9,
        "areas 2+2+2+2+1+1"
    );
}

#[test]
fn the_faces_make_the_solid_again() {
    let mut m = boxed();
    m.apply(&Op::ExplodeSolid { object: 1 }).unwrap();
    let all: Vec<[u64; 2]> = faces(&m).into_iter().map(|x| x.0).collect();
    let out = m.apply(&Op::SolidFromFaces { faces: all }).unwrap();
    assert!(matches!(out, OpOutcome::ObjectCreated { .. }), "{out:?}");
    assert!(m.stats().watertight);
    assert!((m.stats().volume_m3 - 2.0).abs() < 1e-9);
    assert!(faces(&m).is_empty(), "the faces are used up");
}

#[test]
fn an_open_set_of_faces_is_refused_and_nothing_changes() {
    let mut m = boxed();
    m.apply(&Op::ExplodeSolid { object: 1 }).unwrap();
    let f = faces(&m);
    let top = f.iter().find(|x| (x.1 - 1.0).abs() < 1e-9).unwrap().0;
    m.apply(&Op::EraseRegion {
        sketch: top[0],
        region: top[1],
    })
    .unwrap();
    let rest: Vec<[u64; 2]> = faces(&m).into_iter().map(|x| x.0).collect();
    assert_eq!(rest.len(), 5);
    let err = m.apply(&Op::SolidFromFaces { faces: rest }).unwrap_err();
    assert!(err.to_string().contains("do not close a volume"), "{err}");
    assert_eq!(faces(&m).len(), 5);
    assert!(m.objects().is_empty());
}

#[test]
fn a_box_with_its_top_replaced_by_a_gable_roof_is_a_solid() {
    // JC's case: the top removed, a roof drawn on, the whole made a volume again.
    let mut m = boxed();
    m.apply(&Op::ExplodeSolid { object: 1 }).unwrap();
    let top = faces(&m)
        .into_iter()
        .find(|x| (x.1 - 1.0).abs() < 1e-9)
        .unwrap()
        .0;
    m.apply(&Op::EraseRegion {
        sketch: top[0],
        region: top[1],
    })
    .unwrap();
    let draw = |m: &mut Model, origin: [f64; 3], normal: [f64; 3], pts: Vec<[f64; 3]>| {
        m.apply(&Op::Draw {
            target: DrawTarget::Plane { origin, normal },
            points: pts,
            closed: true,
            curve: None,
        })
        .unwrap();
    };
    let s = 0.5f64.hypot(0.5);
    draw(
        &mut m,
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        vec![[0.0, 0.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.5, 1.5]],
    );
    draw(
        &mut m,
        [2.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        vec![[2.0, 0.0, 1.0], [2.0, 1.0, 1.0], [2.0, 0.5, 1.5]],
    );
    draw(
        &mut m,
        [0.0, 0.0, 1.0],
        [0.0, -0.5 / s, 0.5 / s],
        vec![
            [0.0, 0.0, 1.0],
            [2.0, 0.0, 1.0],
            [2.0, 0.5, 1.5],
            [0.0, 0.5, 1.5],
        ],
    );
    draw(
        &mut m,
        [0.0, 1.0, 1.0],
        [0.0, 0.5 / s, 0.5 / s],
        vec![
            [0.0, 1.0, 1.0],
            [2.0, 1.0, 1.0],
            [2.0, 0.5, 1.5],
            [0.0, 0.5, 1.5],
        ],
    );
    let all: Vec<[u64; 2]> = faces(&m).into_iter().map(|x| x.0).collect();
    assert_eq!(all.len(), 9, "5 box faces, 2 gables, 2 slopes");
    m.apply(&Op::SolidFromFaces { faces: all }).unwrap();
    assert!(m.stats().watertight);
    assert!(
        (m.stats().volume_m3 - 2.5).abs() < 1e-9,
        "{}",
        m.stats().volume_m3
    );
}
