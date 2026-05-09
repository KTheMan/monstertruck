use super::{Result, *};
trait StepAssociatedEntity {
    fn fmt(&self, idx: usize, formatter: &mut Formatter<'_>) -> Result;
    fn step_length(&self) -> usize;
}

impl<T> StepAssociatedEntity for T
where T: DisplayByStep + StepLength
{
    fn fmt(&self, idx: usize, formatter: &mut Formatter<'_>) -> Result {
        DisplayByStep::fmt(self, idx, formatter)
    }

    fn step_length(&self) -> usize { StepLength::step_length(self) }
}

enum StepAssociatedGeometry<'a, S> {
    ExactParameterCurve(&'a dyn StepAssociatedEntity),
    Surface(&'a S),
}

impl<S> DisplayByStep for StepAssociatedGeometry<'_, S>
where S: DisplayByStep + StepLength
{
    fn fmt(&self, idx: usize, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::ExactParameterCurve(curve) => curve.fmt(idx, formatter),
            Self::Surface(surface) => DisplayByStep::fmt(surface, idx, formatter),
        }
    }
}

impl<S> StepLength for StepAssociatedGeometry<'_, S>
where S: StepLength
{
    fn step_length(&self) -> usize {
        match self {
            Self::ExactParameterCurve(curve) => curve.step_length(),
            Self::Surface(surface) => surface.step_length(),
        }
    }
}

struct StepFace<'a, S> {
    boundaries: Vec<Vec<CompressedEdgeIndex>>,
    orientation: bool,
    surface: &'a S,
}

struct StepSurfaceCurve<'a, C, S> {
    leader: &'a C,
    associated_geometry: Vec<StepAssociatedGeometry<'a, S>>,
}

impl<C, S> DisplayByStep for StepSurfaceCurve<'_, C, S>
where
    C: DisplayByStep + StepLength,
    S: DisplayByStep + StepLength,
{
    fn fmt(&self, idx: usize, formatter: &mut Formatter<'_>) -> Result {
        let leader_idx = idx + 1;
        let (associated_indices, _) = self.associated_geometry.iter().fold(
            (
                Vec::<usize>::with_capacity(self.associated_geometry.len()),
                leader_idx + self.leader.step_length(),
            ),
            |(mut indices, cursor), entry| {
                indices.push(cursor);
                (indices, cursor + StepLength::step_length(entry))
            },
        );
        formatter.write_fmt(format_args!(
            "#{idx} = SURFACE_CURVE('', #{leader_idx}, {associated_geometry}, .CURVE_3D.);\n",
            associated_geometry = IndexSliceDisplay(associated_indices.iter().copied()),
        ))?;
        DisplayByStep::fmt(self.leader, leader_idx, formatter)?;
        self.associated_geometry
            .iter()
            .zip(associated_indices)
            .try_for_each(|(entry, entry_idx)| DisplayByStep::fmt(entry, entry_idx, formatter))
    }
}

impl<C, S> StepLength for StepSurfaceCurve<'_, C, S>
where
    C: StepLength,
    S: StepLength,
{
    fn step_length(&self) -> usize {
        1 + self.leader.step_length()
            + self
                .associated_geometry
                .iter()
                .map(StepLength::step_length)
                .sum::<usize>()
    }
}

impl<C, S> StepCurve for StepSurfaceCurve<'_, C, S>
where C: StepCurve
{
    fn same_sense(&self) -> bool { self.leader.same_sense() }
}

