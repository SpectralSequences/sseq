//! Nassau's untwisted tensor resolution: $\Ext_A(M, k)$ from a resolution of $k$.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use algebra::{
    Algebra, Bialgebra,
    module::{
        FreeModule, Module, ZeroModule,
        homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism},
    },
    pair_algebra::PairAlgebra,
};
use dashmap::DashMap;
use fp::{
    matrix::Matrix,
    prime::{Prime, ValidPrime},
    vector::FpVector,
};
use once::{OnceBiVec, OnceVec};
use sseq::coordinates::{Bidegree, BidegreeElement};

use super::{Cochain, CochainCup, ExtAlgebra, ExtDifferential, ExtModule};
use crate::{
    chain_complex::{ChainComplex, FreeChainComplex},
    resolution::secondary::SecondaryResolution,
    save::{SaveDirectory, SaveKind},
    secondary::SecondaryLift,
};

/// The Hopf antipode $\chi\colon A \to A$ of a connected graded bialgebra, computed generically and
/// memoised.
///
/// For a connected graded Hopf algebra the antipode is determined by
/// $\sum_{(x)} \chi(x_{(1)}) x_{(2)} = \varepsilon(x)$, which at $p = 2$ unrolls to the recursion
/// $$ \chi(x) = x + \sum_{\text{mid}} \chi(x_{(1)})\, x_{(2)}, $$
/// the sum over coproduct terms with $0 < |x_{(1)}| < |x|$. We only ever need $\chi$ on the
/// [`Bialgebra::decompose`] atoms (via the anti-homomorphism property $\chi(ab) = \chi(b)\chi(a)$),
/// whose coproducts are directly available.
///
/// Implemented only at $p = 2$, where all Hopf signs are $+$.
pub struct Antipode<A: Bialgebra> {
    algebra: Arc<A>,
    /// `(degree, basis index)` → $\chi$ of that basis element, as a vector over the algebra basis.
    cache: DashMap<(i32, usize), FpVector>,
}

impl<A: Bialgebra> Antipode<A> {
    /// The antipode of `algebra`.
    ///
    /// # Panics
    /// If `algebra` is not at `p = 2`. Every entry point into the untwisted tensor resolution
    /// builds an `Antipode`, so they all inherit this restriction.
    pub fn new(algebra: Arc<A>) -> Self {
        assert_eq!(
            algebra.prime(),
            ValidPrime::new(2),
            "Antipode is currently implemented only at p = 2"
        );
        Self {
            algebra,
            cache: DashMap::new(),
        }
    }

    /// The prime of the algebra.
    fn prime(&self) -> ValidPrime {
        self.algebra.prime()
    }

    /// $\chi$ of the basis element `(degree, idx)`, as a vector over the degree-`degree` algebra
    /// basis. Requires the algebra basis computed through `degree`.
    pub fn apply(&self, degree: i32, idx: usize) -> FpVector {
        if degree == 0 {
            let mut v = FpVector::new(self.prime(), self.algebra.dimension(0));
            v.add_basis_element(idx, 1);
            return v;
        }
        if let Some(v) = self.cache.get(&(degree, idx)) {
            return v.clone();
        }

        // χ(x) = χ(atom_1) · χ(atom_2) · … in acting order, since χ is an anti-homomorphism and
        // `decompose` lists the factors in the order they act (rightmost algebra factor first).
        let atoms = self.algebra.decompose(degree, idx);
        let result = if atoms.len() == 1 {
            // `x` is itself an atom (a single generator whose product is `x`); use the recursion.
            self.apply_atom(degree, idx)
        } else {
            let mut acc = FpVector::new(self.prime(), self.algebra.dimension(0));
            acc.add_basis_element(0, 1);
            let mut acc_deg = 0;
            for &(a_deg, a_idx) in &atoms {
                let chi_atom = self.apply(a_deg, a_idx);
                let out_deg = acc_deg + a_deg;
                let mut next = FpVector::new(self.prime(), self.algebra.dimension(out_deg));
                for (l_idx, l_coeff) in acc.iter_nonzero() {
                    for (r_idx, r_coeff) in chi_atom.iter_nonzero() {
                        self.algebra.multiply_basis_elements(
                            next.as_slice_mut(),
                            l_coeff * r_coeff,
                            acc_deg,
                            l_idx,
                            a_deg,
                            r_idx,
                        );
                    }
                }
                acc = next;
                acc_deg = out_deg;
            }
            acc
        };

        self.cache
            .entry((degree, idx))
            .or_insert(result)
            .value()
            .clone()
    }

    /// $\chi$ of a single [`decompose`](Bialgebra::decompose) atom, via the recursion
    /// $\chi(g) = g + \sum_{\text{mid}} \chi(g_{(1)}) g_{(2)}$ over its coproduct.
    fn apply_atom(&self, degree: i32, idx: usize) -> FpVector {
        let mut result = FpVector::new(self.prime(), self.algebra.dimension(degree));
        result.add_basis_element(idx, 1);
        for (l_deg, l_idx, r_deg, r_idx) in self.algebra.coproduct(degree, idx) {
            // Skip the two boundary terms g ⊗ 1 (l_deg == degree) and 1 ⊗ g (l_deg == 0).
            if l_deg == 0 || l_deg == degree {
                continue;
            }
            let chi_left = self.apply(l_deg, l_idx);
            for (j, coeff) in chi_left.iter_nonzero() {
                self.algebra.multiply_basis_elements(
                    result.as_slice_mut(),
                    coeff,
                    l_deg,
                    j,
                    r_deg,
                    r_idx,
                );
            }
        }
        result
    }

    /// $\chi$ of a general algebra element `elt` of degree `degree` (linear extension).
    fn apply_element(&self, degree: i32, elt: FpVector) -> FpVector {
        let mut out = FpVector::new(self.prime(), self.algebra.dimension(degree));
        for (idx, coeff) in elt.iter_nonzero() {
            out.as_slice_mut()
                .add(self.apply(degree, idx).as_slice(), coeff);
        }
        out
    }
}

/// The full coproduct $\Delta(x) = \sum_{(x)} x_{(1)} \otimes x_{(2)}$ of a basis element,
/// memoised.
///
/// At $p = 2$ every term has coefficient $1$, so a coproduct is a *set* of basis-element pairs
/// $(|x_{(1)}|, x_{(1)}, |x_{(2)}|, x_{(2)})$. It is obtained by folding the coproducts of the
/// [`decompose`](Bialgebra::decompose) atoms in $A \otimes A$ — the same route the [`Antipode`]
/// uses, but keeping *all* terms (including the two boundary terms $x \otimes 1$ and $1 \otimes
/// x$).
pub(crate) struct FullCoproduct<A: Bialgebra> {
    algebra: Arc<A>,
    cache: DashMap<(i32, usize), Arc<Vec<(i32, usize, i32, usize)>>>,
}

impl<A: Bialgebra> FullCoproduct<A> {
    /// The full coproduct of `algebra`.
    pub(crate) fn new(algebra: Arc<A>) -> Self {
        Self {
            algebra,
            cache: DashMap::new(),
        }
    }

    /// $\Delta$ of the basis element `(degree, idx)`, as the list of surviving pairs
    /// `(|a'|, a', |a''|, a'')`. Requires the algebra basis computed through `degree`.
    pub(crate) fn terms(&self, degree: i32, idx: usize) -> Arc<Vec<(i32, usize, i32, usize)>> {
        if degree == 0 {
            // Δ(1) = 1 ⊗ 1 (the unit is grouplike).
            return Arc::new(vec![(0, 0, 0, 0)]);
        }
        if let Some(v) = self.cache.get(&(degree, idx)) {
            return Arc::clone(&v);
        }

        let p = self.algebra.prime();
        // Δ as a map (l_deg, l_idx, r_deg, r_idx) → coeff, seeded with the unit 1 ⊗ 1.
        let mut terms: HashMap<(i32, usize, i32, usize), u32> = HashMap::new();
        terms.insert((0, 0, 0, 0), 1);
        // `decompose` lists the factors in the order they act, so each later factor multiplies on
        // the left.
        for (a_deg, a_idx) in self.algebra.decompose(degree, idx) {
            let mut next: HashMap<(i32, usize, i32, usize), u32> = HashMap::new();
            for (&(ll, li, rl, ri), &c) in &terms {
                for (cl, cli, cr, cri) in self.algebra.coproduct(a_deg, a_idx) {
                    // (cl ⊗ cr) · (ll ⊗ rl) = (cl · ll) ⊗ (cr · rl).
                    let mut left = FpVector::new(p, self.algebra.dimension(ll + cl));
                    self.algebra
                        .multiply_basis_elements(left.as_slice_mut(), 1, cl, cli, ll, li);
                    let mut right = FpVector::new(p, self.algebra.dimension(rl + cr));
                    self.algebra
                        .multiply_basis_elements(right.as_slice_mut(), 1, cr, cri, rl, ri);
                    for (lj, lc) in left.iter_nonzero() {
                        for (rj, rc) in right.iter_nonzero() {
                            *next.entry((ll + cl, lj, rl + cr, rj)).or_insert(0) += c * lc * rc;
                        }
                    }
                }
            }
            terms = next;
        }
        let result: Vec<(i32, usize, i32, usize)> = terms
            .into_iter()
            .filter(|&(_, c)| c % p.as_u32() != 0)
            .map(|(k, _)| k)
            .collect();
        Arc::clone(
            self.cache
                .entry((degree, idx))
                .or_insert_with(|| Arc::new(result))
                .value(),
        )
    }
}

/// The coboundary $\delta_Q$ of $\Hom_A(P_\bullet \otimes M, k)$, the dual of the untwisted tensor
/// resolution of `M`, as an [`ExtDifferential`] over the resolution $P_\bullet \to k$.
///
/// Nassau's algorithm resolves only finite modules. For an infinite `M`, resolve `k` instead: since
/// the Steenrod algebra `A` is a Hopf algebra, each $P_s \otimes M$ is free, so
/// $P_\bullet \otimes M \to M$ is a free resolution of `M`, in general non-minimal, and
/// $\Ext_A(M, k)$ is the cohomology of $\Hom_A(P_\bullet \otimes M, k)$.
/// [`tensor_resolution_ext`] attaches this to an [`ExtModule`], which takes that cohomology.
///
/// # The coboundary, in closed form
/// The tensor module is never built. Writing $Q(D_s) = (P_s \otimes M)/(\bar A \cdot (P_s\otimes
/// M))$ for the indecomposables (the cochain generators of $\Hom_A(P_s\otimes M, k)$), the Hopf
/// "tensor-with-free is free" isomorphism identifies $Q(D_s) \cong \bigoplus_i \Sigma^{d_i} M$,
/// where the $x_i$ are the generators of the free module $P_s$ in internal degrees $d_i$. So a
/// cochain generator at internal degree `t` is a pair $(i, \alpha)$ — a $P_s$-generator $x_i$ and
/// an `M`-basis element $m_\alpha$ with $d_i + |m_\alpha| = t$ — and $$ \dim \Hom_A(P_s\otimes
/// M, k)_t = \sum_i \dim M_{t - d_i}. $$ The coboundary $\delta\colon C^s \to C^{s+1}$, dual to
/// $Q(\partial)$, has the closed form $$ \delta_{(i,\alpha),(l,\gamma)} =
/// [m_\alpha]\bigl(\chi(a_{li}) \cdot m_\gamma\bigr), $$ where $a_{li} \in A$ is the component of
/// the $k$-resolution differential $d_P(z_l)$ on the generator $x_i$ (an algebra element of degree
/// $|z_l| - d_i$), $\chi$ is the [antipode](Antipode), and the action is on `M`. For a minimal
/// $P_\bullet$ every $a_{li}$ lies in $\bar A$, so $\delta$ vanishes when $\bar A$ acts trivially
/// on `M`, as for `M = k`. Otherwise it need not, and the cohomology must be taken.
pub struct TensorResolutionDifferential<CC, N>
where
    CC: FreeChainComplex,
    CC::Algebra: Bialgebra,
    N: Module<Algebra = CC::Algebra>,
{
    /// $P_\bullet$: a (minimal) free resolution of `k`.
    resolution: Arc<CC>,
    /// The module `M` being resolved.
    module: Arc<N>,
    antipode: Antipode<CC::Algebra>,
    /// `(s, t)` → the ordered cochain basis of $\Hom_A(P_s \otimes M, k)_t$: pairs
    /// `(generator degree d_i, generator index i, M-basis index α)`. Cached so that `matrix(b)`'s
    /// rows and `matrix(source)`'s columns share one coordinate system.
    cochain_bases: DashMap<(i32, i32), Arc<Vec<(i32, usize, usize)>>>,
    /// The coboundary matrix `δ_Q` out of each bidegree, memoised because every product and Massey
    /// bracket reads it.
    matrix_cache: DashMap<Bidegree, Arc<Matrix>>,
    /// [`module_fingerprint`](Self::module_fingerprint) at each `t`, memoised.
    fingerprints: DashMap<i32, u64>,
    /// How many matrices were read from disk rather than computed.
    #[cfg(test)]
    disk_reads: std::sync::atomic::AtomicUsize,
    /// The `tensor/{M}` subgroup of the $k$-resolution's save directory, which holds the `δ_Q`
    /// matrices in the format of `ext/SAVE-FORMAT.md`.
    ///
    /// The parent store pins the algebra and the resolution of $k$; the group name and the
    /// [fingerprint](Self::module_fingerprint) in each payload pin `M`.
    save_dir: SaveDirectory,
}

