"""Generate independent OCCT STEP fixtures for Phase 5 continuity validation."""

from __future__ import annotations

import hashlib
import json
import math
import re
from pathlib import Path
from typing import Callable, Sequence

import OCP
from OCP.BRepBuilderAPI import (
    BRepBuilderAPI_MakeEdge,
    BRepBuilderAPI_MakeFace,
    BRepBuilderAPI_MakeWire,
    BRepBuilderAPI_Sewing,
)
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepLib import BRepLib
from OCP.GCE2d import GCE2d_MakeSegment
from OCP.Geom import Geom_BSplineSurface
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_AsIs, STEPControl_Writer
from OCP.TColStd import (
    TColStd_Array1OfInteger,
    TColStd_Array1OfReal,
    TColStd_Array2OfReal,
)
from OCP.TColgp import TColgp_Array2OfPnt
from OCP.gp import gp_Pnt, gp_Pnt2d

EXPECTED_OCP_VERSION = "7.8.1.1"
FIXED_STEP_TIMESTAMP = "2000-01-01T00:00:00"
ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / "validation" / "continuity"
Vec3 = tuple[float, float, float]


def _array1_real(values: Sequence[float]) -> TColStd_Array1OfReal:
    result = TColStd_Array1OfReal(1, len(values))
    for index, value in enumerate(values, 1):
        result.SetValue(index, value)
    return result


def _array1_integer(values: Sequence[int]) -> TColStd_Array1OfInteger:
    result = TColStd_Array1OfInteger(1, len(values))
    for index, value in enumerate(values, 1):
        result.SetValue(index, value)
    return result


def _poles(values: Sequence[Sequence[Vec3]]) -> TColgp_Array2OfPnt:
    result = TColgp_Array2OfPnt(1, len(values), 1, len(values[0]))
    for u_index, row in enumerate(values, 1):
        for v_index, point in enumerate(row, 1):
            result.SetValue(u_index, v_index, gp_Pnt(*point))
    return result


def _weights(values: Sequence[Sequence[float]]) -> TColStd_Array2OfReal:
    result = TColStd_Array2OfReal(1, len(values), 1, len(values[0]))
    for u_index, row in enumerate(values, 1):
        for v_index, weight in enumerate(row, 1):
            result.SetValue(u_index, v_index, weight)
    return result


def _surface(
    poles: Sequence[Sequence[Vec3]],
    u_knots: Sequence[float],
    u_multiplicities: Sequence[int],
    v_knots: Sequence[float],
    v_multiplicities: Sequence[int],
    u_degree: int,
    v_degree: int,
    weights: Sequence[Sequence[float]] | None = None,
) -> Geom_BSplineSurface:
    arguments = (
        _poles(poles),
        _array1_real(u_knots),
        _array1_real(v_knots),
        _array1_integer(u_multiplicities),
        _array1_integer(v_multiplicities),
        u_degree,
        v_degree,
        False,
        False,
    )
    if weights is None:
        return Geom_BSplineSurface(*arguments)
    return Geom_BSplineSurface(arguments[0], _weights(weights), *arguments[1:])


def _add(*vectors: Vec3) -> Vec3:
    return tuple(sum(values) for values in zip(*vectors, strict=True))


def _scale(vector: Vec3, factor: float) -> Vec3:
    return tuple(value * factor for value in vector)


