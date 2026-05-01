"""Python bindings for the `ext` Rust crate.

The native module is `sseq_ext._sseq_ext`. We re-export everything at the top
level so users can write::

    import sseq_ext as ext
    res = ext.construct("S_2", save_dir=None)
    res.compute_through_stem(30, 7)
    print(res.graded_dimension_string())
"""

from ._sseq_ext import (  # noqa: F401
    init_logging,
    MilnorAlgebra,
    Bidegree,
    BidegreeGenerator,
    BidegreeElement,
    ValidPrime,
    FpVector,
    Matrix,
    MatrixView,
    MatrixViewMut,
    AugmentedMatrix,
    AugmentedMatrixView,
    AugmentedMatrixViewMut,
    Subspace,
    construct,
    get_unit,
    secondary_job,
    Resolution,
    FreeModuleHomomorphism,
    ResolutionHomomorphism,
    ChainHomotopy,
    SecondaryResolution,
    SecondaryHomotopy,
    Sseq,
    Product,
    write_sseq_svg,
)

__all__ = [
    "init_logging",
    "MilnorAlgebra",
    "Bidegree",
    "BidegreeGenerator",
    "BidegreeElement",
    "ValidPrime",
    "FpVector",
    "Matrix",
    "MatrixView",
    "MatrixViewMut",
    "AugmentedMatrix",
    "AugmentedMatrixView",
    "AugmentedMatrixViewMut",
    "Subspace",
    "construct",
    "get_unit",
    "secondary_job",
    "Resolution",
    "FreeModuleHomomorphism",
    "ResolutionHomomorphism",
    "ChainHomotopy",
    "SecondaryResolution",
    "SecondaryHomotopy",
    "Sseq",
    "Product",
    "write_sseq_svg",
]
