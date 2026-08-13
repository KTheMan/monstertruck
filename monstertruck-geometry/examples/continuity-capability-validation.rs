use anyhow::{Result, ensure};
use monstertruck_geometry::base::Vector4;
use monstertruck_geometry::nurbs::continuity::{
    BoundarySide, ContinuityOrder, UnsupportedContinuityCapability, capability_for_bspline,
    capability_for_nurbs,
};
use monstertruck_geometry::nurbs::{BsplineSurface, KnotVector, NurbsSurface};

const SIDES: [BoundarySide; 4] = [
    BoundarySide::MinU,
    BoundarySide::MaxU,
    BoundarySide::MinV,
    BoundarySide::MaxV,
];

fn control_net(weight: f64) -> Vec<Vec<Vector4>> {
    (0..6)
        .map(|u| {
            (0..6)
                .map(|v| Vector4::new(u as f64 * weight, v as f64 * weight, 0.0, weight))
                .collect()
        })
        .collect()
}

fn surface(knots_u: KnotVector, knots_v: KnotVector, weight: f64) -> BsplineSurface<Vector4> {
    BsplineSurface::new_unchecked((knots_u, knots_v), control_net(weight))
}

fn main() -> Result<()> {
    let quintic = KnotVector::bezier_knot(5);
    let valid = surface(quintic.clone(), quintic.clone(), 1.0);
    SIDES.into_iter().try_for_each(|side| {
        let capability = capability_for_bspline(&valid, side, ContinuityOrder::G4);
        ensure!(capability.unsupported_reason().is_none());
        ensure!(
            capability.maximum_supported_order() == Some(ContinuityOrder::G4),
            "{side:?} did not report concrete G4 capability"
        );
        Ok::<_, anyhow::Error>(())
    })?;

    let invalid = KnotVector::from(vec![0.0, 0.0, 1.0, 0.5]);
    [
        surface(invalid.clone(), quintic.clone(), 1.0),
        surface(quintic.clone(), invalid, 1.0),
    ]
    .into_iter()
    .try_for_each(|surface| {
        SIDES.into_iter().try_for_each(|side| {
            let capability = capability_for_bspline(&surface, side, ContinuityOrder::G3);
            ensure!(
                capability.unsupported_reason()
                    == Some(UnsupportedContinuityCapability::InvalidKnotVector),
                "{side:?} did not preserve the invalid-knot reason"
            );
            Ok::<_, anyhow::Error>(())
        })
    })?;

    [
        (f64::NAN, UnsupportedContinuityCapability::NonFiniteWeight),
        (0.0, UnsupportedContinuityCapability::NonPositiveWeight),
    ]
    .into_iter()
    .try_for_each(|(weight, expected)| {
        let malformed = NurbsSurface::new(surface(
            KnotVector::from(vec![0.0, 1.0]),
            KnotVector::from(vec![0.0, 1.0]),
            weight,
        ));
        SIDES.into_iter().try_for_each(|side| {
            let capability = capability_for_nurbs(&malformed, side, ContinuityOrder::G3);
            ensure!(
                capability.unsupported_reason() == Some(expected),
                "{side:?} allowed malformed layout to hide {expected:?}"
            );
            ensure!(capability.maximum_supported_order().is_none());
            Ok::<_, anyhow::Error>(())
        })
    })?;

    println!("validated concrete capability diagnostics for every side through G4");
    Ok(())
}