def _cross_layers(
    boundary: Sequence[Vec3],
    first: Sequence[Vec3],
    second: Sequence[Vec3],
    third: Sequence[Vec3],
    degree: int,
) -> tuple[list[list[Vec3]], list[list[Vec3]]]:
    first_factor = float(degree)
    second_factor = float(degree * (degree - 1))
    third_factor = float(degree * (degree - 1) * (degree - 2))
    left_near = [
        [
            boundary[index],
            _add(boundary[index], _scale(first[index], -1.0 / first_factor)),
            _add(
                boundary[index],
                _scale(first[index], -2.0 / first_factor),
                _scale(second[index], 1.0 / second_factor),
            ),
            _add(
                boundary[index],
                _scale(first[index], -3.0 / first_factor),
                _scale(second[index], 3.0 / second_factor),
                _scale(third[index], -1.0 / third_factor),
            ),
        ]
        for index in range(len(boundary))
    ]
    right_near = [
        [
            boundary[index],
            _add(boundary[index], _scale(first[index], 1.0 / first_factor)),
            _add(
                boundary[index],
                _scale(first[index], 2.0 / first_factor),
                _scale(second[index], 1.0 / second_factor),
            ),
            _add(
                boundary[index],
                _scale(first[index], 3.0 / first_factor),
                _scale(second[index], 3.0 / second_factor),
                _scale(third[index], 1.0 / third_factor),
            ),
        ]
        for index in range(len(boundary))
    ]
    return (
        [list(layer) for layer in zip(*left_near, strict=True)],
        [list(layer) for layer in zip(*right_near, strict=True)],
    )


def _full_face(surface: Geom_BSplineSurface):
    u_min, u_max, v_min, v_max = surface.Bounds()
    builder = BRepBuilderAPI_MakeFace(
        surface, u_min, u_max, v_min, v_max, 1.0e-9
    )
    if not builder.IsDone():
        raise RuntimeError("OCCT could not construct a full rectangular face.")
    return builder.Face()


def _sew(faces: Sequence) -> object:
    sewing = BRepBuilderAPI_Sewing(1.0e-8)
    for face in faces:
        sewing.Add(face)
    sewing.Perform()
    shape = sewing.SewedShape()
    if shape.IsNull() or not BRepCheck_Analyzer(shape).IsValid():
        raise RuntimeError("OCCT produced an invalid sewn fixture.")
    return shape


def _polynomial_g1() -> tuple[object, dict[str, object]]:
    boundary = [
        (0.0, 0.0, 0.0),
        (0.0, 0.22, 0.13),
        (0.0, 0.48, -0.07),
        (0.0, 0.76, 0.18),
        (0.0, 1.0, 0.03),
    ]
    first = [(1.0, 0.0, 0.20 + 0.05 * index) for index in range(5)]
    left = [
        [
            (-1.0, point[1], point[2] - 0.20),
            (-0.62, point[1], point[2] - 0.12),
            _add(point, _scale(first[index], -1.0 / 3.0)),
            point,
        ]
        for index, point in enumerate(boundary)
    ]
    right = [
        [
            point,
            _add(point, _scale(first[index], 1.0 / 3.0)),
            (0.71, point[1], point[2] + 0.55 + 0.04 * index),
            (1.25, point[1], point[2] + 0.75),
        ]
        for index, point in enumerate(boundary)
    ]
    left_surface = _surface(
        list(map(list, zip(*left, strict=True))),
        [0.0, 1.0],
        [4, 4],
        [0.0, 0.37, 1.0],
        [4, 1, 4],
        3,
        3,
    )
    right_surface = _surface(
        list(map(list, zip(*right, strict=True))),
        [0.0, 1.0],
        [4, 4],
        [0.0, 0.37, 1.0],
        [4, 1, 4],
        3,
        3,
    )
    _assert_continuity(left_surface, right_surface, 1, lambda value: value)
    _assert_discontinuity(left_surface, right_surface, 2, lambda value: value)
    return (
        _sew([_full_face(left_surface), _full_face(right_surface)]),
        {
            "expected_continuity": "G1",
            "construction": "Polynomial bicubic/multi-span pair; C1 cross-seam derivatives match and second derivatives intentionally differ.",
            "full_rectangular_faces": True,
            "rational": False,
            "reversed_boundary_parameter": False,
            "u_degrees": [3, 3],
            "v_degrees": [3, 3],
        },
    )


