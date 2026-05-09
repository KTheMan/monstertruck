use super::*;
use mesh::{boundary_segment_parameter, spade_round};

#[derive(Clone, Copy, Debug, derive_more::Deref, derive_more::DerefMut)]
pub(super) struct SurfacePoint {
    pub(super) point: Point3,
    #[deref]
    #[deref_mut]
    pub(super) uv: Point2,
}

impl From<(Point2, Point3)> for SurfacePoint {
    fn from((uv, point): (Point2, Point3)) -> Self { Self { point, uv } }
}

#[derive(Debug, Default, Clone)]
pub(super) struct PolyBoundaryPiece(pub(super) Vec<SurfacePoint>);

impl PolyBoundaryPiece {
    fn from_surface_points(mut vec: Vec<SurfacePoint>) -> Option<Self> {
        vec = vec
            .into_iter()
            .fold(Vec::<SurfacePoint>::new(), |mut acc, point| {
                if acc.last().is_none_or(|last| !last.uv.near(&point.uv)) {
                    acc.push(point);
                }
                acc
            });
        if vec.is_empty() {
            None
        } else {
            if vec
                .first()
                .is_some_and(|first| vec.last().is_some_and(|last| !first.uv.near(&last.uv)))
            {
                vec.push(vec[0]);
            }
            Some(Self(vec))
        }
    }

    fn from_parameter_boundary<S: PreMeshableSurface>(
        surface: &S,
        boundary: Vec<Point2>,
    ) -> Option<Self> {
        let mut previous = None;
        let vec = boundary
            .into_iter()
            .map(|uv| Self::normalize_uv(surface, (uv.x, uv.y), previous))
            .collect::<Option<Vec<_>>>()?
            .into_iter()
            .map(|(u, v)| {
                previous = Some((u, v));
                SurfacePoint::from((Point2::new(u, v), surface.subs(u, v)))
            })
            .collect();
        Self::from_surface_points(vec)
    }