enum StepEdgeGeometry<'a, C, S> {
    Curve(&'a C),
    SurfaceCurve(StepSurfaceCurve<'a, C, S>),
}

impl<C, S> DisplayByStep for StepEdgeGeometry<'_, C, S>
where
    C: DisplayByStep + StepLength,
    S: DisplayByStep + StepLength,
{
    fn fmt(&self, idx: usize, formatter: &mut Formatter<'_>) -> Result {
        match self {
            Self::Curve(curve) => DisplayByStep::fmt(curve, idx, formatter),
            Self::SurfaceCurve(curve) => DisplayByStep::fmt(curve, idx, formatter),
        }
    }
}

impl<C, S> StepLength for StepEdgeGeometry<'_, C, S>
where
    C: StepLength,
    S: StepLength,
{
    fn step_length(&self) -> usize {
        match self {
            Self::Curve(curve) => curve.step_length(),
            Self::SurfaceCurve(curve) => curve.step_length(),
        }
    }
}

impl<C, S> StepCurve for StepEdgeGeometry<'_, C, S>
where C: StepCurve
{
    fn same_sense(&self) -> bool {
        match self {
            Self::Curve(curve) => curve.same_sense(),
            Self::SurfaceCurve(curve) => curve.same_sense(),
        }
    }
}

pub(super) struct StepShell<'a, P, C, S> {
    vertices: &'a [P],
    edges: &'a [CompressedEdge<C>],
    faces: Vec<StepFace<'a, S>>,
    idx: usize,
    face_indices: Vec<usize>,
    ep_edges: usize,
    ep_vertices: usize,
    surface_indices: Vec<usize>,
    edge_geometries: Vec<StepEdgeGeometry<'a, C, S>>,
    curve_indices: Vec<usize>,
    ep_points: usize,
    is_open: bool,
}

impl<'a, P, C, S> StepShell<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn new_curve3d_only(shell: &'a CompressedShell<P, C, S>, idx: usize, is_open: bool) -> Self {
        let faces = shell
            .faces
            .iter()
            .map(|face| StepFace {
                boundaries: face.boundaries.clone(),
                orientation: face.orientation,
                surface: &face.surface,
            })
            .collect::<Vec<_>>();
        let edge_associations = std::iter::repeat_with(Vec::<StepAssociatedGeometry<'a, S>>::new)
            .take(shell.edges.len())
            .collect::<Vec<_>>();
        Self::from_parts(
            &shell.vertices,
            &shell.edges,
            faces,
            edge_associations,
            idx,
            is_open,
        )
    }