def _rational_reversed_g2() -> tuple[object, dict[str, object]]:
    boundary = [
        (0.0, 0.0, 0.0),
        (0.0, 0.18, 0.28),
        (0.0, 0.52, -0.16),
        (0.0, 0.81, 0.24),
        (0.0, 1.0, 0.04),
    ]
    first = [(1.1, 0.0, 0.12 + 0.04 * index) for index in range(5)]
    second = [(0.0, 0.0, -0.18 + 0.06 * index) for index in range(5)]
    zero = [(0.0, 0.0, 0.0)] * 5
    left_near, right_near = _cross_layers(boundary, first, second, zero, 3)
    left = [
        [
            (-1.35, point[1], point[2] - 0.65 - 0.03 * index)
            for index, point in enumerate(boundary)
        ],
        left_near[2],
        left_near[1],
        left_near[0],
    ]
    right = [
        right_near[0],
        right_near[1],
        right_near[2],
        [
            (1.45, point[1], point[2] + 0.78 + 0.02 * index)
            for index, point in enumerate(boundary)
        ],
    ]
    boundary_weights = [1.0, math.sqrt(0.5), 1.35, 0.82, 1.0]
    left_weights = [boundary_weights.copy() for _ in range(4)]
    right_weights = [list(reversed(boundary_weights)) for _ in range(4)]
    left_surface = _surface(
        left,
        [0.0, 1.0],
        [4, 4],
        [0.0, 0.74, 2.0],
        [4, 1, 4],
        3,
        3,
        left_weights,
    )
    right_surface = _surface(
        [list(reversed(row)) for row in right],
        [0.0, 1.0],
        [4, 4],
        [10.0, 12.52, 14.0],
        [4, 1, 4],
        3,
        3,
        right_weights,
    )
    parameter_map = lambda value: 14.0 - 2.0 * value
    _assert_continuity(left_surface, right_surface, 2, parameter_map)
    _assert_discontinuity(left_surface, right_surface, 3, parameter_map)
    return (
        _sew([_full_face(left_surface), _full_face(right_surface)]),
        {
            "expected_continuity": "G2",
            "construction": "Rational bicubic/multi-span pair; C2 cross-seam derivatives match and third derivatives intentionally differ.",
            "full_rectangular_faces": True,
            "rational": True,
            "reversed_boundary_parameter": True,
            "left_v_domain": [0.0, 2.0],
            "right_v_domain": [10.0, 14.0],
            "boundary_parameter_map": "v_right = 14 - 2 * v_left",
            "u_degrees": [3, 3],
            "v_degrees": [3, 3],
        },
    )


def _repeated_knot_g2() -> tuple[object, dict[str, object]]:
    boundary = [(0.0, index / 8.0, 0.0) for index in range(9)]
    first = [(1.05, 0.0, 0.18)] * 9
    second = [(0.0, 0.0, -0.12)] * 9
    zero = [(0.0, 0.0, 0.0)] * 9
    left_near, right_near = _cross_layers(boundary, first, second, zero, 3)
    left = [
        [
            (-1.30, point[1], point[2] - 0.58 - 0.02 * index)
            for index, point in enumerate(boundary)
        ],
        left_near[2],
        left_near[1],
        left_near[0],
    ]
    right = [
        right_near[0],
        right_near[1],
        right_near[2],
        [
            (1.38, point[1], point[2] + 0.66 + 0.025 * index)
            for index, point in enumerate(boundary)
        ],
    ]
    v_knots = [0.0, 0.47, 1.0]
    v_multiplicities = [6, 3, 6]
    left_surface = _surface(
        left,
        [0.0, 1.0],
        [4, 4],
        v_knots,
        v_multiplicities,
        3,
        5,
    )
    right_surface = _surface(
        right,
        [0.0, 1.0],
        [4, 4],
        v_knots,
        v_multiplicities,
        3,
        5,
    )
    _assert_continuity(left_surface, right_surface, 2, lambda value: value)
    _assert_discontinuity(left_surface, right_surface, 3, lambda value: value)
    return (
        _sew([_full_face(left_surface), _full_face(right_surface)]),
        {
            "expected_continuity": "G2",
            "construction": (
                "Polynomial cubic-by-quintic pair with a multiplicity-three "
                "internal seam-direction knot; C2 cross-seam derivatives match "
                "and third derivatives intentionally differ."
            ),
            "full_rectangular_faces": True,
            "rational": False,
            "reversed_boundary_parameter": False,
            "repeated_knot_axis": "v",
            "v_knots": v_knots,
            "v_multiplicities": v_multiplicities,
            "internal_knot_continuity": "C2",
            "u_degrees": [3, 3],
            "v_degrees": [5, 5],
        },
    )


