//! Taylor-jet transition and rational-surface composition.

use super::*;

pub(super) fn parameter_jets(
    frame: BoundaryFrame,
    seam: TaylorJet<Dual>,
    inward: TaylorJet<Dual>,
) -> (TaylorJet<Dual>, TaylorJet<Dual>) {
    let along = TaylorJet::constant(seam.order(), Dual::constant(frame.along_domain().start()))
        + seam.scaled_f64(frame.along_parameter_span());
    let cross_start = if frame.inward_parameter_sign().is_sign_positive() {
        frame.cross_domain().start()
    } else {
        frame.cross_domain().end()
    };
    let cross = TaylorJet::constant(inward.order(), Dual::constant(cross_start))
        + inward.scaled_f64(frame.inward_parameter_scale());
    match frame.cross_axis() {
        SurfaceAxis::U => (cross, along),
        SurfaceAxis::V => (along, cross),
    }
}

pub(super) fn aligned_seam_jet(
    seam: TaylorJet<Dual>,
    alignment: BoundaryAlignment,
) -> TaylorJet<Dual> {
    match alignment {
        BoundaryAlignment::Aligned => seam,
        BoundaryAlignment::Reversed => {
            TaylorJet::constant(seam.order(), Dual::constant(1.0)) - seam
        }
    }
}

pub(super) fn bernstein_field(
    coefficients: &[Dual],
    parameter: TaylorJet<Dual>,
) -> TaylorJet<Dual> {
    let order = parameter.order();
    let one_minus = TaylorJet::constant(order, Dual::constant(1.0)) - parameter.clone();
    let mut level = coefficients
        .iter()
        .cloned()
        .map(|coefficient| TaylorJet::constant(order, coefficient))
        .collect::<Vec<_>>();
    (1..coefficients.len()).for_each(|remaining| {
        level = (0..coefficients.len() - remaining)
            .map(|index| {
                level[index].clone() * one_minus.clone()
                    + level[index + 1].clone() * parameter.clone()
            })
            .collect();
    });
    level
        .into_iter()
        .next()
        .unwrap_or_else(|| TaylorJet::zero(order))
}

pub(super) fn monotone_seam_map(
    free_log_increments: &[Dual],
    parameter: TaylorJet<Dual>,
) -> TaylorJet<Dual> {
    let mut increments = free_log_increments
        .iter()
        .cloned()
        .map(Dual::exp)
        .collect::<Vec<_>>();
    increments.push(Dual::constant(1.0));
    let total = increments
        .iter()
        .cloned()
        .fold(Dual::constant(0.0), |sum, increment| sum + increment);
    let controls = std::iter::once(Dual::constant(0.0))
        .chain(
            increments
                .into_iter()
                .scan(Dual::constant(0.0), |cumulative, increment| {
                    *cumulative = cumulative.clone() + increment / total.clone();
                    Some(cumulative.clone())
                }),
        )
        .collect::<Vec<_>>();
    bernstein_field(&controls, parameter)
}

pub(super) fn compose_surface(
    surface: &NurbsSurface<Vector4>,
    variables: Option<ControlVariables<'_>>,
    u: &TaylorJet<Dual>,
    v: &TaylorJet<Dual>,
) -> [TaylorJet<Dual>; 3] {
    let order = u.order();
    // SAFETY: Every `TaylorJet` contains the active constant coefficient.
    let u_base = u
        .coefficient(0, 0)
        .expect("the constant coefficient is active")
        .value();
    // SAFETY: Every `TaylorJet` contains the active constant coefficient.
    let v_base = v
        .coefficient(0, 0)
        .expect("the constant coefficient is active")
        .value();
    let u_basis = basis_jets(
        surface.knot_vector_u(),
        surface.udegree(),
        surface.control_points().len(),
        u_base,
        u,
    );
    let v_basis = basis_jets(
        surface.knot_vector_v(),
        surface.vdegree(),
        surface.control_points()[0].len(),
        v_base,
        v,
    );
    let denominator = u_basis
        .iter()
        .flat_map(|(row, basis_u)| {
            let v_basis = &v_basis;
            v_basis.iter().map(move |(column, basis_v)| {
                let point = surface.control_point(row, column);
                basis_u.clone()
                    * basis_v.clone()
                    * TaylorJet::constant(order, Dual::constant(point.w))
            })
        })
        .fold(TaylorJet::zero(order), |sum, value| sum + value);
    std::array::from_fn(|coordinate| {
        let numerator = u_basis
            .iter()
            .flat_map(|(row, basis_u)| {
                let v_basis = &v_basis;
                v_basis.iter().map(move |(column, basis_v)| {
                    let point = surface.control_point(row, column);
                    let physical = match variables {
                        Some((offsets, values)) => offsets[row][column]
                            .map(|offset| values[offset + coordinate].clone())
                            .unwrap_or_else(|| {
                                Dual::constant(physical_coordinate(point, coordinate))
                            }),
                        None => Dual::constant(physical_coordinate(point, coordinate)),
                    };
                    basis_u.clone()
                        * basis_v.clone()
                        * TaylorJet::constant(order, physical * Dual::constant(point.w))
                })
            })
            .fold(TaylorJet::zero(order), |sum, value| sum + value);
        numerator / denominator.clone()
    })
}

fn basis_jets(
    knots: &KnotVector,
    degree: usize,
    control_count: usize,
    base: f64,
    parameter: &TaylorJet<Dual>,
) -> BasisJets {
    let order = parameter.order();
    let delta = parameter.clone() - TaylorJet::constant(order, Dual::constant(base));
    let windows = (0..=order)
        .map(|derivative| knots.bspline_basis_functions(degree, derivative, base))
        .collect::<Vec<_>>();
    let start = windows
        .iter()
        .map(BasisWindow::start_index)
        .min()
        .unwrap_or(0)
        .min(control_count);
    let end = windows
        .iter()
        .map(|window| window.start_index() + window.len())
        .max()
        .unwrap_or(start)
        .min(control_count);
    let values = (start..end)
        .map(|control| {
            (0..=order).fold(TaylorJet::zero(order), |sum, derivative| {
                sum + delta
                    .powi(derivative)
                    .scaled_f64(basis_value(&windows[derivative], control) / factorial(derivative))
            })
        })
        .collect();
    BasisJets { start, values }
}

struct BasisJets {
    start: usize,
    values: Vec<TaylorJet<Dual>>,
}

impl BasisJets {
    fn iter(&self) -> impl Iterator<Item = (usize, &TaylorJet<Dual>)> {
        self.values
            .iter()
            .enumerate()
            .map(|(offset, value)| (self.start + offset, value))
    }
}

fn basis_value(window: &BasisWindow, index: usize) -> f64 {
    index
        .checked_sub(window.start_index())
        .and_then(|offset| window.values().get(offset))
        .copied()
        .unwrap_or(0.0)
}

pub(super) fn physical_control_scalar(
    surface: &NurbsSurface<Vector4>,
    offsets: &[Vec<Option<usize>>],
    variables: &[Dual],
    (row, column): (usize, usize),
    coordinate: usize,
) -> Dual {
    offsets[row][column]
        .map(|offset| variables[offset + coordinate].clone())
        .unwrap_or_else(|| {
            Dual::constant(physical_coordinate(
                surface.control_point(row, column),
                coordinate,
            ))
        })
}

fn physical_coordinate(point: &Vector4, coordinate: usize) -> f64 { point[coordinate] / point.w }
