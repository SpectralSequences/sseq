use crate::chain_complex::{BoundedChainComplex, ChainComplex, ChainHomotopy};
use crate::resolution::Resolution as Resolution_;
use crate::utils::HashMapTuple;
use crate::CCC;
use algebra::combinatorics;
use algebra::milnor_algebra::{
    MilnorAlgebra as Algebra, MilnorBasisElement as MilnorElt, PPartAllocation, PPartMultiplier,
};
use algebra::module::homomorphism::FreeModuleHomomorphism;
use algebra::module::{BoundedModule, FreeModule, Module};
use algebra::{Algebra as _, MilnorAlgebraT, SteenrodAlgebra};
use fp::prime::ValidPrime;
use fp::vector::{FpVector, SliceMut};
use rustc_hash::FxHashMap as HashMap;
use std::cell::RefCell;
use std::hash::{BuildHasher, Hash, Hasher};

#[cfg(feature = "concurrent")]
use {
    bivec::BiVec,
    crossbeam_channel::{unbounded, RecvTimeoutError},
    saveload::{Load, Save},
    std::{
        io::{BufReader, BufWriter, Read, Write},
        path::Path,
        sync::{Arc, Mutex},
        time::Instant,
    },
    thread_token::TokenBucket,
};

type Resolution = Resolution_<CCC>;
type FMH = FreeModuleHomomorphism<FreeModule<SteenrodAlgebra>>;

const TWO: ValidPrime = ValidPrime::new(2);

/// Whether picking δ₂ = 0 gives a valid secondary refinement. This requires
///  1. The chain complex is concentrated in degree zero;
///  2. The module is finite dimensional; and
///  3. $\mathrm{Hom}(\mathrm{Ext}^{2, t}_A(H^*X, k), H^{t - 1} X) = 0$ for all $t$ or $\mathrm{Hom}(\mathrm{Ext}^{3, t}_A(H^*X, k), H^{t - 1} X) = 0$ for all $t$.
pub fn can_compute(res: &Resolution) -> bool {
    let complex = res.complex();
    if *complex.prime() != 2 {
        eprintln!("Prime is not 2");
        return false;
    }
    if complex.max_s() != 1 {
        eprintln!("Complex is not concentrated in degree 0.");
        return false;
    }
    let module = complex.module(0);
    let module = module.as_fd_module();
    if module.is_none() {
        eprintln!("Module is not finite dimensional");
        return false;
    }
    let module = module.unwrap();
    let max_degree = module.max_degree();

    (0..max_degree)
        .all(|t| module.dimension(t) == 0 || res.number_of_gens_in_bidegree(2, t + 1) == 0)
        || (0..max_degree)
            .all(|t| module.dimension(t) == 0 || res.number_of_gens_in_bidegree(3, t + 1) == 0)
}

/// An element in the Milnor algebra
struct MilnorClass {
    elements: Vec<MilnorElt>,
    degree: i32,
}

impl MilnorClass {
    #[cfg(test)]
    fn from_elements(elements: Vec<MilnorElt>) -> Self {
        let degree = elements.get(0).map(|x| x.degree).unwrap_or(0);

        Self { elements, degree }
    }

    fn from_module_row(
        vec: &FpVector,
        module: &FreeModule<SteenrodAlgebra>,
        degree: i32,
        gen_t: i32,
        gen_idx: usize,
    ) -> Self {
        let algebra = module.algebra();
        let algebra = algebra.milnor_algebra();

        let offset = module.generator_offset(degree, gen_t, gen_idx);

        let elements = algebra.basis_table[(degree - gen_t) as usize]
            .iter()
            .enumerate()
            .filter(|(i, _)| vec.entry(offset + i) != 0)
            .map(|(_, x)| x.clone())
            .collect();

        Self {
            elements,
            degree: degree - gen_t,
        }
    }

