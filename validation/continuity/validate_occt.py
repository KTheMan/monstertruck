"""Validate the repaired STEP artifact with the pinned OCCT Python binding."""

from argparse import ArgumentParser
from pathlib import Path

import OCP
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_FACE
from OCP.TopExp import TopExp_Explorer

EXPECTED_OCP_VERSION = "7.9.3.1"


def face_count(shape: object) -> int:
    """Count topological faces in an OCCT shape."""
    explorer = TopExp_Explorer(shape, TopAbs_FACE)
    count = 0
    while explorer.More():
        count += 1
        explorer.Next()
    return count


def main() -> None:
    """Read one STEP file and require successful transfer and shape validity."""
    parser = ArgumentParser()
    parser.add_argument("step", type=Path)
    args = parser.parse_args()

    if OCP.__version__ != EXPECTED_OCP_VERSION:
        raise RuntimeError(
            f"expected OCCT binding {EXPECTED_OCP_VERSION}, found {OCP.__version__}"
        )

    reader = STEPControl_Reader()
    status = reader.ReadFile(str(args.step))
    if status != IFSelect_RetDone:
        raise RuntimeError(f"OCCT rejected {args.step}: {status}")
    if reader.NbRootsForTransfer() < 1 or reader.TransferRoots() < 1:
        raise RuntimeError(f"OCCT transferred no roots from {args.step}")

    shape = reader.OneShape()
    if shape.IsNull():
        raise RuntimeError(f"OCCT produced a null shape from {args.step}")
    if not BRepCheck_Analyzer(shape, True).IsValid():
        raise RuntimeError(f"OCCT reports an invalid shape for {args.step}")

    faces = face_count(shape)
    if faces != 2:
        raise RuntimeError(f"expected 2 OCCT faces, found {faces}")
    print(f"occt_version={OCP.__version__}")
    print(f"occt_valid_faces={faces}")


if __name__ == "__main__":
    main()