    pub(super) fn try_new_from_trimmed<'a, S, T, C>(
        surface: &S,
        wire: impl Iterator<Item = (bool, Option<&'a T>, &'a C)>,
        tolerance: f64,
    ) -> Option<Self>
    where
        S: PreMeshableSurface,
        T: ExactTrimBoundary2D + 'a,
        C: ParameterBoundary2D<S> + ExactParameterBoundary2D<S> + 'a,
        <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
    {
        wire.map(|(orientation, trim_curve, edge_curve)| {
            let mut boundary = trim_curve
                .map(|trim_curve| trim_curve.exact_trim_boundary_2d(tolerance))
                .or_else(|| {
                    edge_curve
                        .exact_parameter_boundary_2d(surface)
                        .map(|trim_curve| trim_curve.exact_trim_boundary_2d(tolerance))
                })
                .or_else(|| edge_curve.parameter_boundary_2d(surface, tolerance))
                .map(|boundary| simplify_parameter_boundary(surface, boundary, tolerance))?;
            if !orientation {
                boundary.reverse();
            }
            Some(boundary)
        })
        .collect::<Option<Vec<Vec<Point2>>>>()
        .and_then(|boundaries| {
            let concatenated =
                boundaries
                    .into_iter()
                    .fold(Vec::<Point2>::new(), |mut acc, mut boundary| {
                        if !acc.is_empty() && !boundary.is_empty() {
                            boundary.remove(0);
                        }
                        acc.extend(boundary);
                        acc
                    });
            Self::from_parameter_boundary(surface, concatenated)
        })
    }

    pub(super) fn try_new_from_aligned_trimmed<'a, S, T, C>(
        surface: &S,
        wire: impl Iterator<Item = (Option<&'a T>, &'a C, PolylineCurve)>,
        sp: &impl SP<S>,
        tolerance: f64,
    ) -> Option<Self>
    where
        S: PreMeshableSurface,
        T: ExactTrimBoundary2D + 'a,
        C: ParameterBoundary2D<S> + ExactParameterBoundary2D<S> + 'a,
        <C as ExactParameterBoundary2D<S>>::BoundaryCurve: ExactTrimBoundary2D,
    {
        wire.map(|(trim_curve, edge_curve, polyline)| {
            trim_curve
                .and_then(|trim_curve| {
                    let mut previous_t = None;
                    let mut previous_uv = None;
                    polyline
                        .iter()
                        .copied()
                        .map(|point| {
                            let (t, uv) = trim_curve.project_boundary_point(point, previous_t)?;
                            let (u, v) = Self::normalize_uv(surface, (uv.x, uv.y), previous_uv)?;
                            previous_t = Some(t);
                            previous_uv = Some((u, v));
                            Some(SurfacePoint::from((Point2::new(u, v), point)))
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .or_else(|| {
                    let mut previous = None;
                    polyline
                        .iter()
                        .copied()
                        .map(|point| {
                            let (u, v) = sp(surface, point, previous)
                                .and_then(|uv| Self::normalize_uv(surface, uv, previous))?;
                            previous = Some((u, v));
                            Some(SurfacePoint::from((Point2::new(u, v), point)))
                        })
                        .collect::<Option<Vec<_>>>()
                })
                .or_else(|| {
                    edge_curve
                        .parameter_boundary_2d(surface, tolerance)
                        .map(|boundary| simplify_parameter_boundary(surface, boundary, tolerance))
                        .and_then(|boundary| Self::from_parameter_boundary(surface, boundary))
                        .map(|piece| piece.0)
                })
        })
        .collect::<Option<Vec<Vec<SurfacePoint>>>>()
        .and_then(|pieces| {
            let concatenated =
                pieces
                    .into_iter()
                    .fold(Vec::<SurfacePoint>::new(), |mut acc, mut piece| {
                        if !acc.is_empty() && !piece.is_empty() {
                            piece.remove(0);
                        }
                        acc.extend(piece);
                        acc
                    });
            Self::from_surface_points(concatenated)
        })
    }

    pub(super) fn try_new_from_exact<'a, S: PreMeshableSurface, C: ParameterBoundary2D<S> + 'a>(
        surface: &S,
        wire: impl Iterator<Item = (bool, &'a C)>,
        tolerance: f64,
    ) -> Option<Self> {
        wire.map(|(orientation, curve)| {
            let mut boundary = curve.parameter_boundary_2d(surface, tolerance)?;
            if !orientation {
                boundary.reverse();
            }
            Some(boundary)
        })
        .collect::<Option<Vec<Vec<Point2>>>>()
        .and_then(|boundaries| {
            let concatenated =
                boundaries
                    .into_iter()
                    .fold(Vec::<Point2>::new(), |mut acc, mut boundary| {
                        if !acc.is_empty() && !boundary.is_empty() {
                            boundary.remove(0);
                        }
                        acc.extend(boundary);
                        acc
                    });
            Self::from_parameter_boundary(surface, concatenated)
        })
    }

    pub(super) fn try_new_from_aligned_exact<S: PreMeshableSurface>(
        surface: &S,
        wire: impl Iterator<Item = (Option<Vec<Point2>>, PolylineCurve)>,
        sp: &impl SP<S>,
    ) -> Option<Self> {
        let pieces = wire.collect::<Vec<_>>();
        if pieces.is_empty() {
            None
        } else {
            let piece_count = pieces.len();
            let start = pieces
                .iter()
                .position(|(boundary, _)| boundary.is_some())
                .unwrap_or(0);
            let mut previous = None::<SurfacePoint>;
            pieces
                .into_iter()
                .cycle()
                .skip(start)
                .take(piece_count)
                .map(|(boundary, polyline)| {
                    let boundary_piece = boundary
                        .as_ref()
                        .and_then(|boundary| resample_boundary(boundary, polyline.len()))
                        .and_then(|boundary| {
                            boundary
                                .into_iter()
                                .zip(polyline.iter().copied())
                                .map(|(uv, point)| {
                                    let (u, v) = Self::normalize_uv(
                                        surface,
                                        (uv.x, uv.y),
                                        previous.as_ref().map(|point| (point.x, point.y)),
                                    )?;
                                    let surface_point =
                                        SurfacePoint::from((Point2::new(u, v), point));
                                    previous = Some(surface_point);
                                    Some(surface_point)
                                })
                                .collect::<Option<Vec<_>>>()
                        });
                    let projected_piece = || {
                        let seed = previous.as_ref().and_then(|previous_point| {
                            polyline
                                .first()
                                .copied()
                                .filter(|point| point.near(&previous_point.point))
                                .map(|point| {
                                    SurfacePoint::from((
                                        Point2::new(previous_point.x, previous_point.y),
                                        point,
                                    ))
                                })
                        });
                        let prefix = seed.into_iter().collect::<Vec<_>>();
                        let suffix = polyline
                            .iter()
                            .copied()
                            .skip(prefix.len())
                            .map(|point| {
                                let surface_point = if previous
                                    .as_ref()
                                    .is_some_and(|previous_point| point.near(&previous_point.point))
                                {
                                    previous.as_ref().map(|previous_point| {
                                        SurfacePoint::from((
                                            Point2::new(previous_point.x, previous_point.y),
                                            point,
                                        ))
                                    })?
                                } else {
                                    let (u, v) = sp(
                                        surface,
                                        point,
                                        previous.as_ref().map(|point| (point.x, point.y)),
                                    )
                                    .and_then(|uv| {
                                        Self::normalize_uv(
                                            surface,
                                            uv,
                                            previous.as_ref().map(|point| (point.x, point.y)),
                                        )
                                    })?;
                                    SurfacePoint::from((Point2::new(u, v), point))
                                };
                                previous = Some(surface_point);
                                Some(surface_point)
                            })
                            .collect::<Option<Vec<_>>>()?;
                        Some(prefix.into_iter().chain(suffix).collect())
                    };
                    boundary_piece.or_else(projected_piece).or_else(|| {
                        boundary.and_then(|boundary| {
                            Self::from_parameter_boundary(surface, boundary).map(|piece| {
                                previous = piece.0.last().copied();
                                piece.0
                            })
                        })
                    })
                })
                .collect::<Option<Vec<Vec<SurfacePoint>>>>()
                .and_then(|pieces| {
                    let concatenated = pieces.into_iter().fold(
                        Vec::<SurfacePoint>::new(),
                        |mut acc, mut piece| {
                            if !acc.is_empty() && !piece.is_empty() {
                                piece.remove(0);
                            }
                            acc.extend(piece);
                            acc
                        },
                    );
                    Self::from_surface_points(concatenated)
                })
        }
    }

    fn normalize_axis(
        value: f64,
        previous: Option<f64>,
        period: Option<f64>,
        range: Option<(f64, f64)>,
    ) -> Option<f64> {
        if !value.is_finite() {
            None
        } else if let Some(previous) = previous {
            if let Some(period) = period {
                Some(get_mindiff(value, previous, period))
            } else if let Some((min, max)) = range {
                Some(value.clamp(min, max))
            } else {
                Some(value)
            }
        } else if let Some((min, max)) = range {
            if let Some(period) = period {
                let span = max - min;
                if span.so_small() {
                    Some(min)
                } else {
                    let mut normalized = value - f64::floor((value - min) / period) * period;
                    if normalized < min {
                        normalized += period;
                    }
                    if normalized > max {
                        normalized -= period;
                    }
                    Some(normalized.clamp(min, max))
                }
            } else {
                Some(value.clamp(min, max))
            }
        } else {
            Some(value)
        }
    }

    fn normalize_uv<S: PreMeshableSurface>(
        surface: &S,
        uv: (f64, f64),
        previous: Option<(f64, f64)>,
    ) -> Option<(f64, f64)> {
        let (urange, vrange) = surface.try_range_tuple();
        let u = Self::normalize_axis(uv.0, previous.map(|(u, _)| u), surface.u_period(), urange)?;
        let v = Self::normalize_axis(uv.1, previous.map(|(_, v)| v), surface.v_period(), vrange)?;
        Some((u, v))
    }

    fn parameter_seed_score<S: PreMeshableSurface>(surface: &S, (u, v): (f64, f64)) -> f64 {
        let uder = surface.uder(u, v);
        let vder = surface.vder(u, v);
        uder.dot(uder) + vder.dot(vder)
    }

    fn project_loop<S: PreMeshableSurface>(
        surface: &S,
        boundary: &[Point3],
        sp: &impl SP<S>,
        start: usize,
        initial_uv: Option<(f64, f64)>,
    ) -> Option<Vec<SurfacePoint>> {
        let mut initial_uv = initial_uv;
        boundary
            .iter()
            .copied()
            .cycle()
            .skip(start)
            .take(boundary.len() + 1)
            .scan(None, |previous, pt| {
                let uv = initial_uv
                    .take()
                    .or_else(|| sp(surface, pt, *previous))
                    .and_then(|uv| Self::normalize_uv(surface, uv, *previous))
                    .map(|(u, v)| {
                        let points = if let Some((u0, v0)) = *previous {
                            if !u0.near(&u) && surface.uder(u0, v0).so_small() {
                                vec![
                                    SurfacePoint::from((Point2::new(u, v0), pt)),
                                    SurfacePoint::from((Point2::new(u, v), pt)),
                                ]
                            } else if !v0.near(&v) && surface.vder(u0, v0).so_small() {
                                vec![
                                    SurfacePoint::from((Point2::new(u0, v), pt)),
                                    SurfacePoint::from((Point2::new(u, v), pt)),
                                ]
                            } else {
                                vec![SurfacePoint::from((Point2::new(u, v), pt))]
                            }
                        } else {
                            vec![SurfacePoint::from((Point2::new(u, v), pt))]
                        };
                        *previous = Some((u, v));
                        points
                    });
                Some(uv)
            })
            .collect::<Option<Vec<Vec<SurfacePoint>>>>()
            .map(|chunks| chunks.into_iter().flatten().collect())
    }

    pub(super) fn try_new<S: PreMeshableSurface>(
        surface: &S,
        wire: impl Iterator<Item = PolylineCurve>,
        sp: impl SP<S>,
    ) -> Option<Self> {
        let (urange, vrange) = surface.try_range_tuple();
        let (bdry3d, candidate_starts) = wire.fold(
            (Vec::<Point3>::new(), Vec::<usize>::new()),
            |(mut boundary, mut starts), poly_edge| {
                let edge_len = poly_edge.len().saturating_sub(1);
                if edge_len != 0 {
                    starts.push(boundary.len());
                    boundary.extend(poly_edge.into_iter().take(edge_len));
                }
                (boundary, starts)
            },
        );
        if bdry3d.is_empty() {
            return None;
        }
        let mut vec = Self::project_loop(surface, &bdry3d, &sp, 0, None).or_else(|| {
            candidate_starts
                .into_iter()
                .filter(|start| *start != 0)
                .filter_map(|start| {
                    let point = bdry3d[start];
                    let uv = sp(surface, point, None)
                        .and_then(|uv| Self::normalize_uv(surface, uv, None))?;
                    Some((Self::parameter_seed_score(surface, uv), start, uv))
                })
                .max_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0))
                .and_then(|(_, start, uv)| {
                    Self::project_loop(surface, &bdry3d, &sp, start, Some(uv))
                })
                .or_else(|| {
                    bdry3d
                        .iter()
                        .copied()
                        .enumerate()
                        .filter(|(start, _)| *start != 0)
                        .filter_map(|(start, point)| {
                            let uv = sp(surface, point, None)
                                .and_then(|uv| Self::normalize_uv(surface, uv, None))?;
                            Some((Self::parameter_seed_score(surface, uv), start, uv))
                        })
                        .max_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0))
                        .and_then(|(_, start, uv)| {
                            Self::project_loop(surface, &bdry3d, &sp, start, Some(uv))
                        })
                })
        })?;
        let grav = vec.iter().fold(Point2::origin(), |g, p| g + p.uv.to_vec()) / vec.len() as f64;
        if let (Some(up), Some((u0, _))) = (surface.u_period(), urange) {
            let quot = f64::floor((grav.x - u0) / up);
            vec.iter_mut().for_each(|p| p.x -= quot * up);
        }
        if let (Some(vp), Some((v0, _))) = (surface.v_period(), vrange) {
            let quot = f64::floor((grav.y - v0) / vp);
            vec.iter_mut().for_each(|p| p.y -= quot * vp);
        }
        // SAFETY: vec is non-empty because it was built from a non-empty boundary.
        let last = *vec.last().unwrap();
        if !vec[0].near(&last) {
            let Point2 { x: u0, y: v0 } = last.uv;
            if surface.uder(u0, v0).so_small() || surface.vder(u0, v0).so_small() {
                vec.push(vec[0]);
            }
        }
        Some(Self(vec))
    }
}