    fn new(shell: &'a CompressedShell<P, C, S>, idx: usize, is_open: bool) -> Self {
        let faces = shell
            .faces
            .iter()
            .map(|face| StepFace {
                boundaries: face.boundaries.clone(),
                orientation: face.orientation,
                surface: &face.surface,
            })
            .collect::<Vec<_>>();
        let mut edge_associations =
            std::iter::repeat_with(Vec::<StepAssociatedGeometry<'a, S>>::new)
                .take(shell.edges.len())
                .collect::<Vec<_>>();
        faces.iter().for_each(|face| {
            face.boundaries.iter().for_each(|wire| {
                wire.iter().for_each(|ce| {
                    if let Some(associations) = edge_associations.get_mut(ce.index) {
                        associations.push(StepAssociatedGeometry::Surface(face.surface));
                    }
                });
            });
        });
        Self::from_parts(
            &shell.vertices,
            &shell.edges,
            faces,
            edge_associations,
            idx,
            is_open,
        )
    }
}

impl<'a, P, C, S> StepShell<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn new_trimmed<T>(
        shell: &'a CompressedTrimmedShell<P, C, S, T>,
        idx: usize,
        is_open: bool,
    ) -> Self
    where
        T: DisplayByStep + StepLength,
    {
        let faces = shell
            .faces
            .iter()
            .map(|face| StepFace {
                boundaries: face
                    .boundaries
                    .iter()
                    .map(|wire| {
                        wire.iter()
                            .map(
                                |CompressedEdgeUse {
                                     index, orientation, ..
                                 }| {
                                    CompressedEdgeIndex {
                                        index: *index,
                                        orientation: *orientation,
                                    }
                                },
                            )
                            .collect()
                    })
                    .collect(),
                orientation: face.orientation,
                surface: &face.surface,
            })
            .collect::<Vec<_>>();
        let mut edge_associations =
            std::iter::repeat_with(Vec::<StepAssociatedGeometry<'a, S>>::new)
                .take(shell.edges.len())
                .collect::<Vec<_>>();
        shell.faces.iter().for_each(|face| {
            face.boundaries.iter().for_each(|wire| {
                wire.iter().for_each(|edge_use| {
                    let association = edge_use
                        .trim_curve
                        .as_ref()
                        .map(|trim_curve| StepAssociatedGeometry::ExactParameterCurve(trim_curve))
                        .unwrap_or_else(|| StepAssociatedGeometry::Surface(&face.surface));
                    edge_associations[edge_use.index].push(association);
                });
            });
        });
        Self::from_parts(
            &shell.vertices,
            &shell.edges,
            faces,
            edge_associations,
            idx,
            is_open,
        )
    }

    fn from_parts(
        vertices: &'a [P],
        edges: &'a [CompressedEdge<C>],
        faces: Vec<StepFace<'a, S>>,
        mut edge_associations: Vec<Vec<StepAssociatedGeometry<'a, S>>>,
        idx: usize,
        is_open: bool,
    ) -> Self {
        let mut cursor = idx + 1;
        let face_indices = faces
            .iter()
            .map(|face| {
                let res = cursor;
                cursor += match face.boundaries.is_empty() {
                    true => 5,
                    false => {
                        1 + face
                            .boundaries
                            .iter()
                            .map(|boundary| 2 + boundary.len())
                            .sum::<usize>()
                    }
                };
                res
            })
            .collect::<Vec<_>>();
        let ep_edges = cursor;
        let ep_vertices = ep_edges + edges.len();
        cursor = ep_vertices + vertices.len();
        let surface_indices = faces
            .iter()
            .map(|face| {
                let res = cursor;
                cursor += face.surface.step_length();
                res
            })
            .collect::<Vec<_>>();
        let edge_geometries = edges
            .iter()
            .enumerate()
            .map(|(i, edge)| {
                if edge_associations[i].is_empty() {
                    StepEdgeGeometry::Curve(&edge.curve)
                } else {
                    StepEdgeGeometry::SurfaceCurve(StepSurfaceCurve {
                        leader: &edge.curve,
                        associated_geometry: std::mem::take(&mut edge_associations[i]),
                    })
                }
            })
            .collect::<Vec<_>>();
        let curve_indices = edge_geometries
            .iter()
            .map(|geometry| {
                let res = cursor;
                cursor += geometry.step_length();
                res
            })
            .collect::<Vec<_>>();
        let ep_points = cursor;
        StepShell {
            vertices,
            edges,
            faces,
            idx,
            face_indices,
            ep_edges,
            ep_vertices,
            surface_indices,
            edge_geometries,
            curve_indices,
            ep_points,
            is_open,
        }
    }
}