impl<CC, N> TensorResolutionDifferential<CC, N>
where
    CC: FreeChainComplex,
    CC::Algebra: Bialgebra,
    N: Module<Algebra = CC::Algebra>,
{
    /// The coboundary of $\Hom_A(P_\bullet \otimes M, k)$, for `resolution` = $P_\bullet$ and
    /// `module` = `M`.
    ///
    /// `M` must be fully defined in every degree it is read in: its basis there is read once and
    /// cached, along with the matrices built from it.
    ///
    /// # Panics
    ///
    /// If `M` has elements in negative degrees. Suspend it first: $\Ext(\Sigma^k M, k)$ is
    /// $\Ext(M, k)$ shifted by `k`.
    pub fn new(resolution: Arc<CC>, module: Arc<N>) -> Self {
        assert_nonnegative(&*module);
        let antipode = Antipode::new(resolution.algebra());
        let save_dir = match resolution.save_dir().store() {
            Some(parent) => SaveDirectory::Store(
                parent
                    .subgroup(&format!("tensor/{}", save_group_name(&module.to_string())))
                    .expect("Failed to create tensor subgroup"),
            ),
            None => SaveDirectory::None,
        };
        Self {
            resolution,
            module,
            antipode,
            cochain_bases: DashMap::new(),
            matrix_cache: DashMap::new(),
            fingerprints: DashMap::new(),
            #[cfg(test)]
            disk_reads: std::sync::atomic::AtomicUsize::new(0),
            save_dir,
        }
    }

    /// The prime of the $k$-resolution.
    fn prime(&self) -> ValidPrime {
        self.resolution.prime()
    }

    /// A fingerprint of `M` through internal degree `t`, stored in every `δ_Q` payload.
    ///
    /// Two modules with the same name share a save group, and their matrices can agree in shape at
    /// every bidegree read; the fingerprint is what rejects the other module's data. It covers the
    /// action of every algebra basis element, and is chained degree by degree so each degree is
    /// hashed once; `ext/SAVE-FORMAT.md` gives the encoding.
    fn module_fingerprint(&self, t: i32) -> u64 {
        let min = self.module.min_degree();
        if t < min {
            let mut hash = FNV_OFFSET;
            fnv1a(&mut hash, self.module.to_string().as_bytes());
            fnv1a(&mut hash, &min.to_le_bytes());
            return hash;
        }
        if let Some(hash) = self.fingerprints.get(&t) {
            return *hash;
        }
        let mut hash = FNV_OFFSET;
        fnv1a(&mut hash, &self.module_fingerprint(t - 1).to_le_bytes());

        let algebra = self.resolution.algebra();
        algebra.compute_basis(t - min);
        self.module.compute_basis(t);
        let dim = self.module.dimension(t);
        fnv1a(&mut hash, &(dim as u64).to_le_bytes());
        // Every action landing in degree `t`.
        for d in min..t {
            let op_deg = t - d;
            for op_idx in 0..algebra.dimension(op_deg) {
                for j in 0..self.module.dimension(d) {
                    let mut out = FpVector::new(self.prime(), dim);
                    self.module
                        .act_on_basis(out.as_slice_mut(), 1, op_deg, op_idx, d, j);
                    for (k, c) in out.iter_nonzero() {
                        for word in [d, op_idx as i32, j as i32, k as i32, c as i32] {
                            fnv1a(&mut hash, &word.to_le_bytes());
                        }
                    }
                }
            }
        }
        *self.fingerprints.entry(t).or_insert(hash)
    }

    /// The saved `δ_Q` matrix out of `b`, if there is one.
    ///
    /// Panics if the payload was written for another module or has another shape.
    fn read_delta(&self, b: Bidegree, rows: usize, cols: usize) -> Option<Matrix> {
        use byteorder::{LittleEndian, ReadBytesExt};

        let store = self.save_dir.store()?;
        let data = store
            .read(SaveKind::TensorDifferential, b)
            .expect("Failed to read tensor differential")?;
        let mut f = data.as_slice();
        let saved_fingerprint = f.read_u64::<LittleEndian>().unwrap();
        let fingerprint = self.module_fingerprint(b.t());
        assert_eq!(
            saved_fingerprint, fingerprint,
            "tensor differential at {b} was written for a different module (fingerprint \
             {saved_fingerprint:#018x}, expected {fingerprint:#018x})"
        );
        let saved_rows = f.read_u64::<LittleEndian>().unwrap() as usize;
        let saved_cols = f.read_u64::<LittleEndian>().unwrap() as usize;
        assert_eq!(
            (saved_rows, saved_cols),
            (rows, cols),
            "tensor differential at {b} has shape {saved_rows}×{saved_cols}, expected \
             {rows}×{cols}"
        );
        Some(Matrix::from_bytes(self.prime(), rows, cols, &mut f).unwrap())
    }

    /// Save the `δ_Q` matrix out of `b`, if there is a save directory.
    fn write_delta(&self, b: Bidegree, matrix: &Matrix) {
        use byteorder::{LittleEndian, WriteBytesExt};

        let Some(store) = self.save_dir.store() else {
            return;
        };
        let mut data = Vec::new();
        data.write_u64::<LittleEndian>(self.module_fingerprint(b.t()))
            .unwrap();
        data.write_u64::<LittleEndian>(matrix.rows() as u64)
            .unwrap();
        data.write_u64::<LittleEndian>(matrix.columns() as u64)
            .unwrap();
        matrix.to_bytes(&mut data).unwrap();
        store
            .write(SaveKind::TensorDifferential, b, &data)
            .expect("Failed to write tensor differential");
    }

    /// True once $P_s$ is resolved through internal degree `t` (so its generators up to `t` and the
    /// differential out of it are available).
    fn computed(&self, s: i32, t: i32) -> bool {
        s >= 0 && self.resolution.has_computed_bidegree(Bidegree::s_t(s, t))
    }

    /// The ordered cochain basis of $\Hom_A(P_s \otimes M, k)_t$ (see [`Self::cochain_bases`]).
    fn cochain_basis(&self, s: i32, t: i32) -> Arc<Vec<(i32, usize, usize)>> {
        if let Some(b) = self.cochain_bases.get(&(s, t)) {
            return Arc::clone(&b);
        }
        let module_s = self.resolution.module(s);
        let mut basis = Vec::new();
        // Generators of P_s in increasing degree; then M-basis in the complementary degree.
        for (d_i, i) in module_s.iter_gens(t) {
            let m_dim = self.module.dimension(t - d_i);
            for alpha in 0..m_dim {
                basis.push((d_i, i, alpha));
            }
        }
        Arc::clone(
            self.cochain_bases
                .entry((s, t))
                .or_insert_with(|| Arc::new(basis))
                .value(),
        )
    }

    /// The matrix of the cochain map $\Hom_A(Q_{\mathrm{tgt}}, k) \to \Hom_A(Q_{\mathrm{src}}, k)$
    /// induced by a map of $P_\bullet$, in the closed form of [`TensorResolutionDifferential`].
    ///
    /// Rows index the cochain basis $(d_i, i, \alpha)$ at $(\text{src\_s}, \text{src\_t})$, columns
    /// the cochain basis $(e_l, l, \gamma)$ at $(\text{tgt\_s}, \text{tgt\_t})$.
    /// `get_image(e_l, l)` is the image of the generator $z_l$ in $P_{\text{src\_s}}$, at internal
    /// degree $e_l - (\text{tgt\_t} - \text{src\_t})$; its component on $x_i$ is the $a_{li}$ of
    /// the closed form.
    fn closed_form_matrix(
        &self,
        src_s: i32,
        src_t: i32,
        tgt_s: i32,
        tgt_t: i32,
        get_image: impl Fn(i32, usize) -> FpVector,
    ) -> Matrix {
        let p = self.prime();
        let algebra = self.resolution.algebra();
        let rows = self.cochain_basis(src_s, src_t); // (d_i, i, α)  — Q(D_src)
        let cols = self.cochain_basis(tgt_s, tgt_t); // (e_l, l, γ) — Q(D_tgt)
        let mut matrix = Matrix::new(p, rows.len(), cols.len());

        let module_src = self.resolution.module(src_s);
        let internal_shift = tgt_t - src_t;

        // Row lookup: (generator degree, generator index) → its first row (α = 0). The α rows for a
        // generator are contiguous by construction of `cochain_basis`.
        let mut row_base: HashMap<(i32, usize), usize> = HashMap::new();
        for (r, &(d_i, i, alpha)) in rows.iter().enumerate() {
            if alpha == 0 {
                row_base.insert((d_i, i), r);
            }
        }

        for (c, &(e_l, l, gamma)) in cols.iter().enumerate() {
            let img_deg = e_l - internal_shift; // degree of the image in P_{src_s}
            if img_deg < 0 {
                continue;
            }
            let mut img = get_image(e_l, l); // image of z_l ∈ (P_{src_s})_{img_deg}
            if img.is_zero() {
                continue;
            }
            // A stored value (e.g. a Nassau differential) can predate generators added later in its
            // degree, on which it vanishes.
            img.extend_len(module_src.dimension(img_deg));
            let gamma_deg = tgt_t - e_l; // |m_γ|

            // For each P_{src_s}-generator (d_i, i): extract a_{li}, act χ(a_{li}) on m_γ, scatter.
            for (&(d_i, i), &base) in &row_base {
                let op_deg = img_deg - d_i;
                if op_deg < 0 {
                    continue;
                }
                let width = algebra.dimension(op_deg);
                if width == 0 {
                    continue;
                }
                // a_{li} = block of the image at generator (d_i, i), as an algebra element.
                let offset = module_src.generator_offset(img_deg, d_i, i);
                let mut a_li = FpVector::new(p, width);
                for q in 0..width {
                    let v = img.entry(offset + q);
                    if v != 0 {
                        a_li.add_basis_element(q, v);
                    }
                }
                if a_li.is_zero() {
                    continue;
                }
                // χ(a_{li}) · m_γ  ∈ M_{src_t - d_i}.  (`op_deg == 0` forces `src_t - d_i ==
                // gamma_deg`, so both branches write into the same graded piece.)
                let mut acted = FpVector::new(p, self.module.dimension(src_t - d_i));
                if op_deg == 0 {
                    // a_{li} = c·1, so χ(a_{li}) m_γ = c·m_γ. Do this directly: some module actions
                    // (e.g. `RealProjectiveSpace`) short-circuit the identity operation
                    // `op_degree == 0` and would drop this term. Only the cup product reaches here
                    // (a minimal `P` has no degree-0 differential component), and this component
                    // is exactly what `hom_k` reads.
                    acted.add_basis_element(gamma, a_li.entry(0));
                } else {
                    let chi = self.antipode.apply_element(op_deg, a_li);
                    for (op_idx, coeff) in chi.iter_nonzero() {
                        self.module.act_on_basis(
                            acted.as_slice_mut(),
                            coeff,
                            op_deg,
                            op_idx,
                            gamma_deg,
                            gamma,
                        );
                    }
                }
                if acted.is_zero() {
                    continue;
                }
                // Entry for each (i, α): coefficient of m_α.
                for alpha in 0..acted.len() {
                    let v = acted.entry(alpha);
                    if v != 0 {
                        matrix.row_mut(base + alpha).add_basis_element(c, v);
                    }
                }
            }
        }
        matrix
    }
}

impl<CC, N> ExtDifferential for TensorResolutionDifferential<CC, N>
where
    CC: FreeChainComplex,
    CC::Algebra: Bialgebra,
    N: Module<Algebra = CC::Algebra>,
{
    fn shift(&self) -> Bidegree {
        // δ: Ext^{s,t} → Ext^{s+1,t}, i.e. (n, s) → (n-1, s+1) with t fixed.
        Bidegree::n_s(-1, 1)
    }

    fn has_own_cochains(&self) -> bool {
        true
    }

    fn dimension(&self, b: Bidegree) -> Option<usize> {
        let (s, t) = (b.s(), b.t());
        if s < 0 {
            return Some(0);
        }
        if !self.computed(s, t) {
            return None;
        }
        Some(self.cochain_basis(s, t).len())
    }

    fn matrix(&self, b: Bidegree) -> Option<Matrix> {
        if let Some(m) = self.matrix_cache.get(&b) {
            return Some((**m).clone());
        }
        let (s, t) = (b.s(), b.t());
        // Below filtration zero there are no cochains, so the differential out of there is a known
        // zero.
        if s < 0 {
            let cols = ExtDifferential::dimension(self, b + self.shift())?;
            return Some(Matrix::new(self.prime(), 0, cols));
        }
        // Need P_s (rows / a_{li}) and P_{s+1} (columns / d_P out of s+1) through degree t.
        if !self.computed(s, t) || !self.computed(s + 1, t) {
            return None;
        }

        // The shape a saved matrix must have.
        let rows = self.cochain_basis(s, t).len();
        let cols = self.cochain_basis(s + 1, t).len();
        let (matrix, from_disk) = if let Some(m) = self.read_delta(b, rows, cols) {
            #[cfg(test)]
            self.disk_reads
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            (m, true)
        } else {
            // δ: C^s → C^{s+1} at the same internal degree, read off the free differential d_P.
            let d_p = self.resolution.differential(s + 1); // P_{s+1} → P_s
            let matrix =
                self.closed_form_matrix(s, t, s + 1, t, |e_l, l| d_p.output(e_l, l).clone());
            (matrix, false)
        };

        // Claim the cache slot before writing: whoever inserts owns the (single) disk write, so a
        // freshly computed matrix is persisted exactly once even if two threads raced to compute it
        // (a fresh `create_new` would otherwise panic on the loser).
        use dashmap::mapref::entry::Entry;
        Some(match self.matrix_cache.entry(b) {
            Entry::Occupied(e) => (**e.get()).clone(),
            Entry::Vacant(e) => {
                if !from_disk {
                    self.write_delta(b, &matrix);
                }
                (**e.insert(Arc::new(matrix))).clone()
            }
        })
    }
}