fn abs_diff(previous: f64) -> impl Fn(&f64, &f64) -> std::cmp::Ordering {
    let f = move |x: &f64| f64::abs(x - previous);
    // SAFETY: UV parameters from surface evaluation are finite, so comparison succeeds.
    move |x: &f64, y: &f64| f(x).partial_cmp(&f(y)).unwrap()
}
fn get_mindiff(u: f64, u0: f64, up: f64) -> f64 {
    let closure = |i| u + i as f64 * up;
    // SAFETY: the iterator (-2..=2) is non-empty, containing five elements.
    (-2..=2).map(closure).min_by(abs_diff(u0)).unwrap()
}

#[derive(Debug, Clone)]
pub(super) struct PolyBoundary {
    pub(super) loops: Vec<Vec<SurfacePoint>>,
    /// UV-space axis-aligned bounding box for cheap rejection in `include()`.
    pub(super) uv_min: Point2,
    pub(super) uv_max: Point2,
}

impl Default for PolyBoundary {
    fn default() -> Self {
        Self {
            loops: Vec::new(),
            uv_min: Point2::new(f64::INFINITY, f64::INFINITY),
            uv_max: Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        }
    }
}

fn normalize_range(curve: &mut Vec<SurfacePoint>, compidx: usize, (u0, u1): (f64, f64)) {
    let p = curve[0];
    let q = curve[curve.len() - 1];
    let tmp = f64::min(p[compidx], q[compidx]) + TOLERANCE;
    let del = f64::floor((tmp - u0) / (u1 - u0)) * (u1 - u0);
    curve.iter_mut().for_each(|p| p[compidx] -= del);
    let Some(i) = curve
        .iter()
        .position(|p| (curve[0][compidx] - u1) * (p[compidx] - u1) < 0.0)
    else {
        return;
    };
    let mut curve1 = curve.split_off(i + 1);
    curve1.pop();
    curve1.insert(0, curve[i]);
    match curve[0][compidx] < curve[curve.len() - 1][compidx] {
        true => curve1.iter_mut(),
        false => curve.iter_mut(),
    }
    .for_each(|p| p[compidx] -= u1 - u0);
    curve1.append(curve);
    *curve = curve1;
}