impl<P, C, S> Display for StepShell<'_, P, C, S>
where
    P: DisplayByStep + Copy,
    C: DisplayByStep + StepLength + StepCurve,
    S: DisplayByStep + StepLength + StepSurface,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result {
        let StepShell {
            vertices,
            edges,
            faces,
            idx,
            face_indices,
            ep_edges,
            ep_vertices,
            surface_indices,
            edge_geometries,
            curve_indices,
            ep_points,
            is_open,
        } = self;
        let shell_kind = match is_open {
            true => "OPEN_SHELL",
            false => "CLOSED_SHELL",
        };
        formatter.write_fmt(format_args!(
            "#{idx} = {shell_kind}('', {face_indices});\n",
            face_indices = IndexSliceDisplay(self.face_indices.clone()),
        ))?;
        faces.iter().enumerate().try_for_each(|(i, f)| {
            let idx = face_indices[i];
            let mut cursor = idx + 1;
            let face_geometry = surface_indices[i];
            let face_bounds = match f.boundaries.is_empty() {
                true => vec![cursor],
                false => {
                    let closure = |b: &Vec<CompressedEdgeIndex>| {
                        let res = cursor;
                        cursor += 2 + b.len();
                        res
                    };
                    f.boundaries.iter().map(closure).collect()
                }
            };
            formatter.write_fmt(format_args!(
                "#{idx} = FACE_SURFACE('', {face_bound}, #{face_geometry}, {same_sense});\n",
                same_sense = BooleanDisplay(f.orientation == f.surface.same_sense()),
                face_bound = IndexSliceDisplay(face_bounds.clone()),
            ))?;
            cursor = idx + 1;
            if f.boundaries.is_empty() {
                let face_bound_idx = cursor;
                let vertex_loop_idx = cursor + 1;
                let vertex_idx = cursor + 2;
                let vertex_geometry = cursor + 3;
                formatter.write_fmt(format_args!(
                    "#{face_bound_idx} = FACE_BOUND('', #{vertex_loop_idx}, .T.);
#{vertex_loop_idx} = VERTEX_LOOP('', #{vertex_idx});
#{vertex_idx} = VERTEX_POINT('', #{vertex_geometry});
#{vertex_geometry} = POINT_ON_SURFACE('', #{face_geometry}, 0.0, 0.0);\n"
                ))?;
            }
            f.boundaries.iter().try_for_each(|b| {
                let face_bound_idx = cursor;
                let edge_loop_idx = cursor + 1;
                let ep_oriented_edges = cursor + 2;
                cursor += 2 + b.len();
                formatter.write_fmt(format_args!(
                    "#{face_bound_idx} = FACE_BOUND('', #{edge_loop_idx}, {orientation});
#{edge_loop_idx} = EDGE_LOOP('', {oriented_edge_indices});\n",
                    orientation = BooleanDisplay(f.orientation),
                    oriented_edge_indices =
                        IndexSliceDisplay(ep_oriented_edges..ep_oriented_edges + b.len()),
                ))?;
                b.iter().enumerate().try_for_each(|(j, ce)| {
                    formatter.write_fmt(format_args!(
                        "#{idx} = ORIENTED_EDGE('', *, *, #{edge_element}, {orientation});\n",
                        idx = ep_oriented_edges + j,
                        edge_element = ep_edges + ce.index,
                        orientation = if ce.orientation { ".T." } else { ".F." },
                    ))
                })
            })
        })?;
        edge_geometries
            .iter()
            .enumerate()
            .try_for_each(|(i, geometry)| {
                let same_sense = if geometry.same_sense() { ".T." } else { ".F." };
                formatter.write_fmt(format_args!(
                    "#{idx} = EDGE_CURVE('', #{edge_start}, #{edge_end}, #{edge_geometry}, {same_sense});\n",
                    idx = ep_edges + i,
                    edge_start = ep_vertices + edges[i].vertices.0,
                    edge_end = ep_vertices + edges[i].vertices.1,
                    edge_geometry = curve_indices[i],
                ))
            })?;
        (0..vertices.len()).try_for_each(|i| {
            formatter.write_fmt(format_args!(
                "#{idx} = VERTEX_POINT('', #{vertex_geometry});\n",
                idx = ep_vertices + i,
                vertex_geometry = ep_points + i,
            ))
        })?;
        faces.iter().zip(surface_indices).try_for_each(|(f, idx)| {
            Display::fmt(&StepDisplay::new(&f.surface, *idx), formatter)
        })?;
        edge_geometries
            .iter()
            .zip(curve_indices)
            .try_for_each(|(geometry, idx)| DisplayByStep::fmt(geometry, *idx, formatter))?;
        vertices
            .iter()
            .enumerate()
            .try_for_each(|(i, v)| Display::fmt(&StepDisplay::new(*v, ep_points + i), formatter))
    }
}

impl<P, C, S> StepLength for StepShell<'_, P, C, S> {
    fn step_length(&self) -> usize {
        1 + self.ep_points + self.vertices.len() - self.face_indices[0]
    }
}

pub(super) struct StepSolid<'a, P, C, S> {
    idx: usize,
    boundaries: Vec<StepShell<'a, P, C, S>>,
}