impl<CC, N> CochainCup<CC> for TensorResolutionDifferential<CC, N>
where
    CC: FreeChainComplex,
    CC::Algebra: Bialgebra,
    N: Module<Algebra = CC::Algebra>,
{
    /// The closed form of [`TensorResolutionDifferential`]'s coboundary, with `map` in place of
    /// $d_P$.
    fn cup_matrix(
        &self,
        map: &FreeModuleHomomorphism<CC::Module>,
        src: Bidegree,
        target: Bidegree,
    ) -> Matrix {
        self.closed_form_matrix(src.s(), src.t(), target.s(), target.t(), |e_l, l| {
            map.output(e_l, l).clone()
        })
    }
}

/// $\Ext_A(M, k)$ by the untwisted tensor resolution (see [`TensorResolutionDifferential`]), as a
/// module over `algebra`.
///
/// The module is built on the resolution $P_\bullet$ of $k$ behind `algebra`, and everything about
/// `M` lives in the attached [`TensorResolutionDifferential`], which serves as both the coboundary
/// $\delta_Q$ and the [`CochainCup`]. Products and Massey products therefore touch only cochains, and the only chain
/// maps built are the ring's own self-maps of $P_\bullet$.
pub fn tensor_resolution_ext<CC, N>(algebra: Arc<ExtAlgebra<CC>>, module: Arc<N>) -> ExtModule<CC>
where
    CC: FreeChainComplex + 'static,
    CC::Algebra: Bialgebra,
    N: Module<Algebra = CC::Algebra> + 'static,
{
    let engine = Arc::new(TensorResolutionDifferential::new(
        Arc::clone(algebra.resolution()),
        module,
    ));
    ExtModule::over_unit(algebra)
        .with_differential(Arc::clone(&engine) as Arc<dyn ExtDifferential>)
        .with_cup(engine as Arc<dyn CochainCup<CC>>)
}

/// The FNV-1a offset basis, the hash of no bytes.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// The FNV-1a prime each byte is multiplied by.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Feed `bytes` into the FNV-1a hash `hash`, which unlike `DefaultHasher` is fixed across Rust
/// releases.
fn fnv1a(hash: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *hash = (*hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
    }
}

/// Panics if `module` has elements in negative degrees.
///
/// A cochain in internal degree `t` pairs a generator of $P_\bullet$ with an element of `M` in the
/// complementary degree, so a negative degree of `M` would need $P_\bullet$ resolved above `t`.
fn assert_nonnegative<N: Module + ?Sized>(module: &N) {
    assert!(
        module.min_degree() >= 0,
        "the untwisted tensor resolution needs a module in nonnegative degrees, but {module} \
         starts in degree {}; suspend it first",
        module.min_degree()
    );
}