fn loop_orientation(curve: &[SurfacePoint]) -> bool {
    curve
        .iter()
        .circular_tuple_windows()
        .fold(0.0, |sum, (p, q)| sum + (q.x + p.x) * (q.y - p.y))
        > 0.0
}

pub(super) type UvKey = (u64, u64);

pub(super) fn uv_key(uv: Point2) -> UvKey { (uv.x.to_bits(), uv.y.to_bits()) }

pub(super) fn surface_point_with_cache(
    surface: &impl PreMeshableSurface,
    uv: Point2,
    point_cache: &mut HashMap<UvKey, Point3>,
) -> SurfacePoint {
    let point = *point_cache
        .entry(uv_key(uv))
        .or_insert_with(|| surface.subs(uv.x, uv.y));
    (uv, point).into()
}

impl PolyBoundary {
    pub(super) fn new(
        pieces: Vec<PolyBoundaryPiece>,
        surface: &impl PreMeshableSurface,
        tolerance: f64,
    ) -> Self {
        let (mut closed, mut open) = (Vec::new(), Vec::new());
        pieces.into_iter().for_each(|PolyBoundaryPiece(mut vec)| {
            match vec[0].uv.distance(vec[vec.len() - 1].uv) < 1.0e-3 {
                true => {
                    vec.pop();
                    closed.push(vec)
                }
                false => open.push(vec),
            }
        });
        fn connect_edges<P>(vecs: impl IntoIterator<Item = Vec<P>>) -> Vec<P> {
            let closure = |vec: Vec<P>| {
                let len = vec.len();
                vec.into_iter().take(len - 1)
            };
            vecs.into_iter().flat_map(closure).collect()
        }
        let mut point_cache = HashMap::<UvKey, Point3>::default();
        match open.len() {
            1 => {
                // SAFETY: open.len() == 1 was matched above.
                let mut curve = open.pop().unwrap();
                let p = curve[0];
                let q = curve[curve.len() - 1];
                if let (Some((u0, u1)), Some((v0, v1))) = surface.try_range_tuple() {
                    if p.x < q.x - TOLERANCE {
                        normalize_range(&mut curve, 0, (u0, u1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = surface_point_with_cache(
                            surface,
                            Point2::new(u0, v1),
                            &mut point_cache,
                        );
                        let y = surface_point_with_cache(
                            surface,
                            Point2::new(u1, v1),
                            &mut point_cache,
                        );
                        let vec0 = polyline_on_surface(surface, q, y, tolerance, &mut point_cache);
                        let vec1 = polyline_on_surface(surface, y, x, tolerance, &mut point_cache);
                        let vec2 = polyline_on_surface(surface, x, p, tolerance, &mut point_cache);
                        closed.push(connect_edges([vec0, vec1, vec2, curve]));
                    } else if q.x < p.x - TOLERANCE {
                        normalize_range(&mut curve, 0, (u0, u1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = surface_point_with_cache(
                            surface,
                            Point2::new(u1, v0),
                            &mut point_cache,
                        );
                        let y = surface_point_with_cache(
                            surface,
                            Point2::new(u0, v0),
                            &mut point_cache,
                        );
                        let vec0 = polyline_on_surface(surface, q, y, tolerance, &mut point_cache);
                        let vec1 = polyline_on_surface(surface, y, x, tolerance, &mut point_cache);
                        let vec2 = polyline_on_surface(surface, x, p, tolerance, &mut point_cache);
                        closed.push(connect_edges([vec0, vec1, vec2, curve]));
                    } else if p.y < q.y - TOLERANCE {
                        normalize_range(&mut curve, 1, (v0, v1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = surface_point_with_cache(
                            surface,
                            Point2::new(u0, v0),
                            &mut point_cache,
                        );
                        let y = surface_point_with_cache(
                            surface,
                            Point2::new(u0, v1),
                            &mut point_cache,
                        );
                        let vec0 = polyline_on_surface(surface, q, y, tolerance, &mut point_cache);
                        let vec1 = polyline_on_surface(surface, y, x, tolerance, &mut point_cache);
                        let vec2 = polyline_on_surface(surface, x, p, tolerance, &mut point_cache);
                        closed.push(connect_edges([vec0, vec1, vec2, curve]));
                    } else if q.y < p.y - TOLERANCE {
                        normalize_range(&mut curve, 1, (v0, v1));
                        let p = curve[0];
                        let q = curve[curve.len() - 1];
                        let x = surface_point_with_cache(
                            surface,
                            Point2::new(u1, v1),
                            &mut point_cache,
                        );
                        let y = surface_point_with_cache(
                            surface,
                            Point2::new(u1, v0),
                            &mut point_cache,
                        );
                        let vec0 = polyline_on_surface(surface, q, y, tolerance, &mut point_cache);
                        let vec1 = polyline_on_surface(surface, y, x, tolerance, &mut point_cache);
                        let vec2 = polyline_on_surface(surface, x, p, tolerance, &mut point_cache);
                        closed.push(connect_edges([vec0, vec1, vec2, curve]));
                    }
                }
            }
            2 => {
                // SAFETY: open.len() == 2 was matched above.
                let mut curve1 = open.pop().unwrap();
                let mut curve0 = open.pop().unwrap();
                fn end_pts<T: Copy>(vec: &[T]) -> (T, T) { (vec[0], vec[vec.len() - 1]) }
                let ((p0, p1), (q0, q1)) = (end_pts(&curve0), end_pts(&curve1));
                if !p0.x.near(&p1.x) && !q0.x.near(&q1.x) {
                    if let (Some(urange), _) = surface.try_range_tuple() {
                        normalize_range(&mut curve0, 0, urange);
                        normalize_range(&mut curve1, 0, urange);
                    }
                } else if !p0.y.near(&p1.y)
                    && !q0.y.near(&q1.y)
                    && let (_, Some(vrange)) = surface.try_range_tuple()
                {
                    normalize_range(&mut curve0, 1, vrange);
                    normalize_range(&mut curve1, 1, vrange);
                }
                let ((p0, p1), (q0, q1)) = (end_pts(&curve0), end_pts(&curve1));
                let vec0 = polyline_on_surface(surface, p1, q0, tolerance, &mut point_cache);
                let vec1 = polyline_on_surface(surface, q1, p0, tolerance, &mut point_cache);
                closed.push(connect_edges([curve0, vec0, curve1, vec1]));
            }
            _ => {}
        }
        if !closed.iter().any(|curve| loop_orientation(curve))
            && let (Some((u0, u1)), Some((v0, v1))) = surface.try_range_tuple()
        {
            let p = [
                surface_point_with_cache(surface, Point2::new(u0, v0), &mut point_cache),
                surface_point_with_cache(surface, Point2::new(u1, v0), &mut point_cache),
                surface_point_with_cache(surface, Point2::new(u1, v1), &mut point_cache),
                surface_point_with_cache(surface, Point2::new(u0, v1), &mut point_cache),
            ];
            let vec0 = polyline_on_surface(surface, p[0], p[1], tolerance, &mut point_cache);
            let vec1 = polyline_on_surface(surface, p[1], p[2], tolerance, &mut point_cache);
            let vec2 = polyline_on_surface(surface, p[2], p[3], tolerance, &mut point_cache);
            let vec3 = polyline_on_surface(surface, p[3], p[0], tolerance, &mut point_cache);
            closed.push(connect_edges([vec0, vec1, vec2, vec3]));
        }
        let (mut uv_min, mut uv_max) = (
            Point2::new(f64::INFINITY, f64::INFINITY),
            Point2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for pt in closed.iter().flatten() {
            uv_min.x = f64::min(uv_min.x, pt.x);
            uv_min.y = f64::min(uv_min.y, pt.y);
            uv_max.x = f64::max(uv_max.x, pt.x);
            uv_max.y = f64::max(uv_max.y, pt.y);
        }
        Self {
            loops: closed,
            uv_min,
            uv_max,
        }
    }

    /// Whether `c` is included in the domain with boundary = `self`.
    pub(super) fn include(&self, c: Point2) -> bool {
        // AABB early reject.
        if c.x < self.uv_min.x || c.x > self.uv_max.x || c.y < self.uv_min.y || c.y > self.uv_max.y
        {
            return false;
        }
        let t = 2.0 * std::f64::consts::PI * HashGen::hash1(c);
        let r = Vector2::new(f64::cos(t), f64::sin(t));
        self.loops
            .iter()
            .flat_map(|vec| vec.iter().circular_tuple_windows())
            .try_fold(0_i32, move |counter, (p0, p1)| {
                let a = **p0 - c;
                let b = **p1 - c;
                let s0 = r.x * a.y - r.y * a.x; // v times a.
                let s1 = r.x * b.y - r.y * b.x; // v times b.
                let s2 = a.x * b.y - a.y * b.x; // a times b.
                let x = s2 / (s1 - s0);
                if x.so_small() && s0 * s1 < 0.0 {
                    None
                } else if x > 0.0 && s0 <= 0.0 && s1 > 0.0 {
                    Some(counter + 1)
                } else if x > 0.0 && s0 >= 0.0 && s1 < 0.0 {
                    Some(counter - 1)
                } else {
                    Some(counter)
                }
            })
            .map(|counter| counter > 0)
            .unwrap_or(false)
    }

    /// Inserts points and adds constraint into triangulation.
    pub(super) fn insert_to(
        &self,
        triangulation: &mut Cdt,
        boundary_map: &mut HashMap<FixedVertexHandle, Point3>,
    ) -> (usize, usize, usize) {
        let poly2tri: Vec<_> = self
            .loops
            .iter()
            .flatten()
            .map(|pt| {
                let p = [spade_round(pt.x), spade_round(pt.y)];
                match triangulation.insert(SPoint2::from(p)) {
                    Err(_) => None,
                    Ok(idx) => {
                        boundary_map.insert(idx, pt.point);
                        Some(idx)
                    }
                }
            })
            .collect();
        let vertex_positions = triangulation
            .vertices()
            .map(|vertex| {
                let point = *vertex.as_ref();
                (vertex.fix(), Point2::new(point.x, point.y))
            })
            .collect::<HashMap<_, _>>();
        let mut prev: Option<usize> = None;
        let mut counter = 0;
        let mut added_constraints = 0usize;
        let mut skipped_constraints = 0usize;
        let mut add_constraint = |front: FixedVertexHandle, back: FixedVertexHandle| {
            if triangulation.can_add_constraint(front, back) {
                triangulation.add_constraint(front, back);
                added_constraints += 1;
                true
            } else if let (Some(front_uv), Some(back_uv)) =
                (vertex_positions.get(&front), vertex_positions.get(&back))
            {
                let mut chain = vertex_positions
                    .iter()
                    .filter_map(|(handle, point)| {
                        if *handle == front || *handle == back {
                            None
                        } else {
                            boundary_segment_parameter(*point, *front_uv, *back_uv)
                                .map(|parameter| (parameter, *handle))
                        }
                    })
                    .collect::<Vec<_>>();
                chain.sort_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0));
                let handles = std::iter::once(front)
                    .chain(chain.into_iter().map(|(_, handle)| handle))
                    .chain(std::iter::once(back))
                    .collect::<Vec<_>>();
                let success = handles.len() > 2
                    && handles.windows(2).all(|window| {
                        window[0] == window[1]
                            || if triangulation.can_add_constraint(window[0], window[1]) {
                                triangulation.add_constraint(window[0], window[1]);
                                added_constraints += 1;
                                true
                            } else {
                                false
                            }
                    });
                if !success {
                    skipped_constraints += 1;
                }
                success
            } else {
                skipped_constraints += 1;
                false
            }
        };
        self.loops
            .iter()
            .map(Vec::len)
            .flat_map(|len| {
                let range = counter..counter + len;
                counter += len;
                range.circular_tuple_windows()
            })
            .for_each(|(i, j)| {
                let Some(vj) = poly2tri[j] else { return };
                if let Some(p) = prev {
                    let Some(v) = poly2tri[p] else { return };
                    if add_constraint(v, vj) {
                        prev = None;
                    }
                } else {
                    let Some(vi) = poly2tri[i] else { return };
                    if !add_constraint(vi, vj) {
                        prev = Some(i);
                    }
                }
            });
        (boundary_map.len(), added_constraints, skipped_constraints)
    }
}

fn polyline_on_surface(
    surface: impl PreMeshableSurface,
    p: SurfacePoint,
    q: SurfacePoint,
    tolerance: f64,
    point_cache: &mut HashMap<UvKey, Point3>,
) -> Vec<SurfacePoint> {
    use monstertruck_geometry::prelude::*;
    let line = Line(p.uv, q.uv);
    let pcurve = ParameterCurve::new(line, &surface);
    let (vec, _) = pcurve.parameter_division(pcurve.range_tuple(), tolerance);
    vec.into_iter()
        .map(|t| {
            let uv = line.subs(t);
            surface_point_with_cache(&surface, uv, point_cache)
        })
        .collect()
}