def _extreme_positive_weights_g2() -> tuple[object, dict[str, object]]:
    boundary = [
        (0.0, 0.0, 0.0),
        (0.0, 0.25, 0.0),
        (0.0, 0.50, 0.0),
        (0.0, 0.75, 0.0),
        (0.0, 1.0, 0.0),
    ]
    first = [(1.0, 0.0, 0.16)] * 5
    second = [(0.0, 0.0, -0.10)] * 5
    zero = [(0.0, 0.0, 0.0)] * 5
    left_near, right_near = _cross_layers(boundary, first, second, zero, 3)
    left = [
        [
            (-1.28, point[1], point[2] - 0.61 - 0.025 * index)
            for index, point in enumerate(boundary)
        ],
        left_near[2],
        left_near[1],
        left_near[0],
    ]
    right = [
        right_near[0],
        right_near[1],
        right_near[2],
        [
            (1.34, point[1], point[2] + 0.69 + 0.02 * index)
            for index, point in enumerate(boundary)
        ],
    ]
    boundary_weights = [1.0, 1.0, 1.0e-8, 1.0, 1.0]
    weights = [boundary_weights.copy() for _ in range(4)]
    left_surface = _surface(
        left,
        [0.0, 1.0],
        [4, 4],
        [0.0, 0.43, 1.0],
        [4, 1, 4],
        3,
        3,
        weights,
    )
    right_surface = _surface(
        right,
        [0.0, 1.0],
        [4, 4],
        [0.0, 0.43, 1.0],
        [4, 1, 4],
        3,
        3,
        weights,
    )
    _assert_continuity(left_surface, right_surface, 2, lambda value: value)
    _assert_discontinuity(left_surface, right_surface, 3, lambda value: value)
    return (
        _sew([_full_face(left_surface), _full_face(right_surface)]),
        {
            "expected_continuity": "G2",
            "construction": (
                "Rational bicubic/multi-span pair with strictly positive weights "
                "spanning eight orders of magnitude; C2 cross-seam derivatives "
                "match and third derivatives intentionally differ."
            ),
            "full_rectangular_faces": True,
            "rational": True,
            "reversed_boundary_parameter": False,
            "minimum_weight": min(boundary_weights),
            "maximum_weight": max(boundary_weights),
            "weight_ratio": max(boundary_weights) / min(boundary_weights),
            "u_degrees": [3, 3],
            "v_degrees": [3, 3],
        },
    )