impl<'a, P, C, S> StepSolid<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn new_curve3d_only(solid: &'a CompressedSolid<P, C, S>, idx: usize) -> Self {
        let mut cursor = idx + 1;
        let boundaries = solid
            .boundaries
            .iter()
            .map(|shell| {
                let res = StepShell::new_curve3d_only(shell, cursor, false);
                cursor += 1 + res.step_length();
                res
            })
            .collect::<Vec<_>>();
        StepSolid { idx, boundaries }
    }

    fn new(solid: &'a CompressedSolid<P, C, S>, idx: usize) -> Self {
        let mut cursor = idx + 1;
        let boundaries = solid
            .boundaries
            .iter()
            .map(|shell| {
                let res = StepShell::new(shell, cursor, false);
                cursor += 1 + res.step_length();
                res
            })
            .collect::<Vec<_>>();
        StepSolid { idx, boundaries }
    }
}

impl<'a, P, C, S> StepSolid<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn new_trimmed<T>(solid: &'a CompressedTrimmedSolid<P, C, S, T>, idx: usize) -> Self
    where T: DisplayByStep + StepLength {
        let mut cursor = idx + 1;
        let boundaries = solid
            .boundaries
            .iter()
            .map(|shell| {
                let res = StepShell::new_trimmed(shell, cursor, false);
                cursor += 1 + res.step_length();
                res
            })
            .collect::<Vec<_>>();
        StepSolid { idx, boundaries }
    }
}

impl<P, C, S> Display for StepSolid<'_, P, C, S>
where
    P: DisplayByStep + Copy,
    C: DisplayByStep + StepLength + StepCurve,
    S: DisplayByStep + StepLength + StepSurface,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let StepSolid { idx, boundaries } = self;
        match boundaries.len() {
            0 => {
                f.pad("empty solid!")?;
                Err(std::fmt::Error)
            }
            1 => {
                let shell_idx = idx + 1;
                let step_shell = &boundaries[0];
                f.write_fmt(format_args!(
                    "#{idx} = MANIFOLD_SOLID_BREP('', #{shell_idx});\n"
                ))?;
                Display::fmt(step_shell, f)
            }
            _ => {
                let first_shell_idx = boundaries[0].face_indices[0] - 1;
                f.write_fmt(format_args!(
                    "#{idx} = BREP_WITH_VOIDS('', #{first_shell_idx}, {other_shells});\n",
                    other_shells = IndexSliceDisplay(
                        boundaries[1..]
                            .iter()
                            .map(|step_shell| step_shell.face_indices[0] - 2)
                    ),
                ))?;
                Display::fmt(&boundaries[0], f)?;
                boundaries[1..].iter().try_for_each(|step_shell| {
                    let oriented_shell_idx = step_shell.face_indices[0] - 2;
                    let shell_idx = step_shell.face_indices[0] - 1;
                    f.write_fmt(format_args!(
                    "#{oriented_shell_idx} = ORIENTED_CLOSED_SHELL('', *, #{shell_idx}, .T.);\n",
                ))?;
                    Display::fmt(step_shell, f)
                })
            }
        }
    }
}

impl<P, C, S> StepLength for StepSolid<'_, P, C, S> {
    fn step_length(&self) -> usize {
        let b = &self.boundaries;
        match b.len() {
            0 => 0,
            1 => 1 + b[0].step_length(),
            _ => b.len() + b.iter().map(StepLength::step_length).sum::<usize>(),
        }
    }
}

pub(super) enum PreStepModel<'a, P, C, S> {
    /// shell based surface model
    Shell(StepShell<'a, P, C, S>),
    /// solid model
    Solid(StepSolid<'a, P, C, S>),
}

impl<'a, P, C, S> From<&'a CompressedShell<P, C, S>> for PreStepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn from(shell: &'a CompressedShell<P, C, S>) -> Self {
        Self::Shell(StepShell::new(shell, 17, true))
    }
}

impl<'a, P, C, S> From<&'a CompressedSolid<P, C, S>> for PreStepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn from(solid: &'a CompressedSolid<P, C, S>) -> Self { Self::Solid(StepSolid::new(solid, 16)) }
}