/// `name` as a portable zarr node name, which must also be a directory name on every platform.
///
/// Every byte outside `[a-z0-9._-]` is percent-encoded with uppercase hex digits, and so are a
/// leading `_` (zarr reserves `__`), a trailing `.` (which Windows drops) and the first letter of a
/// Windows device name such as `con` or `nul.txt`. Decoding is plain percent-decoding, so distinct
/// names get distinct groups. Uppercase letters appear only as hex digits after a `%`, so they stay
/// distinct on a case-insensitive filesystem too. The empty name, which encodes to nothing, becomes
/// `%`, which no other name encodes to.
fn save_group_name(name: &str) -> String {
    const DEVICES: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or("");
    let device = DEVICES.iter().any(|d| d.eq_ignore_ascii_case(stem));
    let bytes = name.as_bytes();
    let mut out = String::with_capacity(name.len());
    for (i, &byte) in bytes.iter().enumerate() {
        let plain = byte.is_ascii_lowercase()
            || byte.is_ascii_digit()
            || matches!(byte, b'.' | b'_' | b'-');
        let escaped =
            (i == 0 && (byte == b'_' || device)) || (i + 1 == bytes.len() && byte == b'.');
        if plain && !escaped {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    if out.is_empty() {
        out.push('%');
    }
    out
}

/// The dualised differential $\Hom_A(\partial, k)$ of any free chain complex `Q`, read off `Q`'s
/// own differential, as an [`ExtDifferential`].
///
/// It is zero for a minimal `Q` and non-zero for a non-minimal one, which [`TensorResolution`]
/// may be.
///
/// The cochain basis at each bidegree is exactly `Q`'s generators, so an [`ExtModule`] carrying
/// this differential shares one coordinate system with `Q`'s secondary machinery, which lets
/// [`TensorResolutionSecondary`] transport the Adams $d_2$ between them by
/// [`lift`](ExtModule::lift)/[`project`](ExtModule::project).
pub struct DualizedDifferential<CC: FreeChainComplex> {
    complex: Arc<CC>,
}

impl<CC: FreeChainComplex> DualizedDifferential<CC> {
    /// The dualised differential of `complex`.
    pub fn new(complex: Arc<CC>) -> Self {
        Self { complex }
    }
}

impl<CC: FreeChainComplex> ExtDifferential for DualizedDifferential<CC> {
    fn shift(&self) -> Bidegree {
        // δ: Ext^{s,t} → Ext^{s+1,t}, i.e. (n, s) → (n-1, s+1) with t fixed.
        Bidegree::n_s(-1, 1)
    }

    fn matrix(&self, b: Bidegree) -> Option<Matrix> {
        let (s, t) = (b.s(), b.t());
        let target = b + self.shift(); // (s + 1, t)
        // Below filtration zero there are no cochains, so the differential out of there is a known
        // zero.
        if s < 0 {
            if !self.complex.has_computed_bidegree(target) {
                return None;
            }
            let cols = self.complex.number_of_gens_in_bidegree(target);
            return Some(Matrix::new(self.complex.prime(), 0, cols));
        }
        if !self.complex.has_computed_bidegree(b) || !self.complex.has_computed_bidegree(target) {
            return None;
        }
        let p = self.complex.prime();
        let rows = self.complex.number_of_gens_in_bidegree(b); // C^s_t
        let cols = self.complex.number_of_gens_in_bidegree(target); // C^{s+1}_t
        let mut matrix = Matrix::new(p, rows, cols);
        // ∂_{s+1}: Q_{s+1} → Q_s; `hom_k(t)` is indexed [Q_s gen][Q_{s+1} gen] = [row][col], with
        // entry = coefficient of Q_s-generator `row` in ∂ of Q_{s+1}-generator `col` — exactly the
        // coboundary δ_{row,col}.
        let hk = self.complex.differential(s + 1).hom_k(t);
        for (row, cols_of_row) in hk.iter().enumerate() {
            for (col, &v) in cols_of_row.iter().enumerate() {
                if v != 0 {
                    matrix.row_mut(row).set_entry(col, v);
                }
            }
        }
        Some(matrix)
    }
}

/// The free resolution $Q_\bullet = P_\bullet \otimes M \to M$ of `M` whose dual
/// [`TensorResolutionDifferential`] computes, materialised as a [`FreeChainComplex`].
///
/// [`SecondaryResolution`] needs the chain complex itself, not just its dual, so this is what
/// [`TensorResolutionSecondary`] builds the Adams $d_2$ on.
///
/// # The free differential, in closed form
/// $Q_s = P_s \otimes M$ carries the diagonal $A$-action; the untwisting isomorphism
/// $\Phi\colon P_s \otimes M^{\mathrm{triv}} \xrightarrow{\ \sim\ } P_s \otimes M$,
/// $b\,x_i \otimes m \mapsto \sum b_{(1)} x_i \otimes b_{(2)} m$, presents it as *free* on the
/// pairs $x_i \otimes m_\alpha$. Conjugating $\partial_P \otimes \mathrm{id}$ by $\Phi$ (its
/// inverse is $\Psi(b\,y \otimes m) = \sum b_{(1)} y \otimes \chi(b_{(2)}) m$) gives the free
/// differential $$ \partial_Q(x_i \otimes m_\alpha) = \sum_j \sum_{(a_{ij})} a'_{ij} \cdot
/// \bigl(y_j \otimes \chi(a''_{ij})\, m_\alpha\bigr), $$ where $\partial_P(x_i) = \sum_j a_{ij}
/// y_j$ and $\Delta(a_{ij}) = \sum a'_{ij} \otimes a''_{ij}$ is the full coproduct. Its
/// $\Hom_A(-, k)$ keeps only the augmentation part $a' = 1$, which is the closed form of
/// [`TensorResolutionDifferential`]; and it squares to zero, being conjugate to
/// $\partial_P \otimes \mathrm{id}$.
///
/// Generators of $Q_s$ at internal degree `t` are added in the same $(x_i, m_\alpha)$ order as
/// `TensorResolutionDifferential`'s cochain basis, so the two share one coordinate system.
pub struct TensorResolution<CC, N>
where
    CC: FreeChainComplex,
    CC::Algebra: Bialgebra,
    N: Module<Algebra = CC::Algebra> + ZeroModule,
{
    /// $P_\bullet$: a (minimal) free resolution of `k`.
    resolution: Arc<CC>,
    /// The module `M` being resolved.
    module: Arc<N>,
    antipode: Antipode<CC::Algebra>,
    coproduct: FullCoproduct<CC::Algebra>,
    zero_module: Arc<FreeModule<CC::Algebra>>,
    /// `s` → $Q_s = P_s \otimes M$.
    modules: OnceBiVec<Arc<FreeModule<CC::Algebra>>>,
    /// `s` → $\partial_s\colon Q_s \to Q_{s-1}$ (with $\partial_0\colon Q_0 \to 0$).
    differentials: OnceVec<Arc<FreeModuleHomomorphism<FreeModule<CC::Algebra>>>>,
    lock: Mutex<()>,
}

impl<CC, N> TensorResolution<CC, N>
where
    CC: FreeChainComplex,
    CC::Algebra: Bialgebra,
    N: Module<Algebra = CC::Algebra> + ZeroModule,
{
    /// $P_\bullet \otimes M$ for `resolution` = $P_\bullet$ and `module` = `M`, with nothing
    /// computed yet.
    ///
    /// # Panics
    ///
    /// If `M` has elements in negative degrees, as [`TensorResolutionDifferential::new`].
    pub fn new(resolution: Arc<CC>, module: Arc<N>) -> Self {
        assert_nonnegative(&*module);
        let algebra = resolution.algebra();
        let antipode = Antipode::new(Arc::clone(&algebra));
        let coproduct = FullCoproduct::new(Arc::clone(&algebra));
        let zero_module = Arc::new(FreeModule::new(Arc::clone(&algebra), "0".to_string(), 0));
        Self {
            resolution,
            module,
            antipode,
            coproduct,
            zero_module,
            modules: OnceBiVec::new(0),
            differentials: OnceVec::new(),
            lock: Mutex::new(()),
        }
    }

    /// Build $Q_s$ (free modules and differentials) for all `s ≤ s_max` through internal degree
    /// `t_max`. Idempotent and monotone: re-calling with a larger box extends in place.
    fn extend(&self, s_max: i32, t_max: i32) {
        if s_max < 0 || t_max < 0 {
            return;
        }
        let _lock = self.lock.lock().unwrap();
        let algebra = self.resolution.algebra();

        // Grow the substrate first: P_• through (s_max, t_max); M and the algebra through t_max.
        self.resolution
            .compute_through_bidegree(Bidegree::s_t(s_max, t_max));
        self.module.compute_basis(t_max);
        algebra.compute_basis(t_max);
        self.zero_module.compute_basis(t_max);

        // Ensure a free module exists for each 0..=s_max, then grow its generators + basis to
        // t_max.
        for s in self.modules.len()..=s_max {
            self.modules.push(Arc::new(FreeModule::new(
                Arc::clone(&algebra),
                format!("(P⊗M)_{s}"),
                0,
            )));
        }
        for s in 0..=s_max {
            let fm = &self.modules[s];
            fm.compute_basis(t_max);
            let p_s = self.resolution.module(s);
            for d in (fm.max_computed_degree() + 1)..=t_max {
                // Generators of Q_s at degree d: the (i, α) with |x_i| + |m_α| = d.
                let count: usize = p_s
                    .iter_gens(d)
                    .map(|(d_i, _)| self.module.dimension(d - d_i))
                    .sum();
                fm.add_generators(d, count, None);
            }
        }

        // Ensure the differentials exist, then grow their outputs to t_max.
        for s in self.differentials.len() as i32..=s_max {
            let d = if s == 0 {
                FreeModuleHomomorphism::new(
                    Arc::clone(&self.modules[0]),
                    Arc::clone(&self.zero_module),
                    0,
                )
            } else {
                FreeModuleHomomorphism::new(
                    Arc::clone(&self.modules[s]),
                    Arc::clone(&self.modules[s - 1]),
                    0,
                )
            };
            self.differentials.push(Arc::new(d));
        }
        for s in 0..=s_max {
            let d = &self.differentials[s as usize];
            if s == 0 {
                d.extend_by_zero(t_max);
            } else {
                for degree in d.next_degree()..=t_max {
                    let rows = self.differential_rows(s, degree);
                    d.add_generators_from_rows(degree, rows);
                }
            }
        }

        // The secondary machinery lifts intermediates through the differentials, so each needs its
        // quasi-inverse (image/kernel/QI). Minimal resolvers build these while resolving; here we
        // compute them explicitly for the materialised free differentials.
        for s in 0..=s_max {
            self.differentials[s as usize].compute_auxiliary_data_through_degree(t_max);
        }
    }

    /// The outputs of $\partial_s\colon Q_s \to Q_{s-1}$ on the generators of $Q_s$ at degree
    /// `degree`, one [`FpVector`] (in $(Q_{s-1})_{\text{degree}}$) per generator, in generator
    /// order.
    fn differential_rows(&self, s: i32, degree: i32) -> Vec<FpVector> {
        let p = self.resolution.prime();
        let algebra = self.resolution.algebra();
        let fm = &self.modules[s];
        let target = &self.modules[s - 1];
        let p_s = self.resolution.module(s);
        let p_prev = self.resolution.module(s - 1);
        let d_p = self.resolution.differential(s);
        let tgt_dim = target.dimension(degree);

        let mut rows: Vec<FpVector> = Vec::with_capacity(fm.number_of_gens_in_degree(degree));
        for (d_i, i) in p_s.iter_gens(degree) {
            let e_alpha = degree - d_i;
            let m_dim = self.module.dimension(e_alpha);
            if m_dim == 0 {
                continue;
            }
            // ∂_P(x_i) ∈ (P_{s-1})_{d_i}.
            let mut dp = FpVector::new(p, p_prev.dimension(d_i));
            d_p.apply_to_generator(&mut dp, 1, d_i, i);

            for alpha in 0..m_dim {
                let mut out = FpVector::new(p, tgt_dim);
                if !dp.is_zero() {
                    // For each P_{s-1}-generator (d'_j, j): extract a_{ij}, apply the Hopf formula.
                    for (dpj, j) in p_prev.iter_gens(d_i) {
                        let op_deg = d_i - dpj; // |a_{ij}|
                        let width = algebra.dimension(op_deg);
                        if width == 0 {
                            continue;
                        }
                        let off = p_prev.generator_offset(d_i, dpj, j);
                        for a_idx in 0..width {
                            let coeff = dp.entry(off + a_idx);
                            if coeff == 0 {
                                continue;
                            }
                            // ∂_Q term: Σ_(a) a' · (y_j ⊗ χ(a'') m_α).
                            for &(l_deg, l_idx, r_deg, r_idx) in
                                self.coproduct.terms(op_deg, a_idx).iter()
                            {
                                // χ(a'') m_α ∈ M_{r_deg + e_alpha}.
                                let mbeta_deg = r_deg + e_alpha;
                                let mut acted = FpVector::new(p, self.module.dimension(mbeta_deg));
                                if r_deg == 0 {
                                    // a'' = 1, so χ(a'') m_α = m_α. Done directly for the reason
                                    // in `closed_form_matrix`.
                                    acted.add_basis_element(alpha, coeff);
                                } else {
                                    let chi = self.antipode.apply(r_deg, r_idx);
                                    for (op_idx, op_c) in chi.iter_nonzero() {
                                        self.module.act_on_basis(
                                            acted.as_slice_mut(),
                                            (op_c * coeff) % p.as_u32(),
                                            r_deg,
                                            op_idx,
                                            e_alpha,
                                            alpha,
                                        );
                                    }
                                }
                                if acted.is_zero() {
                                    continue;
                                }
                                // Place a' · gen(j, β) for each β: block = generator (j, β) of
                                // degree d'_j + |m_β|; within-block offset = l_idx (a' at deg
                                // l_deg).
                                let gen_deg = dpj + mbeta_deg;
                                for (beta, b_c) in acted.iter_nonzero() {
                                    let gen_idx =
                                        self.local_gen_index(s - 1, gen_deg, dpj, j, beta);
                                    let block = target.generator_offset(degree, gen_deg, gen_idx);
                                    debug_assert_eq!(degree - gen_deg, l_deg);
                                    out.add_basis_element(block + l_idx, b_c);
                                }
                            }
                        }
                    }
                }
                rows.push(out);
            }
        }
        rows
    }

    /// Index of the generator $(y_j, m_\beta)$ among the generators of $Q_s$ at degree `gen_deg`,
    /// where `y_j` is the P_s-generator `(target_pdeg, target_pidx)`. Matches the add-order in
    /// [`Self::extend`] (iterate P_s generators, then M-basis β).
    fn local_gen_index(
        &self,
        s: i32,
        gen_deg: i32,
        target_pdeg: i32,
        target_pidx: usize,
        beta: usize,
    ) -> usize {
        let p_s = self.resolution.module(s);
        let mut idx = 0;
        for (dk, k) in p_s.iter_gens(gen_deg) {
            if dk == target_pdeg && k == target_pidx {
                return idx + beta;
            }
            idx += self.module.dimension(gen_deg - dk);
        }
        panic!("generator ({target_pdeg}, {target_pidx}) not found in Q_{s} at degree {gen_deg}");
    }
}

impl<CC, N> ChainComplex for TensorResolution<CC, N>
where
    CC: FreeChainComplex,
    CC::Algebra: Bialgebra,
    N: Module<Algebra = CC::Algebra> + ZeroModule,
{
    type Algebra = CC::Algebra;
    type Homomorphism = FreeModuleHomomorphism<FreeModule<CC::Algebra>>;
    type Module = FreeModule<CC::Algebra>;

    fn algebra(&self) -> Arc<Self::Algebra> {
        self.resolution.algebra()
    }

    fn min_degree(&self) -> i32 {
        0
    }

    fn zero_module(&self) -> Arc<Self::Module> {
        Arc::clone(&self.zero_module)
    }

    fn module(&self, s: i32) -> Arc<Self::Module> {
        Arc::clone(&self.modules[s])
    }

    fn differential(&self, s: i32) -> Arc<Self::Homomorphism> {
        Arc::clone(&self.differentials[s as usize])
    }

    fn has_computed_bidegree(&self, b: Bidegree) -> bool {
        b.s() >= 0
            && self.differentials.len() > b.s() as usize
            && self.differential(b.s()).next_degree() > b.t()
    }

    fn compute_through_bidegree(&self, b: Bidegree) {
        self.extend(b.s(), b.t());
    }

    fn next_homological_degree(&self) -> i32 {
        // `extend` pushes each differential after its module, so counting differentials never
        // reports a filtration whose differential a concurrent `extend` has yet to push.
        self.differentials.len() as i32
    }
}

/// The Adams $d_2$ on $\Ext_A(M, k)$ for a module resolved by the untwisted tensor resolution.
///
/// It wraps the [`TensorResolution`] $Q_\bullet = P_\bullet \otimes M$ in the standard
/// [`SecondaryResolution`] machinery (which computes $d_2$ from any free resolution — $d_2$ is a
/// chain-homotopy invariant), and an [`ExtModule`] whose $E_2$ page is the cohomology of
/// $\Hom_A(Q, k)$ (via [`DualizedDifferential`]). Because $Q$ may be non-minimal, $d_2$ is only
/// defined on cohomology classes, not on cochain generators; the transport
/// $$ d_2 = \text{project} \circ (\text{cochain } d_2) \circ \text{lift} $$
/// restricts to cocycles and quotients the spurious coboundary part, via
/// [`lift`](ExtModule::lift)/[`project`](ExtModule::project).
///
/// Like [`SecondaryExtAlgebra`](super::SecondaryExtAlgebra), it assumes the zero secondary
/// structure on `M`, which does not lift for every module (it fails for `RP^∞`).
pub struct TensorResolutionSecondary<CC, N>
where
    CC: FreeChainComplex + 'static,
    CC::Algebra: Bialgebra + PairAlgebra,
    N: Module<Algebra = CC::Algebra> + ZeroModule + 'static,
{
    resolution: Arc<TensorResolution<CC, N>>,
    secondary: Arc<SecondaryResolution<TensorResolution<CC, N>>>,
    /// The $E_2$ page of $\Ext_A(M, k)$, in $Q_\bullet$-generator (cochain) coordinates.
    e2: ExtModule<TensorResolution<CC, N>>,
    /// Held by [`compute_through_stem`](Self::compute_through_stem) throughout.
    lock: Mutex<()>,
}

impl<CC, N> TensorResolutionSecondary<CC, N>
where
    CC: FreeChainComplex + 'static,
    CC::Algebra: Bialgebra + PairAlgebra,
    N: Module<Algebra = CC::Algebra> + ZeroModule + 'static,
{
    /// Build the secondary layer over the untwisted tensor resolution. `resolution` resolves the
    /// base field `k`; `module` is `M`. Construction is cheap — call
    /// [`compute_through_stem`](Self::compute_through_stem) to do the work.
    pub fn new(resolution: Arc<CC>, module: Arc<N>) -> Self {
        let q = Arc::new(TensorResolution::new(resolution, module));
        // Q• may be non-minimal, so its composite ∂∂ can hit same-degree generators.
        let secondary = Arc::new(SecondaryResolution::new_with_hit_generator(
            Arc::clone(&q),
            true,
        ));
        let e2 = ExtModule::intrinsic(Arc::clone(&q))
            .with_differential(Arc::new(DualizedDifferential::new(Arc::clone(&q))));
        Self {
            resolution: q,
            secondary,
            e2,
            lock: Mutex::new(()),
        }
    }

    /// The prime of the resolution.
    fn prime(&self) -> ValidPrime {
        self.resolution.prime()
    }

    /// The materialised free resolution $Q_\bullet = P_\bullet \otimes M$.
    pub fn resolution(&self) -> &Arc<TensorResolution<CC, N>> {
        &self.resolution
    }

    /// The $E_2$ page $\Ext_A(M, k)$, with cohomology exposed via
    /// [`lift`](ExtModule::lift)/[`project`](ExtModule::project).
    pub fn ext(&self) -> &ExtModule<TensorResolution<CC, N>> {
        &self.e2
    }

    /// Compute $Q_\bullet$ and its secondary homotopies far enough to read $d_2$ on the box up to
    /// `max`. Resolves $Q_\bullet$ with the margin the Adams $d_2$ needs (one extra stem and two
    /// extra filtrations), then extends the secondary resolution.
    pub fn compute_through_stem(&self, max: Bidegree) {
        // d2 out of (n, s) reads the target (n-1, s+2), and the secondary homotopies at s need the
        // resolution two filtrations higher; also grow one extra stem for the incoming coboundary.
        let margin = Bidegree::n_s(max.n() + 1, max.s() + 3);
        // Two calls would otherwise both find the same homotopy missing and both insert it.
        let _guard = self.lock.lock().unwrap();
        self.resolution.compute_through_bidegree(margin);
        self.secondary.extend_all();
    }

    /// The dimension of $\Ext_A(M, k)$ (the $E_2$ page) at bidegree `b`.
    pub fn cohomology_dimension(&self, b: Bidegree) -> Option<usize> {
        self.e2.cohomology_dimension(b)
    }

    /// The Adams differential $d_2(x)$, a class in bidegree `(n - 1, s + 2)`.
    ///
    /// Returns `None` if the target bidegree is out of the computed range. A computed-but-zero
    /// differential is `Some` of a zero class.
    pub fn d2(&self, x: &BidegreeElement) -> Option<BidegreeElement> {
        let b = x.degree();
        let target = b + Bidegree::n_s(-1, 2);
        if !self.resolution.has_computed_bidegree(target) {
            return None;
        }
        // The class is lifted at `b` and projected at `target`, through the cochain d2 at `b`.
        self.e2.cohomology(b)?;
        let target_classes = self.e2.cohomology(target)?.dimension();
        if target_classes == 0 {
            return Some(BidegreeElement::new(target, FpVector::new(self.prime(), 0)));
        }
        let homotopies = self.secondary.homotopies();
        if b.t() <= 0
            || !homotopies.range().contains(&(b.s() + 2))
            || homotopies[b.s() + 2].homotopies.next_degree() <= target.t()
        {
            return None;
        }

        // Lift the class to a cocycle representative in Q_•-generator coordinates.
        let cocycle = self.e2.lift(x);

        // Cochain-level d2: `m[i]` is the d2 of the i-th Q_•-generator at `b`, as a vector over the
        // Q_•-generators at `target`. This is exactly what `SecondaryResolution::e3_page` reads.
        let m = self.secondary.homotopy(b.s() + 2).homotopies.hom_k(b.t());

        let target_dim = self.resolution.number_of_gens_in_bidegree(target);
        let mut out = FpVector::new(self.prime(), target_dim);
        if !m.is_empty() && !m[0].is_empty() {
            let p = self.prime().as_u32();
            for (i, ci) in cocycle.vec().iter_nonzero() {
                for (k, &v) in m[i].iter().enumerate() {
                    out.add_basis_element(k, (ci * v) % p);
                }
            }
        }

        // Project the resulting cochain back to an Ext class (quotient by coboundaries).
        Some(self.e2.project(&Cochain::new(target, out)))
    }

    /// Whether `x` is a $d_2$-cycle (survives to $E_3$). `None` if $d_2$ is out of range.
    pub fn survives(&self, x: &BidegreeElement) -> Option<bool> {
        self.d2(x).map(|d| d.vec().is_zero())
    }
}

#[cfg(test)]
mod tests {
    use algebra::{
        SteenrodAlgebra,
        module::{FDModule, Module},
    };
    use fp::prime::TWO;
    use sseq::coordinates::Bidegree;

    use super::*;
    use crate::{
        chain_complex::{ChainComplex, FreeChainComplex},
        utils::construct_standard,
    };

    /// The antipode of `S_2`'s algebra, with the algebra basis computed through `max`.
    fn sphere_antipode(max: i32) -> (Arc<SteenrodAlgebra>, Antipode<SteenrodAlgebra>) {
        let res = construct_standard::<false, _, _>("S_2", None).unwrap();
        let algebra = res.algebra();
        algebra.compute_basis(max + 1);
        let antipode = Antipode::new(Arc::clone(&algebra));
        (algebra, antipode)
    }

    /// χ fixes the primitive degree-1 generator, and χ² = id since `A` is cocommutative.
    ///
    /// Both checks are independent of the algebra's basis.
    /// Over the Adem basis `Sq²Sq¹` decomposes into two factors, so its coproduct depends on the
    /// order they are multiplied in: `Δ(Sq²Sq¹) = Δ(Sq²)Δ(Sq¹)` has the term `Sq²Sq¹ ⊗ 1`, where
    /// `Δ(Sq¹)Δ(Sq²) = Δ(Sq³)` would have `Sq³ ⊗ 1`.
    #[test]
    fn coproduct_multiplies_adem_factors_in_order() {
        use algebra::AdemAlgebra;

        let algebra = Arc::new(AdemAlgebra::new(TWO, false));
        algebra.compute_basis(3);
        let idx = |ps: &[u32]| {
            (0..algebra.dimension(3))
                .find(|&i| algebra.basis_element_from_index(3, i).ps == ps)
                .unwrap()
        };
        let (sq3, sq2sq1) = (idx(&[3]), idx(&[2, 1]));

        let mut terms = FullCoproduct::new(Arc::clone(&algebra))
            .terms(3, sq2sq1)
            .to_vec();
        terms.sort();
        let mut expected = vec![
            (0, 0, 3, sq2sq1),
            (1, 0, 2, 0),
            (2, 0, 1, 0),
            (3, sq2sq1, 0, 0),
        ];
        expected.sort();
        assert_eq!(terms, expected);
        assert!(!terms.contains(&(3, sq3, 0, 0)));
    }

    #[test]
    fn antipode_low_degree_and_involution() {
        let (algebra, chi) = sphere_antipode(10);

        assert_eq!(algebra.dimension(1), 1);
        let mut deg1 = FpVector::new(TWO, 1);
        deg1.add_basis_element(0, 1);
        assert_eq!(
            chi.apply(1, 0),
            deg1,
            "χ fixes the primitive degree-1 generator"
        );

        for degree in 0..=10 {
            for idx in 0..algebra.dimension(degree) {
                let chi_x = chi.apply(degree, idx);
                let chi_chi_x = chi.apply_element(degree, chi_x);
                let mut x = FpVector::new(TWO, algebra.dimension(degree));
                x.add_basis_element(idx, 1);
                assert_eq!(chi_chi_x, x, "χ² = id at degree {degree}, idx {idx}");
            }
        }
    }

    /// Σ χ(x_(1)) x_(2) = ε(x)·1 for every positive-degree basis element x through some degree.
    #[test]
    fn antipode_hopf_identity() {
        let (algebra, chi) = sphere_antipode(10);
        let coproduct = FullCoproduct::new(Arc::clone(&algebra));
        for degree in 1..=10 {
            for idx in 0..algebra.dimension(degree) {
                let mut acc = FpVector::new(TWO, algebra.dimension(degree));
                for &(l_deg, l_idx, r_deg, r_idx) in coproduct.terms(degree, idx).iter() {
                    let chi_left = chi.apply(l_deg, l_idx);
                    for (j, coeff) in chi_left.iter_nonzero() {
                        algebra.multiply_basis_elements(
                            acc.as_slice_mut(),
                            coeff,
                            l_deg,
                            j,
                            r_deg,
                            r_idx,
                        );
                    }
                }
                assert!(
                    acc.is_zero(),
                    "Hopf identity failed at degree {degree}, idx {idx}: {acc:?}"
                );
            }
        }
    }

    /// The untwisted tensor resolution reproduces the direct minimal resolution of a finite module.
    fn assert_matches_direct(module_name: &str, sphere: &str, nn: i32, ss: i32) {
        let t_max = nn + ss;

        // k-resolution and M over the same algebra.
        let k_res = Arc::new(construct_standard::<false, _, _>(sphere, None).unwrap());
        k_res.compute_through_bidegree(Bidegree::s_t(ss + 1, t_max));
        k_res.algebra().compute_basis(t_max + 1);

        let m_json = crate::utils::parse_module_name(module_name).unwrap();
        let module = Arc::new(
            FDModule::from_json(k_res.algebra(), &m_json).expect("finite module for cross-check"),
        );
        module.compute_basis(t_max);

        let alg = tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&k_res))), module);

        // Direct minimal resolution of M (its generator counts are Ext(M,k)).
        let direct = Arc::new(construct_standard::<false, _, _>(module_name, None).unwrap());
        direct.compute_through_stem(Bidegree::n_s(nn, ss));

        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                assert_eq!(
                    alg.cohomology_dimension(b),
                    Some(direct.number_of_gens_in_bidegree(b)),
                    "Ext({module_name}, k) mismatch at {b:?}",
                );
            }
        }
    }

    /// [`assert_matches_direct`] for `C2`.
    #[test]
    fn tensor_resolution_matches_direct_c2() {
        assert_matches_direct("C2", "S_2", 10, 6);
    }

    /// Distinct module names get save groups that differ even ignoring case, each of which a store
    /// can create.
    #[test]
    fn save_group_names_are_distinct_and_valid() {
        let names = [
            "C2", "C/2", "C_2", "C%2F2", "_x", "__x", "%5Fx", "", "%", ".", "..", "a.b", "a.",
            "C?2", "C:2", "C*2", "CON", "con.txt", "CONSOLE", "RP^∞", "C 2", "c2", "%432", "%43",
            "C",
        ];
        let encoded: Vec<String> = names.iter().map(|n| save_group_name(n)).collect();
        let dir = tempfile::tempdir().unwrap();
        let store = crate::save::ZarrSaveStore::create(dir.path()).unwrap();
        for (i, e) in encoded.iter().enumerate() {
            assert!(
                e.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-%".contains(&b)),
                "{e:?}"
            );
            assert!(!e.starts_with("__") && !e.ends_with('.'), "{e:?}");
            for f in &encoded[..i] {
                assert!(
                    !e.eq_ignore_ascii_case(f),
                    "two names share the group {e:?}"
                );
            }
            store
                .subgroup(&format!("tensor/{e}"))
                .unwrap_or_else(|err| panic!("cannot create the group {e:?}: {err}"));
        }
        assert_eq!(save_group_name("RP_1"), "%52%50_1");
    }

    /// A module in negative degrees is refused rather than given a truncated cochain basis.
    #[test]
    #[should_panic(expected = "nonnegative degrees")]
    fn tensor_resolution_rejects_negative_degrees() {
        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let json = serde_json::json!({
            "p": 2,
            "type": "finite dimensional module",
            "gens": { "x": -1 },
            "actions": []
        });
        let m = Arc::new(FDModule::from_json(k_res.algebra(), &json).unwrap());
        tensor_resolution_ext(Arc::new(ExtAlgebra::new(k_res)), m);
    }

    /// The `δ_Q` matrices are saved under the $k$-resolution's save directory, and a fresh engine
    /// over the same directory reads them back and reproduces $\Ext(C2, k)$.
    #[test]
    fn tensor_delta_matrices_persist_to_disk() {
        let (nn, ss) = (10, 6);
        let t_max = nn + ss;
        let dir = tempfile::tempdir().unwrap();
        let k_res = Arc::new(
            construct_standard::<false, _, _>("S_2", Some(dir.path().to_owned())).unwrap(),
        );
        k_res.compute_through_bidegree(Bidegree::s_t(ss + 1, t_max));
        k_res.algebra().compute_basis(t_max + 1);

        let direct = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        direct.compute_through_stem(Bidegree::n_s(nn, ss));

        let check_against_direct = |ext: &ExtModule<_>| {
            for n in 0..=nn {
                for s in 0..=ss {
                    let b = Bidegree::n_s(n, s);
                    assert_eq!(
                        ext.cohomology_dimension(b),
                        Some(direct.number_of_gens_in_bidegree(b)),
                        "Ext(C2, k) mismatch at {b:?}",
                    );
                }
            }
        };

        let ring = Arc::new(ExtAlgebra::new(Arc::clone(&k_res)));
        let ext = tensor_resolution_ext(
            Arc::clone(&ring),
            finite_module(&k_res.algebra(), "C2", t_max),
        );
        check_against_direct(&ext);

        // Every δ_Q the check used is on disk, so the engine below reads rather than computes it.
        // An empty bidegree reads none.
        let store = crate::save::ZarrSaveStore::create(dir.path())
            .unwrap()
            .subgroup(&format!(
                "tensor/{}",
                save_group_name(&c2_name(&k_res.algebra()))
            ))
            .unwrap();
        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                if ext.cochain_dimension(b) == 0 {
                    continue;
                }
                assert!(
                    store
                        .read(SaveKind::TensorDifferential, b)
                        .unwrap()
                        .is_some(),
                    "δ_Q out of {b} was not saved"
                );
            }
        }

        // A fresh engine has an empty in-memory cache, so every matrix it uses comes from disk.
        let engine = Arc::new(TensorResolutionDifferential::new(
            Arc::clone(&k_res),
            finite_module(&k_res.algebra(), "C2", t_max),
        ));
        let reloaded = ExtModule::over_unit(ring)
            .with_differential(Arc::clone(&engine) as Arc<dyn ExtDifferential>);
        check_against_direct(&reloaded);
        let used = engine.matrix_cache.len();
        assert!(used > 0);
        assert_eq!(
            engine.disk_reads.load(std::sync::atomic::Ordering::Relaxed),
            used,
            "the reloaded engine recomputed a saved matrix"
        );
    }

    /// Two modules with the same name share a save group, so the fingerprint in each payload must
    /// reject the matrices of the other, even when they also agree in dimension.
    #[test]
    #[should_panic(expected = "written for a different module")]
    fn tensor_delta_save_dir_rejects_a_different_module() {
        let (nn, ss) = (8, 4);
        let t_max = nn + ss;
        let dir = tempfile::tempdir().unwrap();
        let k_res = Arc::new(
            construct_standard::<false, _, _>("S_2", Some(dir.path().to_owned())).unwrap(),
        );
        k_res.compute_through_bidegree(Bidegree::s_t(ss + 1, t_max));
        k_res.algebra().compute_basis(t_max + 1);
        let ring = Arc::new(ExtAlgebra::new(Arc::clone(&k_res)));

        let c2 = tensor_resolution_ext(
            Arc::clone(&ring),
            finite_module(&k_res.algebra(), "C2", t_max),
        );
        for n in 0..=nn {
            for s in 0..=ss {
                let _ = c2.cohomology_dimension(Bidegree::n_s(n, s));
            }
        }

        // k ⊕ Σk: the dimensions of C2, without its Sq¹.
        let split_json = serde_json::json!({
            "p": 2,
            "type": "finite dimensional module",
            "name": c2_name(&k_res.algebra()),
            "gens": { "x0": 0, "x1": 1 },
            "actions": []
        });
        let split = Arc::new(FDModule::from_json(k_res.algebra(), &split_json).unwrap());
        split.compute_basis(t_max);
        let split = tensor_resolution_ext(ring, split);
        for n in 0..=nn {
            for s in 0..=ss {
                let _ = split.cohomology_dimension(Bidegree::n_s(n, s));
            }
        }
    }

    /// The name `C2` is saved under, as its module reports it.
    fn c2_name(algebra: &Arc<SteenrodAlgebra>) -> String {
        finite_module(algebra, "C2", 0).to_string()
    }

    /// `dimension` is the cohomology dimension, at most the cochain dimension and somewhere
    /// strictly less, and `project ∘ lift` is the identity on cohomology classes.
    #[test]
    fn cohomology_basis_and_lift_project() {
        use sseq::coordinates::BidegreeGenerator;

        let (nn, ss) = (12, 7);
        let t_max = nn + ss;
        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        k_res.compute_through_bidegree(Bidegree::s_t(ss + 1, t_max));
        k_res.algebra().compute_basis(t_max + 1);
        let m = Arc::new(
            FDModule::from_json(
                k_res.algebra(),
                &crate::utils::parse_module_name("C2").unwrap(),
            )
            .unwrap(),
        );
        m.compute_basis(t_max);
        let alg = tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&k_res))), m);

        let mut saw_nonminimal = false;
        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                let dim = alg.dimension(b);
                assert_eq!(Some(dim), alg.cohomology_dimension(b));
                let cochain = alg.cochain_dimension(b);
                assert!(
                    cochain >= dim,
                    "cochain {cochain} < cohomology {dim} at {b:?}"
                );
                saw_nonminimal |= cochain > dim;

                for i in 0..dim {
                    let class = alg.generator(BidegreeGenerator::new(b, i));
                    assert_eq!(
                        class,
                        alg.project(&alg.lift(&class)),
                        "project∘lift {b:?}#{i}"
                    );
                }
            }
        }
        assert!(
            saw_nonminimal,
            "expected a non-minimal bidegree where cochain dim > cohomology dim"
        );
    }

    /// Ext of the infinite module `RP^∞` agrees with its direct resolution.
    ///
    /// Nassau's algorithm cannot resolve `RP^∞`, but the standard resolver works degreewise and
    /// can, which gives the ground truth.
    #[test]
    fn tensor_resolution_matches_direct_rp_inf() {
        use algebra::module::RealProjectiveSpace;

        let (nn, ss) = (18, 8);
        let t_max = nn + ss;

        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        k_res.compute_through_bidegree(Bidegree::s_t(ss + 1, t_max));
        k_res.algebra().compute_basis(t_max + 1);
        let rp = Arc::new(RealProjectiveSpace::new(k_res.algebra(), 1, None, false));
        rp.compute_basis(t_max);
        let alg = tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&k_res))), rp);

        // Direct resolution of the infinite module RP^∞ (non-nassau; resolves degreewise).
        let direct = Arc::new(construct_standard::<false, _, _>("RP_inf", None).unwrap());
        direct.compute_through_stem(Bidegree::n_s(nn, ss));

        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                assert_eq!(
                    alg.cohomology_dimension(b),
                    Some(direct.number_of_gens_in_bidegree(b)),
                    "Ext(RP^∞, k) mismatch at {b:?}",
                );
            }
        }

        // Sanity: Ext^0 = A-module indecomposables of RP^∞ = F_2[x], which sit in degrees 2^k − 1
        // (since Sq^1 x = x^2 makes even powers decomposable).
        for n in 1..=nn {
            let expected = usize::from(((n + 1) as u32).is_power_of_two());
            assert_eq!(
                alg.cohomology_dimension(Bidegree::n_s(n, 0)),
                Some(expected),
                "Ext^0(RP^∞, k) at n = {n}",
            );
        }
    }

    /// dim C^s_t = Σ_i dim M_{t - d_i}.
    #[test]
    fn closed_form_dimension() {
        let (nn, ss) = (10, 6);
        let t_max = nn + ss;
        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        k_res.compute_through_bidegree(Bidegree::s_t(ss + 1, t_max));
        k_res.algebra().compute_basis(t_max + 1);
        let m_json = crate::utils::parse_module_name("C2").unwrap();
        let module = Arc::new(FDModule::from_json(k_res.algebra(), &m_json).unwrap());
        module.compute_basis(t_max);

        let diff = TensorResolutionDifferential::new(Arc::clone(&k_res), Arc::clone(&module));

        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                let t = b.t();
                let expected: usize = k_res
                    .module(s)
                    .iter_gens(t)
                    .map(|(d_i, _)| module.dimension(t - d_i))
                    .sum();
                assert_eq!(diff.dimension(b), Some(expected), "dim C^{s}_{t} at {b:?}");
            }
        }
    }

    /// [`finite_module`], for `heavy_tests`.
    pub(crate) fn finite_module_pub(
        algebra: &Arc<SteenrodAlgebra>,
        name: &str,
        t_max: i32,
    ) -> Arc<FDModule<SteenrodAlgebra>> {
        finite_module(algebra, name, t_max)
    }

    /// The finite module `name` over `algebra`, basis computed through `t_max`.
    fn finite_module(
        algebra: &Arc<SteenrodAlgebra>,
        name: &str,
        t_max: i32,
    ) -> Arc<FDModule<SteenrodAlgebra>> {
        let m = Arc::new(
            FDModule::from_json(
                Arc::clone(algebra),
                &crate::utils::parse_module_name(name).unwrap(),
            )
            .unwrap(),
        );
        m.compute_basis(t_max);
        m
    }

    /// ∂ ∘ ∂ = 0 on every generator of Q_• = P_• ⊗ C2 over a box.
    #[test]
    fn tensor_resolution_is_a_complex() {
        use algebra::module::homomorphism::ModuleHomomorphism;

        let (ss, t_max) = (6, 16);
        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let m = finite_module(&k_res.algebra(), "C2", t_max);
        let q = TensorResolution::new(Arc::clone(&k_res), m);
        q.compute_through_bidegree(Bidegree::s_t(ss, t_max));

        for s in 2..=ss {
            let d_s = q.differential(s);
            let d_prev = q.differential(s - 1);
            let q_s = q.module(s);
            let q_prev2 = q.module(s - 2);
            for t in 0..=t_max {
                for i in 0..q_s.number_of_gens_in_degree(t) {
                    let dx = d_s.output(t, i); // ∂(gen) ∈ Q_{s-1} at t
                    let mut ddx = FpVector::new(TWO, q_prev2.dimension(t));
                    d_prev.apply(ddx.as_slice_mut(), 1, t, dx.as_slice());
                    assert!(ddx.is_zero(), "∂² ≠ 0 at s = {s}, t = {t}, gen {i}");
                }
            }
        }
    }

    /// The materialised Q_•, through [`DualizedDifferential`], computes the same Ext(C2, k) as the
    /// closed form and the direct minimal resolution, and is non-minimal somewhere.
    #[test]
    fn q_complex_computes_ext_c2() {
        let (nn, ss) = (10, 6);
        let t_max = nn + ss;
        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        k_res.compute_through_bidegree(Bidegree::s_t(ss + 1, t_max));
        k_res.algebra().compute_basis(t_max + 1);
        let m = finite_module(&k_res.algebra(), "C2", t_max);

        let q = Arc::new(TensorResolution::new(Arc::clone(&k_res), Arc::clone(&m)));
        q.compute_through_bidegree(Bidegree::s_t(ss + 1, t_max));
        let e2 = ExtModule::intrinsic(Arc::clone(&q))
            .with_differential(Arc::new(DualizedDifferential::new(Arc::clone(&q))));

        let closed = tensor_resolution_ext(
            Arc::new(ExtAlgebra::new(Arc::clone(&k_res))),
            Arc::clone(&m),
        );
        let direct = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        direct.compute_through_stem(Bidegree::n_s(nn, ss));

        let mut saw_nonminimal = false;
        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                let d = direct.number_of_gens_in_bidegree(b);
                assert_eq!(
                    e2.cohomology_dimension(b),
                    Some(d),
                    "Q• Ext vs direct at {b:?}"
                );
                assert_eq!(
                    e2.cohomology_dimension(b),
                    closed.cohomology_dimension(b),
                    "Q• Ext vs closed form at {b:?}"
                );
                saw_nonminimal |= e2.cochain_dimension(b) > d;
            }
        }
        assert!(
            saw_nonminimal,
            "expected a non-minimal bidegree (cochains > cohomology)"
        );
    }

    /// With M = k, Q_• collapses to the minimal P_• (χ(a) acts as ε(a)), so d2 must be the
    /// standard Adams d2: d2(h4) = h0 h3², with the unit, h0, h1 and h2 surviving.
    #[test]
    fn tensor_d2_reproduces_sphere() {
        use sseq::coordinates::BidegreeGenerator;

        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let m = finite_module(&k_res.algebra(), "S_2", 24);
        let sec = TensorResolutionSecondary::new(k_res, m);
        sec.compute_through_stem(Bidegree::n_s(16, 4));

        for (n, s) in [(0, 0), (0, 1), (1, 1), (3, 1)] {
            let b = Bidegree::n_s(n, s);
            let h = sec.ext().generator(BidegreeGenerator::new(b, 0));
            assert_eq!(
                sec.survives(&h),
                Some(true),
                "h at (n = {n}, s = {s}) should survive d2"
            );
        }

        let h4 = sec
            .ext()
            .generator(BidegreeGenerator::new(Bidegree::n_s(15, 1), 0));
        let d = sec.d2(&h4).expect("d2(h4) target should be computed");
        assert_eq!(d.degree(), Bidegree::n_s(14, 3));
        assert_eq!(sec.cohomology_dimension(Bidegree::n_s(14, 3)), Some(1));
        assert!(!d.vec().is_zero(), "d2(h4) = h0 h3² should be nonzero");
        assert_eq!(sec.survives(&h4), Some(false), "h4 should not survive d2");
    }

    /// On the non-minimal Q_• = P_• ⊗ C2, d2 agrees with the direct minimal resolution's.
    ///
    /// The two E2 bases differ, so this compares E2 dimensions and the basis-independent rank of
    /// d2 out of each bidegree.
    #[test]
    fn tensor_d2_matches_direct_c2() {
        use fp::matrix::Matrix;
        use sseq::coordinates::BidegreeGenerator;

        use crate::ext_algebra::SecondaryExtAlgebra;

        let (nn, ss) = (12, 6);

        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let m = finite_module(&k_res.algebra(), "C2", nn + ss + 8);
        let tensor = TensorResolutionSecondary::new(k_res, m);
        tensor.compute_through_stem(Bidegree::n_s(nn, ss));

        let direct_res = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        direct_res.compute_through_stem(Bidegree::n_s(nn + 1, ss + 3));
        let direct_e2 = Arc::new(ExtModule::new(
            Arc::clone(&direct_res),
            Arc::new(ExtAlgebra::new(Arc::clone(&direct_res))),
        ));
        let direct_sec = SecondaryExtAlgebra::new(Arc::clone(&direct_e2));
        direct_sec.extend_all();

        let rank_of =
            |dim: usize, target_dim: usize, d2_of: &mut dyn FnMut(usize) -> FpVector| -> usize {
                if dim == 0 || target_dim == 0 {
                    return 0;
                }
                let rows: Vec<FpVector> = (0..dim).map(&mut *d2_of).collect();
                Matrix::from_rows(TWO, rows, target_dim).row_reduce()
            };

        let mut compared = 0;
        for n in 1..=nn {
            for s in 1..=ss {
                let b = Bidegree::n_s(n, s);
                let target = b + Bidegree::n_s(-1, 2);
                let fd = tensor
                    .cohomology_dimension(b)
                    .expect("computed in the compared range");
                let ftd = tensor
                    .cohomology_dimension(target)
                    .expect("computed in the compared range");
                assert_eq!(fd, direct_e2.dimension(b), "E2 dim mismatch at {b:?}");
                assert_eq!(
                    ftd,
                    direct_e2.dimension(target),
                    "E2 dim mismatch at target {target:?}"
                );

                let tensor_rank = rank_of(fd, ftd, &mut |i| {
                    tensor
                        .d2(&tensor.ext().generator(BidegreeGenerator::new(b, i)))
                        .map(BidegreeElement::into_vec)
                        .expect("computed in the compared range")
                });
                let direct_rank = rank_of(fd, ftd, &mut |i| {
                    direct_sec
                        .d2(&direct_e2.generator(BidegreeGenerator::new(b, i)))
                        .map(BidegreeElement::into_vec)
                        .expect("computed in the compared range")
                });
                assert_eq!(tensor_rank, direct_rank, "d2 rank mismatch at {b:?}");
                compared += 1;
            }
        }
        assert!(compared > 0, "no bidegrees compared");
    }

    /// The Ext(k, k)-module structure on Ext(C2, k) agrees with the direct minimal resolution's.
    ///
    /// The two Ext(C2, k) bases differ, so this compares Ext dimensions and the basis-independent
    /// rank of multiplication by `h0`. Both sides share one ring, so `h0` is the same class.
    #[test]
    fn tensor_products_match_direct_c2() {
        use sseq::coordinates::BidegreeGenerator;

        use super::tensor_resolution_ext;

        let (nn, ss) = (8, 5);
        let t_max = nn + ss + 6;

        let s2 = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        s2.compute_through_bidegree(Bidegree::s_t(ss + 3, t_max));
        s2.algebra().compute_basis(t_max + 1);
        let m = finite_module(&s2.algebra(), "C2", t_max);

        let tensor =
            tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&s2))), Arc::clone(&m));
        tensor
            .resolution()
            .compute_through_bidegree(Bidegree::s_t(ss + 3, t_max));

        // Direct side: minimal resolution of C2, over the same ring.
        let c2 = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        c2.compute_through_stem(Bidegree::n_s(nn + 1, ss + 3));
        let direct = ExtModule::new(Arc::clone(&c2), Arc::new(ExtAlgebra::new(Arc::clone(&s2))));
        // The tensor module is built on the resolution of `k`, but is not `k`.
        assert!(!tensor.is_unit());

        let h0 = BidegreeGenerator::new(Bidegree::n_s(0, 1), 0);
        let tensor_h0 = tensor.algebra().generator(h0);
        let direct_h0 = direct.algebra().generator(h0);

        let rank_of =
            |dim: usize, tdim: usize, f: &mut dyn FnMut(usize) -> Option<FpVector>| -> usize {
                if dim == 0 || tdim == 0 {
                    return 0;
                }
                let rows: Vec<FpVector> = (0..dim)
                    .map(|i| f(i).expect("computed in the compared range"))
                    .collect();
                Matrix::from_rows(TWO, rows, tdim).row_reduce()
            };

        let mut compared = 0;
        let mut saw_nonzero = false;
        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                let target = b + h0.degree();
                let fd = tensor
                    .cohomology_dimension(b)
                    .expect("computed in the compared range");
                let ftd = tensor
                    .cohomology_dimension(target)
                    .expect("computed in the compared range");
                assert_eq!(fd, direct.dimension(b), "Ext dim mismatch at {b:?}");
                assert_eq!(
                    ftd,
                    direct.dimension(target),
                    "Ext dim mismatch at target {target:?}"
                );

                let tensor_rank = rank_of(fd, ftd, &mut |i| {
                    tensor
                        .try_multiply(&tensor.generator(BidegreeGenerator::new(b, i)), &tensor_h0)
                        .map(BidegreeElement::into_vec)
                });
                let direct_rank = rank_of(fd, ftd, &mut |i| {
                    direct
                        .try_multiply(&direct.generator(BidegreeGenerator::new(b, i)), &direct_h0)
                        .map(BidegreeElement::into_vec)
                });
                assert_eq!(tensor_rank, direct_rank, "(·h0) rank mismatch at {b:?}");
                saw_nonzero |= tensor_rank > 0;
                compared += 1;
            }
        }
        assert!(compared > 0, "no bidegrees compared");
        assert!(saw_nonzero, "expected nonzero h0-multiplication somewhere");
    }

    /// As [`tensor_products_match_direct_c2`], for the infinite module `RP^∞`.
    ///
    /// `RealProjectiveSpace::act_on_basis` returns early on `op_degree == 0` where
    /// `FDModule::act_on_basis` does not, so a closed form that reaches the identity component
    /// through `act_on_basis` fails here while passing on `C2`.
    #[test]
    fn tensor_products_match_direct_rp_inf() {
        use algebra::module::RealProjectiveSpace;
        use sseq::coordinates::BidegreeGenerator;

        use super::tensor_resolution_ext;

        let (nn, ss) = (8, 4);
        let t_max = nn + ss + 6;

        let s2 = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        s2.compute_through_bidegree(Bidegree::s_t(ss + 3, t_max));
        s2.algebra().compute_basis(t_max + 1);
        let rp = Arc::new(RealProjectiveSpace::new(s2.algebra(), 1, None, false));
        rp.compute_basis(t_max);

        let tensor =
            tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&s2))), Arc::clone(&rp));
        tensor
            .resolution()
            .compute_through_bidegree(Bidegree::s_t(ss + 3, t_max));

        let rp_direct = Arc::new(construct_standard::<false, _, _>("RP_inf", None).unwrap());
        rp_direct.compute_through_stem(Bidegree::n_s(nn + 1, ss + 3));
        let direct = ExtModule::new(
            Arc::clone(&rp_direct),
            Arc::new(ExtAlgebra::new(Arc::clone(&s2))),
        );

        let h0 = BidegreeGenerator::new(Bidegree::n_s(0, 1), 0);
        let tensor_h0 = tensor.algebra().generator(h0);
        let direct_h0 = direct.algebra().generator(h0);

        let rank_of =
            |dim: usize, tdim: usize, f: &mut dyn FnMut(usize) -> Option<FpVector>| -> usize {
                if dim == 0 || tdim == 0 {
                    return 0;
                }
                let rows: Vec<FpVector> = (0..dim)
                    .map(|i| f(i).expect("computed in the compared range"))
                    .collect();
                Matrix::from_rows(TWO, rows, tdim).row_reduce()
            };

        let mut saw_nonzero = false;
        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                let target = b + h0.degree();
                let fd = tensor
                    .cohomology_dimension(b)
                    .expect("computed in the compared range");
                let ftd = tensor
                    .cohomology_dimension(target)
                    .expect("computed in the compared range");
                assert_eq!(fd, direct.dimension(b), "Ext dim mismatch at {b:?}");
                assert_eq!(
                    ftd,
                    direct.dimension(target),
                    "Ext dim mismatch at target {target:?}"
                );

                let tensor_rank = rank_of(fd, ftd, &mut |i| {
                    tensor
                        .try_multiply(&tensor.generator(BidegreeGenerator::new(b, i)), &tensor_h0)
                        .map(BidegreeElement::into_vec)
                });
                let direct_rank = rank_of(fd, ftd, &mut |i| {
                    direct
                        .try_multiply(&direct.generator(BidegreeGenerator::new(b, i)), &direct_h0)
                        .map(BidegreeElement::into_vec)
                });
                assert_eq!(tensor_rank, direct_rank, "(·h0) rank mismatch at {b:?}");
                saw_nonzero |= tensor_rank > 0;
            }
        }
        // `Ext(RP^∞, k)` has an infinite h0-tower off every `Ext^0` class, so a build that drops
        // the identity component of the cup product would leave every rank at zero.
        assert!(saw_nonzero, "expected nonzero h0-multiplication somewhere");
    }

    /// As [`tensor_products_match_direct_c2`], with `P•` resolved by Nassau's algorithm over the
    /// Milnor basis, the `P•` a large computation loads from disk.
    ///
    /// The other tests use the standard resolver; this one covers the antipode, coproduct and cup
    /// product over the Milnor basis.
    #[test]
    fn tensor_products_match_direct_c2_nassau() {
        use sseq::coordinates::BidegreeGenerator;

        use super::tensor_resolution_ext;

        let (nn, ss) = (8, 5);
        let t_max = nn + ss + 6;

        // Nassau keeps its quasi-inverses on disk, not in memory, and the chain maps behind
        // products lift through `P•` by `apply_quasi_inverse`, so this needs a save directory.
        let s2_dir = tempfile::tempdir().unwrap();
        let s2 = Arc::new(
            crate::utils::construct_nassau("S_2", Some(s2_dir.path().to_owned())).unwrap(),
        );
        s2.compute_through_bidegree(Bidegree::s_t(ss + 3, t_max));
        s2.algebra().compute_basis(t_max + 1);
        let m = Arc::new(
            FDModule::from_json(
                s2.algebra(),
                &crate::utils::parse_module_name("C2").unwrap(),
            )
            .unwrap(),
        );
        m.compute_basis(t_max);
        let tensor =
            tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&s2))), Arc::clone(&m));
        tensor
            .resolution()
            .compute_through_bidegree(Bidegree::s_t(ss + 3, t_max));

        // Direct side: minimal resolution of C2 (over `SteenrodAlgebra`) as ground truth.
        let c2 = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        c2.compute_through_stem(Bidegree::n_s(nn + 1, ss + 3));
        let s2_std = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        s2_std.compute_through_bidegree(Bidegree::s_t(ss + 3, t_max));
        let direct = ExtModule::new(Arc::clone(&c2), Arc::new(ExtAlgebra::new(s2_std)));

        let h0 = BidegreeGenerator::new(Bidegree::n_s(0, 1), 0);
        let tensor_h0 = tensor.algebra().generator(h0);
        let direct_h0 = direct.algebra().generator(h0);

        let rank_of =
            |dim: usize, tdim: usize, f: &mut dyn FnMut(usize) -> Option<FpVector>| -> usize {
                if dim == 0 || tdim == 0 {
                    return 0;
                }
                let rows: Vec<FpVector> = (0..dim)
                    .map(|i| f(i).expect("computed in the compared range"))
                    .collect();
                Matrix::from_rows(TWO, rows, tdim).row_reduce()
            };

        let mut compared = 0;
        let mut saw_nonzero = false;
        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                let target = b + h0.degree();
                let fd = tensor
                    .cohomology_dimension(b)
                    .expect("computed in the compared range");
                let ftd = tensor
                    .cohomology_dimension(target)
                    .expect("computed in the compared range");
                assert_eq!(fd, direct.dimension(b), "Ext dim mismatch at {b:?}");
                assert_eq!(
                    ftd,
                    direct.dimension(target),
                    "Ext dim mismatch at {target:?}"
                );

                let tensor_rank = rank_of(fd, ftd, &mut |i| {
                    tensor
                        .try_multiply(&tensor.generator(BidegreeGenerator::new(b, i)), &tensor_h0)
                        .map(BidegreeElement::into_vec)
                });
                let direct_rank = rank_of(fd, ftd, &mut |i| {
                    direct
                        .try_multiply(&direct.generator(BidegreeGenerator::new(b, i)), &direct_h0)
                        .map(BidegreeElement::into_vec)
                });
                assert_eq!(tensor_rank, direct_rank, "(·h0) rank mismatch at {b:?}");
                saw_nonzero |= tensor_rank > 0;
                compared += 1;
            }
        }
        assert!(compared > 0, "no bidegrees compared");
        assert!(saw_nonzero, "expected nonzero h0-multiplication somewhere");
    }

    /// The Massey families ⟨h0, h1, -⟩, ⟨-, h0, h1·ι⟩ and ⟨h2², h0, -⟩ on Ext(C2, k) agree with
    /// the direct minimal resolution's.
    ///
    /// Families are compared as the sorted bracket bidegrees with their indeterminacy dimensions,
    /// which do not depend on the Ext bases.
    #[test]
    fn tensor_massey_dga_family_matches_direct_c2() {
        use sseq::coordinates::BidegreeGenerator;

        use crate::ext_algebra::massey::MasseyResult;

        let (nn, ss) = (8, 5);
        let s2 = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let m = finite_module(&s2.algebra(), "C2", nn + ss + 8);
        let tensor = tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&s2))), m);
        tensor
            .resolution()
            .compute_through_bidegree(Bidegree::n_s(nn + 2, ss + 3));

        let c2 = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        c2.compute_through_stem(Bidegree::n_s(nn + 1, ss + 3));
        let direct = ExtModule::new(Arc::clone(&c2), Arc::new(ExtAlgebra::new(Arc::clone(&s2))));

        let h0 = tensor
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let h1 = tensor
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));
        let d_h0 = direct
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let d_h1 = direct
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));

        // Restricted to the range both resolutions cover.
        let summarize = |family: Vec<(BidegreeElement, MasseyResult)>| -> Vec<(String, usize)> {
            let mut keyed: Vec<(String, usize)> = family
                .into_iter()
                .filter(|(_, r)| r.degree.n() <= nn && r.degree.s() <= ss)
                .map(|(_, r)| (format!("{:?}", r.degree), r.coset.linear_part().dimension()))
                .collect();
            keyed.sort();
            keyed
        };

        let tensor_fam = summarize(tensor.massey_iter_c(&h0, &h1));
        let direct_fam = summarize(direct.massey_iter_c(&d_h0, &d_h1));
        assert!(!tensor_fam.is_empty(), "expected some nonzero brackets");
        assert_eq!(
            tensor_fam, direct_fam,
            "⟨h0, h1, -⟩ over C2 disagrees tensor (DGA) vs direct"
        );

        // Sweeping the first factor: ⟨-, h0, h1·ι⟩ for the bottom class ι of Ext(C2, k). The
        // tensor side solves δv = h0 ∪ h1ι once and reads each bracket as [a ∪ v + c ∘ (H ⊗ 1)],
        // with H a null-homotopy of f_{h0} ∘ f_a.
        let iota = tensor.generator(BidegreeGenerator::new(Bidegree::n_s(0, 0), 0));
        let d_iota = direct.generator(BidegreeGenerator::new(Bidegree::n_s(0, 0), 0));
        let c = tensor.multiply(&iota, &h1);
        let d_c = direct.multiply(&d_iota, &d_h1);
        assert!(!c.vec().is_zero(), "h1·ι should be nonzero in Ext(C2, k)");
        let tensor_fam = summarize(tensor.massey_iter_a(&h0, &c));
        let direct_fam = summarize(direct.massey_iter_a(&d_h0, &d_c));
        assert!(!tensor_fam.is_empty(), "expected some nonzero brackets");
        assert_eq!(
            tensor_fam, direct_fam,
            "⟨-, h0, h1·ι⟩ over C2 disagrees tensor (DGA) vs direct"
        );

        // ⟨h2², h0, h1·ι⟩ ≠ 0, yet h0 ∪ h1ι vanishes on cochains, so v = 0 and the whole bracket
        // is the term c ∘ (H ⊗ 1) of the null-homotopy H of f_{h0} ∘ f_{h2²}.
        let h2 = BidegreeGenerator::new(Bidegree::n_s(3, 1), 0);
        let h2_sq = tensor.algebra().multiply(
            &tensor.algebra().generator(h2),
            &tensor.algebra().generator(h2),
        );
        let d_h2_sq = direct.algebra().multiply(
            &direct.algebra().generator(h2),
            &direct.algebra().generator(h2),
        );
        let bracket = tensor
            .massey(&h2_sq, &h0, &c)
            .expect("⟨h2², h0, h1·ι⟩ is defined");
        let d_bracket = direct
            .massey(&d_h2_sq, &d_h0, &d_c)
            .expect("⟨h2², h0, h1·ι⟩ is defined");
        assert!(!d_bracket.contains_zero());
        assert!(!bracket.contains_zero(), "⟨h2², h0, h1·ι⟩ lost its H term");
        assert_eq!(
            summarize(tensor.massey_iter_c(&h2_sq, &h0)),
            summarize(direct.massey_iter_c(&d_h2_sq, &d_h0)),
            "⟨h2², h0, -⟩ over C2 disagrees tensor (DGA) vs direct"
        );
    }

    /// `project(h0 ∪ lift(y)) == y · h0` for every class y of Ext(C2, k), through
    /// [`CochainCup::cup_matrix`] directly.
    ///
    /// This checks the cup product apart from the rest of the Massey bracket.
    #[test]
    fn tensor_massey_dga_cup_matches_products() {
        use sseq::coordinates::BidegreeGenerator;

        use crate::resolution_homomorphism::ResolutionHomomorphism;

        let (nn, ss) = (8, 5);
        let s2 = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let m = finite_module(&s2.algebra(), "C2", nn + ss + 8);
        let tensor = tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&s2))), m);
        tensor
            .resolution()
            .compute_through_bidegree(Bidegree::n_s(nn + 2, ss + 3));
        let h0 = tensor
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));

        let cup = tensor
            .cup()
            .expect("the untwisted tensor resolution attaches a cup product");
        let f_h0 = ResolutionHomomorphism::from_class(
            String::new(),
            Arc::clone(tensor.algebra().resolution()),
            Arc::clone(tensor.algebra().resolution()),
            h0.degree(),
            &[1],
        );

        let mut checked = 0;
        for n in 0..=nn {
            for s in 0..=ss {
                let b = Bidegree::n_s(n, s);
                let target = b + h0.degree();
                if tensor.cohomology(b).is_none() || tensor.cohomology(target).is_none() {
                    continue;
                }
                f_h0.extend_through_stem(target);
                let m_cup = cup.cup_matrix(&f_h0.get_map(target.s()), b, target);
                for i in 0..tensor.dimension(b) {
                    let y = tensor.generator(BidegreeGenerator::new(b, i));
                    let mut out = FpVector::new(TWO, m_cup.columns());
                    m_cup.apply(out.as_slice_mut(), 1, tensor.lift(&y).vec());
                    let via_cup = tensor.project(&super::Cochain::new(target, out));
                    let via_mult = tensor.multiply(&y, &h0);
                    assert_eq!(via_cup, via_mult, "h0 ∪ y != y · h0 at {b:?}#{i}");
                    checked += 1;
                }
            }
        }
        assert!(checked > 0, "no cup products checked");
    }

    /// ⟨h0, h1, c⟩ for the class c at (2, 2) of Ext(C2, k) agrees with the direct minimal
    /// resolution's, and is non-zero.
    #[test]
    fn tensor_massey_dga_matches_direct_c2() {
        use sseq::coordinates::BidegreeGenerator;

        let (nn, ss) = (8, 5);
        let s2 = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let m = finite_module(&s2.algebra(), "C2", nn + ss + 8);
        let tensor = tensor_resolution_ext(Arc::new(ExtAlgebra::new(Arc::clone(&s2))), m);
        tensor
            .resolution()
            .compute_through_bidegree(Bidegree::n_s(nn + 2, ss + 3));

        let c2 = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        c2.compute_through_stem(Bidegree::n_s(nn + 1, ss + 3));
        let direct = ExtModule::new(Arc::clone(&c2), Arc::new(ExtAlgebra::new(Arc::clone(&s2))));

        let h0 = tensor
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let h1 = tensor
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));
        let d_h0 = direct
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(0, 1), 0));
        let d_h1 = direct
            .algebra()
            .generator(BidegreeGenerator::new(Bidegree::n_s(1, 1), 0));

        let cd = Bidegree::n_s(2, 2);
        let c_tensor = tensor.generator(BidegreeGenerator::new(cd, 0));
        let c_direct = direct.generator(BidegreeGenerator::new(cd, 0));
        let tb = tensor
            .massey(&h0, &h1, &c_tensor)
            .expect("tensor ⟨h0,h1,c⟩ defined");
        let db = direct
            .massey(&d_h0, &d_h1, &c_direct)
            .expect("direct ⟨h0,h1,c⟩ defined");
        assert_eq!(tb.degree, db.degree, "bracket bidegree");
        assert_eq!(
            tb.coset.linear_part().dimension(),
            db.coset.linear_part().dimension(),
            "indeterminacy dimension"
        );
        // The bracket's bidegree is 1-dimensional with zero indeterminacy, so the representative's
        // vanishing is basis-free.
        assert_eq!(
            tb.coset.offset().is_zero(),
            db.coset.offset().is_zero(),
            "⟨h0,h1,c⟩ vanishing disagrees: tensor={:?} direct={:?}",
            tb.coset.offset(),
            db.coset.offset()
        );
        assert!(
            !tb.coset.offset().is_zero(),
            "tensor ⟨h0,h1,c⟩ should be nonzero"
        );
    }

    /// As [`tensor_resolution_is_a_complex`], for `RP^∞`.
    ///
    /// Its non-trivial higher action exercises every term of ∂_Q, including the identity term
    /// that `RealProjectiveSpace::act_on_basis` drops.
    #[test]
    fn tensor_resolution_is_a_complex_rp_inf() {
        use algebra::module::{RealProjectiveSpace, homomorphism::ModuleHomomorphism};

        let (ss, t_max) = (7, 20);
        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let rp = Arc::new(RealProjectiveSpace::new(k_res.algebra(), 1, None, false));
        rp.compute_basis(t_max);
        let q = TensorResolution::new(Arc::clone(&k_res), rp);
        q.compute_through_bidegree(Bidegree::s_t(ss, t_max));

        for s in 2..=ss {
            let d_s = q.differential(s);
            let d_prev = q.differential(s - 1);
            let q_s = q.module(s);
            let q_prev2 = q.module(s - 2);
            for t in 0..=t_max {
                for i in 0..q_s.number_of_gens_in_degree(t) {
                    let dx = d_s.output(t, i);
                    let mut ddx = FpVector::new(TWO, q_prev2.dimension(t));
                    d_prev.apply(ddx.as_slice_mut(), 1, t, dx.as_slice());
                    assert!(ddx.is_zero(), "∂² ≠ 0 at s = {s}, t = {t}, gen {i}");
                }
            }
        }
    }

    /// As [`tensor_d2_matches_direct_c2`], for `RP^∞`.
    #[test]
    #[ignore = "blocked by the module, not by non-minimality: H*(RP^∞) admits several secondary \
                (B-module) structures, and the secondary machinery assumes the `zero` one, which \
                does not lift for RP^∞ (see secondary_zero_structure_fails_for_rp_inf; the direct \
                minimal resolution fails the same way). Finding the geometrically correct \
                secondary structure of RP^∞ is separate theoretical work."]
    fn tensor_d2_matches_direct_rp_inf() {
        use algebra::module::RealProjectiveSpace;
        use fp::matrix::Matrix;
        use sseq::coordinates::BidegreeGenerator;

        use crate::ext_algebra::SecondaryExtAlgebra;

        let (nn, ss) = (10, 5);

        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let rp = Arc::new(RealProjectiveSpace::new(k_res.algebra(), 1, None, false));
        rp.compute_basis(nn + ss + 8);
        let tensor = TensorResolutionSecondary::new(k_res, rp);
        tensor.compute_through_stem(Bidegree::n_s(nn, ss));

        let direct_res = Arc::new(construct_standard::<false, _, _>("RP_inf", None).unwrap());
        direct_res.compute_through_stem(Bidegree::n_s(nn + 1, ss + 3));
        let direct_e2 = Arc::new(ExtModule::new(
            Arc::clone(&direct_res),
            Arc::new(ExtAlgebra::new(Arc::clone(&direct_res))),
        ));
        let direct_sec = SecondaryExtAlgebra::new(Arc::clone(&direct_e2));
        direct_sec.extend_all();

        let rank_of =
            |dim: usize, target_dim: usize, d2_of: &mut dyn FnMut(usize) -> FpVector| -> usize {
                if dim == 0 || target_dim == 0 {
                    return 0;
                }
                let rows: Vec<FpVector> = (0..dim).map(&mut *d2_of).collect();
                Matrix::from_rows(TWO, rows, target_dim).row_reduce()
            };

        let mut compared = 0;
        for n in 1..=nn {
            for s in 1..=ss {
                let b = Bidegree::n_s(n, s);
                let target = b + Bidegree::n_s(-1, 2);
                let fd = tensor
                    .cohomology_dimension(b)
                    .expect("computed in the compared range");
                let ftd = tensor
                    .cohomology_dimension(target)
                    .expect("computed in the compared range");
                assert_eq!(fd, direct_e2.dimension(b), "E2 dim mismatch at {b:?}");
                assert_eq!(
                    ftd,
                    direct_e2.dimension(target),
                    "E2 dim mismatch at target {target:?}"
                );

                let tensor_rank = rank_of(fd, ftd, &mut |i| {
                    tensor
                        .d2(&tensor.ext().generator(BidegreeGenerator::new(b, i)))
                        .map(BidegreeElement::into_vec)
                        .expect("computed in the compared range")
                });
                let direct_rank = rank_of(fd, ftd, &mut |i| {
                    direct_sec
                        .d2(&direct_e2.generator(BidegreeGenerator::new(b, i)))
                        .map(BidegreeElement::into_vec)
                        .expect("computed in the compared range")
                });
                assert_eq!(tensor_rank, direct_rank, "d2 rank mismatch at {b:?}");
                compared += 1;
            }
        }
        assert!(compared > 0, "no bidegrees compared");
    }

    /// The zero secondary structure on H*(RP^∞) does not lift, even on its direct minimal
    /// resolution, so the failure of [`tensor_d2_matches_direct_rp_inf`] is a property of `RP^∞`.
    ///
    /// Drives the secondary homotopies as `compute_homotopies` does but stops before the failing
    /// step, then runs that step through the fallible path and checks it reports a lift failure
    /// rather than producing a wrong d2 (as `secondary::cofib_h4` does).
    #[test]
    fn secondary_zero_structure_fails_for_rp_inf() {
        use crate::secondary::SecondaryLift;

        let res = Arc::new(construct_standard::<false, _, _>("RP_inf", None).unwrap());
        res.compute_through_stem(Bidegree::n_s(12, 6));
        let lift = crate::secondary::SecondaryResolution::new(Arc::clone(&res));

        let failing = Bidegree::n_s(10, 3);

        lift.initialize_homotopies();
        lift.compute_composites();
        lift.compute_intermediates();
        let shift = lift.shift();
        {
            let h = &lift.homotopies()[shift.s()];
            h.homotopies.extend_by_zero(h.composites.max_degree());
        }
        let min_t = lift.homotopies()[shift.s()].homotopies.min_degree();
        let s_range = lift.homotopies().range();
        let min = Bidegree::s_t(s_range.start + 1, min_t);
        let max = lift.max().restrict(s_range.end);
        sseq::coordinates::iter_s_t(
            &|b| {
                if b.s() > failing.s() || (b.s() == failing.s() && b.t() >= failing.t()) {
                    return b.t()..b.t() + 1;
                }
                lift.compute_homotopy_step(b)
            },
            min,
            max,
        );

        let result = lift.try_compute_homotopy_step(failing);
        assert!(
            result.is_err(),
            "expected RP^∞'s zero secondary structure to fail to lift"
        );
        assert!(
            result.unwrap_err().to_string().contains("Failed to lift"),
            "expected a lift failure at {failing}"
        );
    }
}