    fn iter(&self) -> impl Iterator<Item = &MilnorElt> {
        self.elements.iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut MilnorElt> {
        self.elements.iter_mut()
    }
}

/// A non-concurrent version for computing delta. In practice the concurrent version will be used,
/// and this function should have clear logic rather than being optimal.
pub fn compute_delta(res: &Resolution) -> Vec<FMH> {
    let max_s = res.next_homological_degree();
    if max_s < 3 {
        return vec![];
    }
    let deltas = ChainHomotopy::new(res, res, 3, 1, |s, t, i, result| {
        compute_c(res, s, t, i, result.as_slice_mut());
    });
    deltas.extend_all();
    deltas.into_homotopies().into_vec()
}

#[cfg(feature = "concurrent")]
fn read_saved_data(buffer: &mut impl Read) -> std::io::Result<(u32, i32, usize, FpVector)> {
    let s = u32::load(buffer, &())?;
    let t = i32::load(buffer, &())?;
    let idx = usize::load(buffer, &())?;
    let data = FpVector::load(buffer, &TWO)?;

    Ok((s, t, idx, data))
}

#[cfg(feature = "concurrent")]
pub fn compute_delta_concurrent(
    res: &Resolution,
    bucket: &TokenBucket,
    save_file_path: Option<String>,
) -> Vec<FMH> {
    let max_s = res.next_homological_degree();
    if max_s < 3 {
        return vec![];
    }
    let min_degree = res.min_degree();
    let max_t = |s| {
        1 + std::cmp::min(
            res.module(s).max_computed_degree(),
            res.module(s - 2).max_computed_degree() + 1,
        )
    };

    let ddeltas: Vec<BiVec<Vec<Option<FpVector>>>> = Vec::with_capacity(max_s as usize - 3);
    let ddeltas = Mutex::new(ddeltas);

    let start = Instant::now();
    crossbeam_utils::thread::scope(|scope| {
        // Pretty print progress of first step
        let mut processed: HashMap<(u32, i32), u32> = HashMap::default();

        for s in 3..max_s {
            let m = res.module(s);
            for t in min_degree + 1..max_t(s) {
                processed.insert((s, t), m.number_of_gens_in_degree(t) as u32);
            }
        }

        let (p_sender, p_receiver) = unbounded();
        scope.spawn(move |_| {
            let mut prev = Instant::now();
            // Clear first row
            eprint!("\x1b[2J");
            loop {
                match p_receiver.recv_timeout(std::time::Duration::from_secs(1)) {
                    Ok(data) => {
                        *processed.get_mut(&data).unwrap() -= 1;
                    }
                    Err(RecvTimeoutError::Timeout) => (),
                    Err(RecvTimeoutError::Disconnected) => break,
                }
                if prev.elapsed().as_millis() < 100 {
                    continue;
                }
                // Move cursor to beginning and clear line
                eprint!("\x1b[H\x1b[K");
                eprintln!(
                    "Time elapsed: {:.2?}; Processed bidegrees:",
                    start.elapsed()
                );
                crate::utils::print_resolution_color(res, max_s, &processed);
                // Clear the rest of the screen
                eprint!("\x1b[J");
                std::io::stdout().flush().unwrap();
                prev = Instant::now();
            }
        });

        // We now compute the A terms of dδg. There are no dependencies between the different
        // bidegrees, so we use a thread pool for this. The ddeltas store the results as a vector:
        // source_s -> source_t -> gen_idx -> value. Since they are not populated in order of source_s
        // and source_t, we pre-populate with None and replace with Some.
        for s in 3..max_s {
            let m = res.module(s);
            let max = max_t(s);
            let mut v = BiVec::with_capacity(min_degree + s as i32, max);
            for t in min_degree + s as i32..max {
                v.push(vec![None; m.number_of_gens_in_degree(t)]);
            }
            ddeltas.lock().unwrap().push(v);
        }

        if let Some(p) = save_file_path.as_ref() {
            if Path::new(&*p).exists() {
                let f = std::fs::File::open(&*p).unwrap();
                let mut f = BufReader::new(f);
                loop {
                    match read_saved_data(&mut f) {
                        Ok((s, t, idx, data)) => {
                            if s < max_s && t >= min_degree + s as i32 && t <= max_t(s) {
                                ddeltas.lock().unwrap()[s as usize - 3][t][idx] = Some(data);
                                p_sender.send((s, t)).unwrap();
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }

        let save_file = match save_file_path {
            None => Arc::new(None),
            Some(p) => {
                let f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&*p)
                    .unwrap();
                Arc::new(Some(Mutex::new(BufWriter::new(f))))
            }
        };

        let (sender, receiver) = unbounded::<(u32, i32, usize)>();
        let receiver = Arc::new(Mutex::new(receiver));

        // Redefine these to the borrows so that the underlying doesn't get moved into closures
        let ddeltas = &ddeltas;
        for _ in 0..bucket.max_threads.get() {
            let save_file = Arc::clone(&save_file);
            let receiver = Arc::clone(&receiver);
            let p_sender = p_sender.clone();
            scope.spawn(move |_| loop {
                let job = receiver.lock().unwrap().recv().ok();

                if let Some((s, t, idx)) = job {
                    if ddeltas.lock().unwrap()[s as usize - 3][t][idx].is_some() {
                        continue;
                    }
                    let target_dim = res.module(s - 3).dimension(t - 1);
                    let mut result = FpVector::new(TWO, target_dim);

                    compute_c(&*res, s, t, idx, result.as_slice_mut());

                    if let Some(save_file) = &*save_file {
                        let mut sf = save_file.lock().unwrap();
                        s.save(&mut *sf).unwrap();
                        t.save(&mut *sf).unwrap();
                        idx.save(&mut *sf).unwrap();
                        result.save(&mut *sf).unwrap();
                        sf.flush().unwrap();
                        drop(sf);
                    }

                    ddeltas.lock().unwrap()[s as usize - 3][t][idx] = Some(result);

                    p_sender.send((s, t)).unwrap();
                } else {
                    break;
                }
            });
        }

        // Iterate in reverse order to do the slower ones first
        for s in 3..max_s {
            for t in (min_degree + s as i32..max_t(s)).rev() {
                for idx in 0..res.module(s).number_of_gens_in_degree(t) {
                    sender.send((s, t, idx)).unwrap();
                }
            }
        }
    })
    .unwrap();

    eprintln!("Computed A terms in {:.2?}", start.elapsed());

    let ddeltas = &*ddeltas.lock().unwrap();

    let start = std::time::Instant::now();
    let deltas = ChainHomotopy::new(res, res, 3, 1, |s, t, i, result| {
        // If we are restoring old computations that used resolving up to a stem, then the ddeltas
        // may be shorter than the true ones.
        result.assign_partial(&ddeltas[s as usize - 3][t][i].as_ref().unwrap())
    });
    deltas.extend_all_concurrent(bucket);
    eprintln!("Computed δd terms in {:.2?}", start.elapsed());
    deltas.into_homotopies().into_vec()
}

/// Computes $C(g_i) = A(c_i^j, dd g_j)$.
fn compute_c(res: &Resolution, gen_s: u32, gen_t: i32, gen_idx: usize, mut result: SliceMut) {
    let m = res.module(gen_s - 1);

    let d = res.differential(gen_s);
    let dg = d.output(gen_t, gen_idx);

    for t in 0..gen_t {
        for idx in 0..m.number_of_gens_in_degree(t) {
            let mut a_list = MilnorClass::from_module_row(dg, &m, gen_t, t, idx);

            if !a_list.elements.is_empty() {
                compute_a_dd(res, &mut a_list, gen_s - 1, t, idx, result.copy());
            }
        }
    }
}

macro_rules! sub {
    ($elt:ident, $k:expr, $n:expr) => {
        if $k > 0 {
            if $elt.p_part[$k - 1] < (1 << $n) {
                continue;
            }
            $elt.p_part[$k - 1] -= 1 << $n;
            $elt.degree -= combinatorics::xi_degrees(TWO)[$k - 1] * (1 << $n);
        }
    };
}
macro_rules! unsub {
    ($elt:ident, $k:expr, $n:expr) => {
        if $k > 0 {
            $elt.p_part[$k - 1] += 1 << $n;
            $elt.degree += combinatorics::xi_degrees(TWO)[$k - 1] * (1 << $n);
        }
    };
}

/// Computes $A(a, ddg)$
fn compute_a_dd(
    res: &Resolution,
    a_list: &mut MilnorClass,
    gen_s: u32,
    gen_t: i32,
    gen_idx: usize,
    mut result: SliceMut,
) {
    let target_deg = a_list.degree + gen_t - 1;

    let algebra = res.algebra();
    let algebra = algebra.milnor_algebra();

    let d = res.differential(gen_s);
    let dg = d.output(gen_t, gen_idx);
    let differential_l = res.differential(gen_s - 1);

    let module_h = res.module(gen_s - 1);
    let module_l = res.module(gen_s - 2);

    // (gen_t, gen_idx, target_element) -> coefficient
    let mut coefs: HashMap<(i32, usize, MilnorElt), u32> = HashMap::default();

    let mut b = MilnorElt::default();
    let mut c = MilnorElt::default();

    // If R_1 = 0, then A(Sq(R), 2Sq(S)) = 0, so we don't have to compute the Sq terms of the
    // product.
    let process_two = a_list.iter().any(|x| x.p_part[0] > 0);
    let mut allocation = PPartAllocation::with_capacity(8);

    for (i, _) in dg.iter_nonzero() {
        let elt = module_h.index_to_op_gen(gen_t, i);
        algebra
            .basis_element_from_index(elt.operation_degree, elt.operation_index)
            .clone_into(&mut b);

        let ddg = differential_l.output(elt.generator_degree, elt.generator_index);
        for (j, _) in ddg.iter_nonzero() {
            let elt2 = module_l.index_to_op_gen(elt.generator_degree, j);
            algebra
                .basis_element_from_index(elt2.operation_degree, elt2.operation_index)
                .clone_into(&mut c);

            let offset =
                module_l.generator_offset(target_deg, elt2.generator_degree, elt2.generator_index);
            let num_ops = algebra.dimension(a_list.degree + b.degree + c.degree - 1, 0);

            // Compute the Y terms of the result
            a_sigma_y(
                algebra,
                a_list,
                &mut b,
                &mut c,
                result.slice_mut(offset, offset + num_ops),
            );

            // While the Y terms can be processed separately, we have to be careful with the Sq
            // terms. In the product Sq(S) Sq(T), there will be both terms that have odd
            // coefficients and those with even coefficients. After summing everything, we will be
            // left with only multiples of 2, but these can come from adding two terms with odd
            // coefficients. So we need to collect all coefficients and process them at the end.
            if process_two {
                let mut multiplier = PPartMultiplier::<true>::new_from_allocation(
                    TWO,
                    &b.p_part,
                    &c.p_part,
                    allocation,
                    0,
                    b.degree + c.degree,
                );
                while let Some(c_) = multiplier.next() {
                    let mut hasher = coefs.hasher().build_hasher();
                    elt2.generator_degree.hash(&mut hasher);
                    elt2.generator_index.hash(&mut hasher);
                    multiplier.ans.hash(&mut hasher);
                    let entry = coefs.raw_entry_mut().from_hash(hasher.finish(), |v| {
                        v.0 == elt2.generator_degree
                            && v.1 == elt2.generator_index
                            && v.2 == multiplier.ans
                    });

                    entry
                        .and_modify(|_k, v| *v = (*v + c_) % 4)
                        .or_insert_with(|| {
                            (
                                (
                                    elt2.generator_degree,
                                    elt2.generator_index,
                                    multiplier.ans.clone(),
                                ),
                                c_,
                            )
                        });
                }
                allocation = multiplier.into_allocation();
            }
        }
    }

    if process_two {
        for ((gen_t, gen_idx, elt), c) in coefs {
            if c == 0 {
                continue;
            }
            debug_assert_eq!(c, 2);

            for a in a_list.iter_mut() {
                sub!(a, 1, 0);

                let offset =
                    module_l.generator_offset(a.degree + gen_t + elt.degree, gen_t, gen_idx);
                let num_ops = algebra.dimension(a.degree + elt.degree, 0);

                allocation = algebra.multiply_with_allocation(
                    result.slice_mut(offset, offset + num_ops),
                    1,
                    a,
                    &elt,
                    allocation,
                );
                unsub!(a, 1, 0);
            }
        }
    }
}

/// Compute the Y terms of $A(a, σ(b)σ(c))$
fn a_sigma_y(
    algebra: &Algebra,
    a: &mut MilnorClass,
    b: &mut MilnorElt,
    c: &mut MilnorElt,
    mut result: SliceMut,
) {
    let mut u = MilnorElt::default();
    let mut scratch = FpVector::new(TWO, 0);
    let mut scratch2 = FpVector::new(TWO, 0);
    let mut allocation = PPartAllocation::with_capacity(8);

    for k in 0..c.p_part.len() {
        sub!(c, k + 1, 0);
        for n in 1..b.p_part.len() + 1 {
            sub!(b, n, k);

            for m in 0..n {
                sub!(b, m, k);
                u.degree = b.degree + c.degree;

                // We find a Y_{k, l} b c term in the product, where b and c have been modified.
                // Now compute A(a, Y_{k, l}) and multiply with b and c

                let ay_degree = a.degree + (1 << (m + k)) + (1 << (n + k)) - 2;
                scratch.set_scratch_vector_size(algebra.dimension(ay_degree, 0));
                a_y(algebra, a, m + k, n + k, &mut scratch);

                scratch2.set_scratch_vector_size(algebra.dimension(ay_degree + b.degree, 0));

                allocation = algebra.multiply_element_by_basis_with_allocation(
                    scratch2.as_slice_mut(),
                    1,
                    ay_degree,
                    scratch.as_slice(),
                    b,
                    allocation,
                );
                allocation = algebra.multiply_element_by_basis_with_allocation(
                    result.copy(),
                    1,
                    ay_degree + b.degree,
                    scratch2.as_slice(),
                    c,
                    allocation,
                );

                unsub!(b, m, k);
            }
            unsub!(b, n, k);
        }
        unsub!(c, k + 1, 0);
    }
}

// Use thread-local storage to memoize a_y computation. Since the possible values of k, l grow as
// log n, in practice it is going to be at most, say, 64, and the memory usage here should be
// dwarfed by that of storing a single quasi-inverse

thread_local! {
    static AY_CACHE: RefCell<HashMap<(MilnorElt, (usize, usize)), FpVector>> = RefCell::new(HashMap::default());
}

/// Computes $A(a, Y_{k, l})$ using a thread_local cache. This dispatches to a_y_cached that acts on
/// individual Sq(R) instead of a list of them.
fn a_y(algebra: &Algebra, a_list: &mut MilnorClass, k: usize, l: usize, result: &mut FpVector) {
    for a in a_list.iter_mut() {
        a_y_cached(algebra, a, k, l, result);
    }
}

/// Compute $A(Sq(R), Y_{k, l})$ where $a = Sq(R)$. This queries the cache and computes it using
/// [`a_y_inner`] if not available.
fn a_y_cached(algebra: &Algebra, a: &mut MilnorElt, k: usize, l: usize, result: &mut FpVector) {
    AY_CACHE.with(|cache| {
        let cache = &mut *cache.try_borrow_mut().unwrap();
        match cache.get_tuple(a, &(k, l)) {
            Some(v) => result.add(v, 1),
            None => {
                let v = a_y_inner(algebra, a, k, l);
                result.add(&v, 1);
                cache.insert((a.clone(), (k, l)), v);
            }
        }
    });
}

/// Actually computes $A(a, Y_{k, l})$ and returns the result.
fn a_y_inner(algebra: &Algebra, a: &mut MilnorElt, k: usize, l: usize) -> FpVector {
    let mut result = FpVector::new(
        TWO,
        algebra.dimension(a.degree + (1 << k) + (1 << l) - 2, 0),
    );
    let mut t = MilnorElt {
        q_part: 0,
        p_part: vec![],
        degree: 0,
    };

    for i in 0..=a.p_part.len() {
        if i + k < l {
            continue;
        }

        sub!(a, i, k);
        for j in 0..=std::cmp::min(i + k - l, a.p_part.len()) {
            sub!(a, j, l);

            t.p_part.clear();
            t.p_part.resize(k + i, 0);

            t.p_part[k + i - 1] += 1;
            t.p_part[l + j - 1] += 1;

            t.degree = (1 << (k + i)) + (1 << (l + j)) - 2;

            // We can just read off the value of the product instead of passing through the
            // algorithm, but this is cached so problem for another day...
            algebra.multiply(result.as_slice_mut(), 1, &t, a);

            unsub!(a, j, l);
        }
        unsub!(a, i, k);
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::construct;
    use algebra::milnor_algebra::PPartEntry;
    use algebra::module::homomorphism::ModuleHomomorphism;
    use expect_test::{expect, Expect};
    use std::fmt::Write;

    fn from_p_part(p_part: &[PPartEntry]) -> MilnorElt {
        let degree = p_part
            .iter()
            .enumerate()
            .map(|(i, &n)| combinatorics::xi_degrees(TWO)[i] * (n as i32))
            .sum();

        MilnorElt {
            q_part: 0,
            p_part: p_part.into(),
            degree,
        }
    }

    #[test]
    fn test_a_y() {
        let algebra = Algebra::new(TWO);

        let mut result = FpVector::new(TWO, 0);

        let mut check = |p_part: &[PPartEntry], k, l, ans: Expect| {
            let mut a = MilnorClass::from_elements(vec![from_p_part(p_part)]);

            let target_deg = a.degree + (1 << k) + (1 << l) - 2;
            algebra.compute_basis(target_deg + 1);
            result.set_scratch_vector_size(algebra.dimension(target_deg, 0));
            a_y(&algebra, &mut a, k, l, &mut result);
            ans.assert_eq(&algebra.element_to_string(target_deg, result.as_slice()));
        };

        check(&[1], 0, 1, expect![["P(2)"]]);
        check(&[1], 1, 2, expect![["0"]]);
        check(&[0, 1], 0, 1, expect![["P(1, 1)"]]);
        check(&[0, 2], 1, 3, expect![["P(0, 0, 2)"]]);
        check(&[1, 2], 0, 1, expect![["P(2, 2)"]]);
    }

    #[test]
    fn test_a_tau_y() {
        let algebra = Algebra::new(TWO);

        let mut result = FpVector::new(TWO, 0);

        let mut check = |a: &[PPartEntry], b: &[PPartEntry], c: &[PPartEntry], ans: Expect| {
            let mut a = MilnorClass::from_elements(vec![from_p_part(a)]);
            let mut b = from_p_part(b);
            let mut c = from_p_part(c);

            let target_deg = a.degree + b.degree + c.degree - 1;
            algebra.compute_basis(target_deg + 1);
            result.set_scratch_vector_size(algebra.dimension(target_deg, 0));
            a_sigma_y(&algebra, &mut a, &mut b, &mut c, result.as_slice_mut());
            ans.assert_eq(&algebra.element_to_string(target_deg, result.as_slice()))
        };

        check(&[1], &[1], &[1], expect![["P(2)"]]);
        check(&[0, 2], &[0, 2], &[0, 2], expect![["P(0, 1, 2)"]]);
        check(
            &[0, 0, 4],
            &[0, 0, 4],
            &[0, 0, 4],
            expect![["P(0, 0, 3, 0, 2)"]],
        );
        check(&[1], &[2, 1], &[0, 1], expect![["0"]]);
        check(&[1], &[1], &[8], expect![["P(6, 1)"]]);
        check(&[1], &[2, 1], &[4], expect![["0"]]);
    }

    #[test]
    fn test_a_dd() {
        let resolution = construct("S_2@milnor", None).unwrap();

        let mut result = FpVector::new(TWO, 0);

        let mut check = |a: &[PPartEntry], gen_s: u32, gen_t: i32, gen_idx, ans: &str| {
            let mut a = MilnorClass::from_elements(vec![from_p_part(a)]);

            let target_deg = a.degree + gen_t - 1;
            resolution.compute_through_bidegree(gen_s, target_deg);
            let m = resolution.module(gen_s - 2);

            result.set_scratch_vector_size(m.dimension(target_deg));
            compute_a_dd(
                &resolution,
                &mut a,
                gen_s,
                gen_t,
                gen_idx,
                result.as_slice_mut(),
            );
            assert_eq!(
                &m.element_to_string(target_deg, result.as_slice()),
                ans,
                "A({}, dd x_({}, {}))",
                a.elements[0],
                gen_t - gen_s as i32,
                gen_s
            );
        };

        check(&[1], 2, 5, 0, "P(5) x_{0,0} + P(2, 1) x_{0,0}");
        check(&[2], 2, 4, 0, "P(5) x_{0,0}");
        check(&[4], 2, 2, 0, "P(2, 1) x_{0,0}");
        check(
            &[1],
            3,
            10,
            0,
            "P(9) x_{1,0} + P(3, 2) x_{1,0} + P(2, 2) x_{2,0} + P(3, 1) x_{4,0} + P(0, 2) x_{4,0}",
        );
    }

    #[test]
    fn test_compute_differentials() {
        let mut result = String::new();
        let resolution = construct("S_2@milnor", None).unwrap();

        let max_s = 7;
        let max_t = 30;

        #[cfg(feature = "concurrent")]
        let deltas = {
            let bucket = TokenBucket::new(core::num::NonZeroUsize::new(2).unwrap());
            resolution.compute_through_bidegree_concurrent(max_s, max_t, &bucket);
            compute_delta_concurrent(&resolution, &bucket, None)
        };

        #[cfg(not(feature = "concurrent"))]
        let deltas = {
            resolution.compute_through_bidegree(max_s, max_t);
            compute_delta(&resolution)
        };

        for s in 1..(max_s - 1) {
            let delta = &deltas[s as usize - 1];

            for t in s as i32 + 1..max_t {
                if delta.source().number_of_gens_in_degree(t + 1) == 0 {
                    continue;
                }
                let d = delta.hom_k(t);

                for (i, entry) in d.into_iter().enumerate() {
                    writeln!(
                        &mut result,
                        "d_2 x_({}, {}, {}) = {:?}",
                        t - s as i32,
                        s,
                        i,
                        entry
                    )
                    .unwrap();
                }
            }
        }

        expect![[r#"
            d_2 x_(1, 1, 0) = [0]
            d_2 x_(15, 1, 0) = [1]
            d_2 x_(8, 2, 0) = [0]
            d_2 x_(15, 2, 0) = [0]
            d_2 x_(16, 2, 0) = [0]
            d_2 x_(18, 2, 0) = [0]
            d_2 x_(15, 3, 0) = [0]
            d_2 x_(18, 3, 0) = [0]
            d_2 x_(19, 3, 0) = [0]
            d_2 x_(21, 3, 0) = [0]
            d_2 x_(15, 4, 0) = [0]
            d_2 x_(17, 4, 0) = [1]
            d_2 x_(18, 4, 0) = [0]
            d_2 x_(18, 4, 1) = [1]
            d_2 x_(17, 5, 0) = [0]
            d_2 x_(18, 5, 0) = [1]
            d_2 x_(24, 5, 0) = [0]
        "#]]
        .assert_eq(&result);
    }
}
