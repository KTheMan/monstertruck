"""Inspect STEP evidence with a pinned independent OCCT runtime."""

from __future__ import annotations

import argparse
import glob
import hashlib
import json
from pathlib import Path
from typing import Sequence

import OCP
from OCP.BRep import BRep_Tool
from OCP.BRepBndLib import BRepBndLib
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.Bnd import Bnd_Box
from OCP.GeomAbs import GeomAbs_BSplineSurface
from OCP.GeomAdaptor import GeomAdaptor_Surface
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import (
    TopAbs_COMPOUND,
    TopAbs_COMPSOLID,
    TopAbs_EDGE,
    TopAbs_FACE,
    TopAbs_SHELL,
    TopAbs_SOLID,
    TopAbs_VERTEX,
    TopAbs_WIRE,
)
from OCP.TopExp import TopExp, TopExp_Explorer
from OCP.TopTools import TopTools_IndexedMapOfShape
from OCP.TopoDS import TopoDS

EXPECTED_OCP_VERSION = "7.8.1.1"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _count(shape: object, kind: object) -> int:
    shapes = TopTools_IndexedMapOfShape()
    TopExp.MapShapes_s(shape, kind, shapes)
    return shapes.Extent()


def _surface_records(shape: object) -> list[dict[str, object]]:
    explorer = TopExp_Explorer(shape, TopAbs_FACE)
    records = []
    while explorer.More():
        face = TopoDS.Face_s(explorer.Current())
        adaptor = GeomAdaptor_Surface(BRep_Tool.Surface_s(face))
        surface_type = adaptor.GetType()
        record: dict[str, object] = {
            "surface_type": str(surface_type).split(".")[-1],
            "u_domain": [adaptor.FirstUParameter(), adaptor.LastUParameter()],
            "v_domain": [adaptor.FirstVParameter(), adaptor.LastVParameter()],
        }
        if surface_type == GeomAbs_BSplineSurface:
            surface = adaptor.BSpline()
            weights = [
                surface.Weight(u_index, v_index)
                for u_index in range(1, surface.NbUPoles() + 1)
                for v_index in range(1, surface.NbVPoles() + 1)
            ]
            minimum_weight = min(weights)
            maximum_weight = max(weights)
            record.update(
                {
                    "u_degree": surface.UDegree(),
                    "v_degree": surface.VDegree(),
                    "u_poles": surface.NbUPoles(),
                    "v_poles": surface.NbVPoles(),
                    "u_knots": surface.NbUKnots(),
                    "v_knots": surface.NbVKnots(),
                    "u_knot_values": [
                        surface.UKnot(index)
                        for index in range(1, surface.NbUKnots() + 1)
                    ],
                    "v_knot_values": [
                        surface.VKnot(index)
                        for index in range(1, surface.NbVKnots() + 1)
                    ],
                    "u_multiplicities": [
                        surface.UMultiplicity(index)
                        for index in range(1, surface.NbUKnots() + 1)
                    ],
                    "v_multiplicities": [
                        surface.VMultiplicity(index)
                        for index in range(1, surface.NbVKnots() + 1)
                    ],
                    "u_rational": surface.IsURational(),
                    "v_rational": surface.IsVRational(),
                    "minimum_weight": minimum_weight,
                    "maximum_weight": maximum_weight,
                    "weight_ratio": (
                        maximum_weight / minimum_weight
                        if minimum_weight > 0.0
                        else None
                    ),
                }
            )
        records.append(record)
        explorer.Next()
    return records


def _inspect(path: Path) -> dict[str, object]:
    reader = STEPControl_Reader()
    if reader.ReadFile(str(path)) != IFSelect_RetDone:
        raise RuntimeError(f"OCCT could not read {path}.")
    transferred_roots = reader.TransferRoots()
    if transferred_roots < 1:
        raise RuntimeError(f"OCCT could not transfer a root from {path}.")
    shape = reader.OneShape()
    box = Bnd_Box()
    BRepBndLib.Add_s(shape, box)
    bounds = box.Get()
    model = reader.WS().Model()
    valid_brep = BRepCheck_Analyzer(shape).IsValid()
    if not valid_brep:
        raise RuntimeError(f"OCCT reports an invalid B-rep in {path}.")
    return {
        "file": path.as_posix(),
        "sha256": _sha256(path),
        "bytes": path.stat().st_size,
        "occt_model_entities": model.NbEntities(),
        "transferred_roots": transferred_roots,
        "shape_type": str(shape.ShapeType()).split(".")[-1],
        "valid_brep": valid_brep,
        "unique_topology": {
            "compounds": _count(shape, TopAbs_COMPOUND),
            "compsolids": _count(shape, TopAbs_COMPSOLID),
            "solids": _count(shape, TopAbs_SOLID),
            "shells": _count(shape, TopAbs_SHELL),
            "faces": _count(shape, TopAbs_FACE),
            "wires": _count(shape, TopAbs_WIRE),
            "edges": _count(shape, TopAbs_EDGE),
            "vertices": _count(shape, TopAbs_VERTEX),
        },
        "bounding_box": {
            "min": list(bounds[:3]),
            "max": list(bounds[3:]),
        },
        "surfaces": _surface_records(shape),
    }


def _arguments(arguments: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate STEP files with pinned OCCT and emit a JSON receipt."
    )
    parser.add_argument("step_files", nargs="+", type=Path)
    parser.add_argument("--output", type=Path)
    return parser.parse_args(arguments)


def _expand_paths(paths: Sequence[Path]) -> list[Path]:
    return [
        candidate
        for path in paths
        for candidate in (
            sorted(Path(candidate) for candidate in glob.glob(str(path)))
            if glob.has_magic(str(path))
            else [path]
        )
    ]


def main(arguments: Sequence[str] | None = None) -> None:
    options = _arguments(arguments)
    if OCP.__version__ != EXPECTED_OCP_VERSION:
        raise RuntimeError(
            f"Expected OCP {EXPECTED_OCP_VERSION}, found {OCP.__version__}. "
            "Run the pinned interpreter with `-E`."
        )
    report = {
        "schema_version": 2,
        "validator": "tools/phase5/validate_step_occt.py",
        "validator_command": "target/phase5-python/Scripts/python.exe -E tools/phase5/validate_step_occt.py <step-files>",
        "occt_version": "7.8.1",
        "ocp_wrapper_version": OCP.__version__,
        "files": [_inspect(path) for path in _expand_paths(options.step_files)],
    }
    rendered = json.dumps(report, indent=2) + "\n"
    if options.output is not None:
        options.output.parent.mkdir(parents=True, exist_ok=True)
        options.output.write_text(rendered, encoding="utf-8", newline="\n")
    print(rendered, end="")


if __name__ == "__main__":
    main()