def _quintic_g3() -> tuple[object, dict[str, object]]:
    boundary = [
        (0.0, 0.0, 0.02),
        (0.0, 0.20, 0.23),
        (0.0, 0.50, -0.11),
        (0.0, 0.79, 0.27),
        (0.0, 1.0, 0.01),
    ]
    first = [(1.0, 0.0, 0.18 + 0.03 * index) for index in range(5)]
    second = [(0.0, 0.0, -0.12 + 0.04 * index) for index in range(5)]
    third = [(0.0, 0.0, 0.20 - 0.03 * index) for index in range(5)]
    left_near, right_near = _cross_layers(boundary, first, second, third, 5)
    left = [
        [
            (-1.4, point[1], point[2] - 0.92 - 0.04 * index)
            for index, point in enumerate(boundary)
        ],
        [
            (-1.05, point[1], point[2] - 0.61 + 0.02 * index)
            for index, point in enumerate(boundary)
        ],
        left_near[3],
        left_near[2],
        left_near[1],
        left_near[0],
    ]
    right = [
        right_near[0],
        right_near[1],
        right_near[2],
        right_near[3],
        [
            (1.08, point[1], point[2] + 0.48 - 0.01 * index)
            for index, point in enumerate(boundary)
        ],
        [
            (1.5, point[1], point[2] + 1.02 + 0.05 * index)
            for index, point in enumerate(boundary)
        ],
    ]
    left_surface = _surface(
        left,
        [0.0, 1.0],
        [6, 6],
        [0.0, 0.41, 1.0],
        [4, 1, 4],
        5,
        3,
    )
    right_surface = _surface(
        right,
        [0.0, 1.0],
        [6, 6],
        [0.0, 0.41, 1.0],
        [4, 1, 4],
        5,
        3,
    )
    _assert_continuity(left_surface, right_surface, 3, lambda value: value)
    _assert_discontinuity(left_surface, right_surface, 4, lambda value: value)
    return (
        _sew([_full_face(left_surface), _full_face(right_surface)]),
        {
            "expected_continuity": "G3",
            "construction": "Polynomial quintic-by-cubic/multi-span pair; C3 cross-seam derivatives match and fourth derivatives intentionally differ.",
            "full_rectangular_faces": True,
            "rational": False,
            "reversed_boundary_parameter": False,
            "u_degrees": [5, 5],
            "v_degrees": [3, 3],
        },
    )