impl<'a, P, C, S, T> From<&'a CompressedTrimmedShell<P, C, S, T>> for PreStepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
    T: DisplayByStep + StepLength,
{
    fn from(shell: &'a CompressedTrimmedShell<P, C, S, T>) -> Self {
        Self::Shell(StepShell::new_trimmed(shell, 17, true))
    }
}

impl<'a, P, C, S, T> From<&'a CompressedTrimmedSolid<P, C, S, T>> for PreStepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
    T: DisplayByStep + StepLength,
{
    fn from(solid: &'a CompressedTrimmedSolid<P, C, S, T>) -> Self {
        Self::Solid(StepSolid::new_trimmed(solid, 16))
    }
}

impl<P, C, S> Display for PreStepModel<'_, P, C, S>
where
    P: DisplayByStep + Copy,
    C: DisplayByStep + StepLength + StepCurve,
    S: DisplayByStep + StepLength + StepSurface,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Shell(x) => {
                f.write_fmt(format_args!(
                    "#{idx} = SHELL_BASED_SURFACE_MODEL('', (#{shell_idx}));\n",
                    idx = x.idx - 1,
                    shell_idx = x.idx
                ))?;
                Display::fmt(&x, f)
            }
            Self::Solid(x) => Display::fmt(x, f),
        }
    }
}

impl<P, C, S> StepLength for PreStepModel<'_, P, C, S> {
    fn step_length(&self) -> usize {
        match self {
            Self::Shell(x) => 1 + x.step_length(),
            Self::Solid(x) => x.step_length(),
        }
    }
}

impl<'a, P, C, S> From<&'a CompressedShell<P, C, S>> for StepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn from(shell: &'a CompressedShell<P, C, S>) -> Self { Self(shell.into()) }
}

impl<'a, P, C, S> From<&'a CompressedSolid<P, C, S>> for StepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn from(solid: &'a CompressedSolid<P, C, S>) -> Self { Self(solid.into()) }
}

impl<'a, P, C, S> StepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    /// Creates a STEP model that exports only shared 3-dimensional edge curves.
    pub fn from_curve3d_only_shell(shell: &'a CompressedShell<P, C, S>) -> Self {
        Self(PreStepModel::Shell(StepShell::new_curve3d_only(
            shell, 17, true,
        )))
    }

    /// Creates a STEP model that exports only shared 3-dimensional edge curves.
    pub fn from_curve3d_only_solid(solid: &'a CompressedSolid<P, C, S>) -> Self {
        Self(PreStepModel::Solid(StepSolid::new_curve3d_only(solid, 16)))
    }
}

impl<'a, P, C, S, T> From<&'a CompressedTrimmedShell<P, C, S, T>> for StepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
    T: DisplayByStep + StepLength,
{
    fn from(shell: &'a CompressedTrimmedShell<P, C, S, T>) -> Self { Self(shell.into()) }
}

impl<'a, P, C, S, T> From<&'a CompressedTrimmedSolid<P, C, S, T>> for StepModel<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
    T: DisplayByStep + StepLength,
{
    fn from(solid: &'a CompressedTrimmedSolid<P, C, S, T>) -> Self { Self(solid.into()) }
}

impl<P, C, S> Display for StepModel<'_, P, C, S>
where
    P: DisplayByStep + Copy,
    C: DisplayByStep + StepLength + StepCurve,
    S: DisplayByStep + StepLength + StepSurface,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.pad(
"#1 = APPLICATION_PROTOCOL_DEFINITION('international standard', 'automotive_design', 2000, #2);
#2 = APPLICATION_CONTEXT('core data for automotive mechanical design processes');
#3 = SHAPE_DEFINITION_REPRESENTATION(#4, #10);
#4 = PRODUCT_DEFINITION_SHAPE('','', #5);
#5 = PRODUCT_DEFINITION('design','', #6, #9);
#6 = PRODUCT_DEFINITION_FORMATION('','', #7);
#7 = PRODUCT('','','', (#8));
#8 = PRODUCT_CONTEXT('', #2, 'mechanical');
#9 = PRODUCT_DEFINITION_CONTEXT('part definition', #2, 'design');
#10 = SHAPE_REPRESENTATION('', (#16), #11);
#11 = (
    GEOMETRIC_REPRESENTATION_CONTEXT(3)
    GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#15))
    GLOBAL_UNIT_ASSIGNED_CONTEXT((#12, #13, #14))
    REPRESENTATION_CONTEXT('Context #1', '3D Context with UNIT and UNCERTAINTY')
);
#12 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );
#13 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );
#14 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );
#15 = UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.0E-6), #12, 'distance_accuracy_value','confusion accuracy');\n"
        )?;
        Display::fmt(&self.0, f)
    }
}

impl<P, C, S> Default for StepModels<'_, P, C, S> {
    fn default() -> Self {
        Self {
            models: Vec::new(),
            next_idx: 16,
        }
    }
}

