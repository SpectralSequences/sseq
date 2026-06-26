#!/usr/bin/env python3
"""Convert between our basis and Bruner's basis (sphere only, mostly hardcoded).

Python port of ext/examples/bruner.rs.
"""

import os
import sys

import _query as query
from ext import Resolution, algebra, fp, sseq
from ext.algebra import MilnorAlgebra, MilnorBasisElement, SteenrodAlgebra
# NOTE: depends on FreeModule, FreeModuleHomomorphism (algebra, API_PROPOSAL §5.3/§5.4),
# FiniteChainComplex (ext top level, API_PROPOSAL §7.1), ResolutionHomomorphism
# (ext top level, API_PROPOSAL §7.3), construct (ext, API_PROPOSAL §7.7).
from ext import FiniteChainComplex, ResolutionHomomorphism
# NOTE: depends on Matrix.from_vec (fp, API_PROPOSAL §4.4) and the TWO constant
# (fp, API_PROPOSAL §4.1).
from ext.fp import FpVector, Matrix

TWO = fp.TWO


def read_line(data):
    """Read the first non-empty line of ``data``; return it without the newline.

    Returns ``None`` at end of file, mirroring the Rust ``read_line`` helper.
    """
    buf = ""
    while buf == "":
        line = data.readline()
        if line == "":
            return None
        # Remove newline character (Rust pops the trailing byte unconditionally).
        buf = line[:-1]
    return buf


def entry(x):
    """Take the first whitespace-delimited item of ``x``; return ``(rest, item)``."""
    x = x.strip()
    k = x.find(" ")
    if k == -1:
        return ("", x)
    return (x[k:], x[:k])


def get_algebra_element(a, input_str):
    """Parse an algebra element ``$op_deg _ $op``; yield indices whose sum is it."""
    input_str, t = entry(input_str)
    t = int(t)
    input_str, _ = entry(input_str)

    input_str = input_str.strip()
    assert input_str[0:1] == "i"

    # Remove the i
    input_str = input_str[1:]
    # Remove the trailing ).
    input_str = input_str[: len(input_str) - 2]

    for piece in input_str.split(")"):
        piece = piece[1:]
        elt = MilnorBasisElement(
            [int(x) for x in piece.split(",")],
            0,
            t,
        )
        yield a.basis_element_to_index(elt)


def get_element(a, m, input_data):
    """Read a generator block; return ``(degree, FpVector)`` or ``None`` at EOF."""
    buf = read_line(input_data)
    if buf is None:
        return None
    degree = int(buf.strip())
    a.compute_basis(degree)
    m.compute_basis(degree)

    buf = read_line(input_data)
    num_lines = int(buf.strip())

    result = FpVector(TWO, m.dimension(degree))

    for _ in range(num_lines):
        buf = read_line(input_data)
        rem, gen_idx = entry(buf)
        gen_idx = int(gen_idx)
        offset = m.internal_generator_offset(degree, gen_idx)
        for op in get_algebra_element(a, rem[1:]):
            result.add_basis_element(offset + op, 1)
    return (degree, result)


def create_chain_complex(num_s):
    """Create a ``FiniteChainComplex`` with ``num_s`` non-zero modules."""
    alg = SteenrodAlgebra.milnor(int(TWO), False)

    modules = []
    differentials = []
    for _ in range(num_s):
        modules.append(algebra.FreeModule(alg, "", 0))
    for s in range(1, num_s):
        differentials.append(
            algebra.FreeModuleHomomorphism(modules[s], modules[s - 1], 0)
        )
    return FiniteChainComplex(modules, differentials)


def read_bruner_resolution(data_dir, max_n):
    """Read the ``hDiff.N`` files in ``data_dir`` into a chain complex.

    Returns ``(max_s, FiniteChainComplex)``.
    """
    num_s = len(os.listdir(data_dir))

    cc = create_chain_complex(num_s)
    # Rust does `cc.algebra().as_ref().try_into()` to view the chain complex's
    # SteenrodAlgebra as a &MilnorAlgebra. No SteenrodAlgebra -> MilnorAlgebra
    # conversion is bound, so we rebuild the same Milnor algebra here.
    # NOTE: depends on SteenrodAlgebra -> MilnorAlgebra conversion (API_PROPOSAL §5.2);
    # using a fresh MilnorAlgebra(2, False) as a faithful stand-in.
    alg = MilnorAlgebra(int(TWO), False)

    s = num_s - 1

    alg.compute_basis(max_n + s + 1)
    # Handle s = 0
    # TODO: actually parse file
    m = cc.module(0)
    m.add_generators(0, 1, None)
    m.extend_by_zero(max_n + 1)

    for s in range(1, num_s):
        m = cc.module(s)
        d = cc.differential(s)

        with open(os.path.join(data_dir, f"hDiff.{s}")) as f:
            read_line(f)

            entries = []
            cur_degree = 0

            while True:
                element = get_element(alg, cc.module(s - 1), f)
                if element is None:
                    break
                t, g = element
                if t == cur_degree:
                    entries.append(g)
                else:
                    m.add_generators(cur_degree, len(entries), None)
                    d.add_generators_from_rows(cur_degree, entries)

                    m.extend_by_zero(t - 1)
                    d.extend_by_zero(t - 1)

                    entries = [g]
                    cur_degree = t

            m.add_generators(cur_degree, len(entries), None)
            d.add_generators_from_rows(cur_degree, entries)

            m.extend_by_zero(max_n + s + 1)
            d.extend_by_zero(max_n + s)

    return (s, cc)


def main():
    data_dir = os.path.join(os.path.dirname(__file__), "bruner_data")
    max_n = query.with_default("Max n", "20", int)

    # Read in Bruner's resolution
    max_s, cc = read_bruner_resolution(data_dir, max_n)
    max = sseq.Bidegree.n_s(max_n, max_s)

    save_dir = query.optional("Save directory", str)

    resolution = Resolution.construct("S_2@milnor", save_dir)

    resolution.compute_through_stem(max)

    # Create a ResolutionHomomorphism object
    # NOTE: Bidegree::zero() is not bound (API_PROPOSAL §6.1 lists only s_t/n_s/x_y);
    # Bidegree.n_s(0, 0) is the faithful equivalent.
    hom = ResolutionHomomorphism("", cc, resolution, sseq.Bidegree.n_s(0, 0))

    # We have to explicitly tell it what to do at (0, 0)
    hom.extend_step(sseq.Bidegree.n_s(0, 0), Matrix.from_vec(TWO, [[1]]))
    hom.extend_all()

    # Now print the results
    print("sseq_basis | bruner_basis")
    for b in hom.target().iter_stem():
        matrix = hom.get_map(b.s).hom_k(b.t)

        for i, row in enumerate(matrix):
            g = sseq.BidegreeGenerator(b, i)
            print(f"x_{g:#} = {list(row)}")


if __name__ == "__main__":
    main()