def _trimmed_negative() -> tuple[object, dict[str, object]]:
    poles = [
        [(0.0, 0.0, 0.0), (0.0, 1.0, 0.0)],
        [(1.0, 0.0, 0.0), (1.0, 1.0, 0.0)],
    ]
    surfaces = [
        _surface(poles, [0.0, 1.0], [2, 2], [0.0, 1.0], [2, 2], 1, 1)
        for _ in range(2)
    ]
    triangles = [
        [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)],
        [(0.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
    ]
    faces = []
    for surface, triangle in zip(surfaces, triangles, strict=True):
        edges = [
            BRepBuilderAPI_MakeEdge(
                GCE2d_MakeSegment(
                    gp_Pnt2d(*start), gp_Pnt2d(*triangle[(index + 1) % 3])
                ).Value(),
                surface,
            ).Edge()
            for index, start in enumerate(triangle)
        ]
        wire = BRepBuilderAPI_MakeWire(*edges).Wire()
        builder = BRepBuilderAPI_MakeFace(surface, wire, True)
        if not builder.IsDone():
            raise RuntimeError("OCCT could not construct an arbitrary trimmed face.")
        face = builder.Face()
        if not BRepLib.BuildCurves3d_s(face):
            raise RuntimeError("OCCT could not construct 3D trim curves.")
        faces.append(face)
    return (
        _sew(faces),
        {
            "expected_continuity": "unsupported",
            "construction": "Two planar NURBS faces trimmed into triangles with a shared diagonal non-isoparametric seam.",
            "full_rectangular_faces": False,
            "rational": False,
            "reversed_boundary_parameter": False,
            "expected_result": "Reject as an unsupported arbitrary-trimmed seam.",
            "u_degrees": [1, 1],
            "v_degrees": [1, 1],
        },
    )


def _assert_continuity(
    left: Geom_BSplineSurface,
    right: Geom_BSplineSurface,
    order: int,
    right_parameter: Callable[[float], float],
) -> None:
    _, left_u, left_v_min, left_v_max = left.Bounds()
    right_u, _, _, _ = right.Bounds()
    for sample in range(17):
        left_v = left_v_min + (left_v_max - left_v_min) * sample / 16.0
        right_v = right_parameter(left_v)
        distance = left.Value(left_u, left_v).Distance(
            right.Value(right_u, right_v)
        )
        if distance > 1.0e-8:
            raise RuntimeError(f"Fixture seam has a C0 residual of {distance}.")
        for derivative_order in range(1, order + 1):
            residual = left.DN(left_u, left_v, derivative_order, 0).Subtracted(
                right.DN(right_u, right_v, derivative_order, 0)
            ).Magnitude()
            if residual > 1.0e-7:
                raise RuntimeError(
                    f"Fixture seam derivative {derivative_order} has a residual of {residual}."
                )


def _assert_discontinuity(
    left: Geom_BSplineSurface,
    right: Geom_BSplineSurface,
    order: int,
    right_parameter: Callable[[float], float],
) -> None:
    _, left_u, left_v_min, left_v_max = left.Bounds()
    right_u, _, _, _ = right.Bounds()
    residuals = [
        left.DN(left_u, left_v, order, 0)
        .Subtracted(right.DN(right_u, right_parameter(left_v), order, 0))
        .Magnitude()
        for left_v in (
            left_v_min,
            (left_v_min + left_v_max) / 2.0,
            left_v_max,
        )
    ]
    if max(residuals) < 1.0e-4:
        raise RuntimeError(
            f"Fixture unexpectedly satisfies cross-seam derivative order {order}."
        )


def _normalize_step_header(path: Path) -> None:
    content = path.read_text(encoding="utf-8")
    content = re.sub(
        r"(FILE_NAME\('[^']*',')[^']*(')",
        rf"\g<1>{FIXED_STEP_TIMESTAMP}\g<2>",
        content,
        count=1,
    )
    path.write_text(content.replace("\r\n", "\n"), encoding="utf-8", newline="\n")


def _write_step(path: Path, shape: object) -> None:
    writer = STEPControl_Writer()
    if writer.Transfer(shape, STEPControl_AsIs) != IFSelect_RetDone:
        raise RuntimeError(f"OCCT could not transfer {path.name} to STEP.")
    if writer.Write(str(path)) != IFSelect_RetDone:
        raise RuntimeError(f"OCCT could not write {path.name}.")
    _normalize_step_header(path)


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> None:
    if OCP.__version__ != EXPECTED_OCP_VERSION:
        raise RuntimeError(
            f"Expected OCP {EXPECTED_OCP_VERSION}, found {OCP.__version__}. "
            "Run the pinned interpreter with `-E`."
        )
    OUTPUT.mkdir(parents=True, exist_ok=True)
    fixtures = {
        "polynomial-g1.step": _polynomial_g1,
        "rational-reversed-g2.step": _rational_reversed_g2,
        "quintic-g3.step": _quintic_g3,
        "arbitrary-trim-negative.step": _trimmed_negative,
        "repeated-knot-g2.step": _repeated_knot_g2,
        "extreme-positive-weights-g2.step": _extreme_positive_weights_g2,
    }
    entries = []
    for filename, factory in fixtures.items():
        shape, expected = factory()
        path = OUTPUT / filename
        _write_step(path, shape)
        entries.append(
            {
                "file": filename,
                "sha256": _sha256(path),
                "bytes": path.stat().st_size,
                **expected,
            }
        )
    manifest = {
        "schema_version": 2,
        "license": "Apache-2.0",
        "provenance": "Generated entirely by tools/phase5/generate_occt_fixtures.py; no third-party CAD model content.",
        "generator_command": "target/phase5-python/Scripts/python.exe -E tools/phase5/generate_occt_fixtures.py",
        "occt_version": "7.8.1",
        "ocp_wrapper_version": OCP.__version__,
        "fixed_step_timestamp": FIXED_STEP_TIMESTAMP,
        "fixtures": entries,
    }
    (OUTPUT / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(json.dumps(manifest, indent=2))


if __name__ == "__main__":
    main()