impl<'a, P, C, S> StepModels<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    /// The next available entity index after all pushed models.
    pub fn next_idx(&self) -> usize { self.next_idx }
    /// push a shell to step models
    pub fn push_shell(&mut self, shell: &'a CompressedShell<P, C, S>) {
        let model = PreStepModel::Shell(StepShell::new(shell, self.next_idx + 1, true));
        self.next_idx += model.step_length();
        self.models.push(model)
    }
    /// push a solid to step models
    pub fn push_solid(&mut self, solid: &'a CompressedSolid<P, C, S>) {
        let model = PreStepModel::Solid(StepSolid::new(solid, self.next_idx));
        self.next_idx += model.step_length();
        self.models.push(model)
    }

    /// Pushes a shell while exporting only shared 3-dimensional edge curves.
    pub fn push_curve3d_only_shell(&mut self, shell: &'a CompressedShell<P, C, S>) {
        let model =
            PreStepModel::Shell(StepShell::new_curve3d_only(shell, self.next_idx + 1, true));
        self.next_idx += model.step_length();
        self.models.push(model)
    }

    /// Pushes a solid while exporting only shared 3-dimensional edge curves.
    pub fn push_curve3d_only_solid(&mut self, solid: &'a CompressedSolid<P, C, S>) {
        let model = PreStepModel::Solid(StepSolid::new_curve3d_only(solid, self.next_idx));
        self.next_idx += model.step_length();
        self.models.push(model)
    }
}

impl<'a, P, C, S> StepModels<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    /// Pushes a trimmed shell to step models.
    pub fn push_trimmed_shell<T>(&mut self, shell: &'a CompressedTrimmedShell<P, C, S, T>)
    where T: DisplayByStep + StepLength {
        let model = PreStepModel::Shell(StepShell::new_trimmed(shell, self.next_idx + 1, true));
        self.next_idx += model.step_length();
        self.models.push(model)
    }

    /// Pushes a trimmed solid to step models.
    pub fn push_trimmed_solid<T>(&mut self, solid: &'a CompressedTrimmedSolid<P, C, S, T>)
    where T: DisplayByStep + StepLength {
        let model = PreStepModel::Solid(StepSolid::new_trimmed(solid, self.next_idx));
        self.next_idx += model.step_length();
        self.models.push(model)
    }
}

impl<'a, P, C, S> FromIterator<&'a CompressedShell<P, C, S>> for StepModels<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn from_iter<T: IntoIterator<Item = &'a CompressedShell<P, C, S>>>(iter: T) -> Self {
        let mut next_idx = 16;
        let models = iter
            .into_iter()
            .map(|shell| {
                let model = PreStepModel::Shell(StepShell::new(shell, next_idx + 1, true));
                next_idx += model.step_length();
                model
            })
            .collect();
        Self { models, next_idx }
    }
}