#[cfg(test)]
mod heavy_tests {
    use std::{sync::Arc, time::Instant};

    use fp::{matrix::Matrix, prime::TWO, vector::FpVector};
    use sseq::coordinates::{Bidegree, BidegreeElement, BidegreeGenerator};

    use super::{TensorResolutionSecondary, tests::finite_module_pub};
    use crate::{
        chain_complex::ChainComplex,
        ext_algebra::{ExtAlgebra, ExtModule, SecondaryExtAlgebra},
        utils::construct_standard,
    };

    /// As `tensor_d2_matches_direct_c2`, through a much larger range.
    ///
    /// Run with `cargo test --release -- --ignored tensor_d2_matches_direct_c2_stem60 --nocapture`.
    #[test]
    #[ignore = "heavy (~30 min with `--features concurrent`): untwisted tensor resolution C2 d2 vs \
                direct up to stem 60"]
    fn tensor_d2_matches_direct_c2_stem60() {
        let (nn, ss): (i32, i32) = (60, 18);
        let t0 = Instant::now();

        let k_res = Arc::new(construct_standard::<false, _, _>("S_2", None).unwrap());
        let m = finite_module_pub(&k_res.algebra(), "C2", nn + ss + 8);
        let tensor = TensorResolutionSecondary::new(k_res, m);
        tensor.compute_through_stem(Bidegree::n_s(nn, ss));
        eprintln!("tensor secondary computed in {:.1?}", t0.elapsed());

        let t1 = Instant::now();
        let direct_res = Arc::new(construct_standard::<false, _, _>("C2", None).unwrap());
        direct_res.compute_through_stem(Bidegree::n_s(nn + 1, ss + 3));
        let direct_e2 = Arc::new(ExtModule::new(
            Arc::clone(&direct_res),
            Arc::new(ExtAlgebra::new(Arc::clone(&direct_res))),
        ));
        let direct_sec = SecondaryExtAlgebra::new(Arc::clone(&direct_e2));
        direct_sec.extend_all();
        eprintln!("direct secondary computed in {:.1?}", t1.elapsed());

        let rank_of = |dim: usize, tdim: usize, f: &mut dyn FnMut(usize) -> FpVector| -> usize {
            if dim == 0 || tdim == 0 {
                return 0;
            }
            Matrix::from_rows(TWO, (0..dim).map(&mut *f).collect(), tdim).row_reduce()
        };

        let mut compared = 0;
        let mut nonzero_d2 = 0;
        let mut mismatches = 0;
        for n in 1..=nn {
            for s in 1..=ss {
                let b = Bidegree::n_s(n, s);
                let target = b + Bidegree::n_s(-1, 2);
                let fd = tensor
                    .cohomology_dimension(b)
                    .expect("computed in the compared range");
                let ftd = tensor
                    .cohomology_dimension(target)
                    .expect("computed in the compared range");
                if (fd, ftd) != (direct_e2.dimension(b), direct_e2.dimension(target)) {
                    eprintln!(
                        "E2 dim mismatch at {b:?} or its target: tensor {:?} direct {:?}",
                        (fd, ftd),
                        (direct_e2.dimension(b), direct_e2.dimension(target))
                    );
                    mismatches += 1;
                    continue;
                }
                let fr = rank_of(fd, ftd, &mut |i| {
                    tensor
                        .d2(&tensor.ext().generator(BidegreeGenerator::new(b, i)))
                        .map(BidegreeElement::into_vec)
                        .expect("computed in the compared range")
                });
                let dr = rank_of(
                    direct_e2.dimension(b),
                    direct_e2.dimension(target),
                    &mut |i| {
                        direct_sec
                            .d2(&direct_e2.generator(BidegreeGenerator::new(b, i)))
                            .map(BidegreeElement::into_vec)
                            .expect("computed in the compared range")
                    },
                );
                if fr != dr {
                    eprintln!("d2 RANK mismatch at {b:?}: tensor {fr} direct {dr}");
                    mismatches += 1;
                }
                if fr > 0 {
                    nonzero_d2 += 1;
                }
                compared += 1;
            }
        }
        eprintln!(
            "compared {compared} bidegrees, {nonzero_d2} with nonzero d2, {mismatches} mismatches"
        );
        assert_eq!(
            mismatches, 0,
            "untwisted tensor resolution d2 disagrees with direct somewhere"
        );
    }
}