impl<'a, P, C, S> FromIterator<&'a CompressedSolid<P, C, S>> for StepModels<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
{
    fn from_iter<T: IntoIterator<Item = &'a CompressedSolid<P, C, S>>>(iter: T) -> Self {
        let mut next_idx = 16;
        let models = iter
            .into_iter()
            .map(|solid| {
                let model = PreStepModel::Solid(StepSolid::new(solid, next_idx));
                next_idx += model.step_length();
                model
            })
            .collect();
        Self { models, next_idx }
    }
}

impl<'a, P, C, S, U> FromIterator<&'a CompressedTrimmedShell<P, C, S, U>>
    for StepModels<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
    U: DisplayByStep + StepLength,
{
    fn from_iter<T: IntoIterator<Item = &'a CompressedTrimmedShell<P, C, S, U>>>(iter: T) -> Self {
        let mut next_idx = 16;
        let models = iter
            .into_iter()
            .map(|shell| {
                let model = PreStepModel::Shell(StepShell::new_trimmed(shell, next_idx + 1, true));
                next_idx += model.step_length();
                model
            })
            .collect();
        Self { models, next_idx }
    }
}

impl<'a, P, C, S, U> FromIterator<&'a CompressedTrimmedSolid<P, C, S, U>>
    for StepModels<'a, P, C, S>
where
    P: Copy,
    C: StepLength,
    S: StepLength,
    U: DisplayByStep + StepLength,
{
    fn from_iter<T: IntoIterator<Item = &'a CompressedTrimmedSolid<P, C, S, U>>>(iter: T) -> Self {
        let mut next_idx = 16;
        let models = iter
            .into_iter()
            .map(|solid| {
                let model = PreStepModel::Solid(StepSolid::new_trimmed(solid, next_idx));
                next_idx += model.step_length();
                model
            })
            .collect();
        Self { models, next_idx }
    }
}

impl<P, C, S> Display for StepModels<'_, P, C, S>
where
    P: DisplayByStep + Copy,
    C: DisplayByStep + StepLength + StepCurve,
    S: DisplayByStep + StepLength + StepSurface,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.pad(
"#1 = APPLICATION_PROTOCOL_DEFINITION('international standard', 'automotive_design', 2000, #2);
#2 = APPLICATION_CONTEXT('core data for automotive mechanical design processes');
#3 = SHAPE_DEFINITION_REPRESENTATION(#4, #10);
#4 = PRODUCT_DEFINITION_SHAPE('','', #5);
#5 = PRODUCT_DEFINITION('design','', #6, #9);
#6 = PRODUCT_DEFINITION_FORMATION('','', #7);
#7 = PRODUCT('','','', (#8));
#8 = PRODUCT_CONTEXT('', #2, 'mechanical');
#9 = PRODUCT_DEFINITION_CONTEXT('part definition', #2, 'design');\n")?;
        let models_slice = IndexSliceDisplay(self.models.iter().map(|model| match model {
            PreStepModel::Shell(x) => x.idx - 1,
            PreStepModel::Solid(x) => x.idx,
        }));
        f.write_fmt(format_args!(
            "#10 = ADVANCED_BREP_SHAPE_REPRESENTATION('', {models_slice}, #11);\n"
        ))?;
        f.pad("#11 = (
    GEOMETRIC_REPRESENTATION_CONTEXT(3) 
    GLOBAL_UNCERTAINTY_ASSIGNED_CONTEXT((#15))
    GLOBAL_UNIT_ASSIGNED_CONTEXT((#12, #13, #14))
    REPRESENTATION_CONTEXT('Context #1', '3D Context with UNIT and UNCERTAINTY')
);
#12 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );
#13 = ( NAMED_UNIT(*) PLANE_ANGLE_UNIT() SI_UNIT($,.RADIAN.) );
#14 = ( NAMED_UNIT(*) SI_UNIT($,.STERADIAN.) SOLID_ANGLE_UNIT() );
#15 = UNCERTAINTY_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.0E-6), #12, 'distance_accuracy_value','confusion accuracy');\n"
        )?;
        self.models
            .iter()
            .try_for_each(|model| Display::fmt(model, f))
    }
}
