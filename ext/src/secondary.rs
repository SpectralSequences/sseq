use crate::chain_complex::ChainComplex;
use crate::resolution::ResolutionInner;
use crate::utils::HashMapTuple;
use crate::CCC;
use algebra::combinatorics;
use algebra::milnor_algebra::{
    MilnorAlgebra as Algebra, MilnorBasisElement as MilnorElt, PPartAllocation, PPartMultiplier,
};
use algebra::module::homomorphism::{FreeModuleHomomorphism, ModuleHomomorphism};
use algebra::module::FreeModule;
use algebra::module::Module;
use algebra::{Algebra as _, MilnorAlgebraT, SteenrodAlgebra};
#[cfg(feature = "concurrent")]
use bivec::BiVec;
use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};
use rustc_hash::FxHashMap as HashMap;
#[cfg(feature = "concurrent")]
use saveload::{Load, Save};
use std::cell::RefCell;
use std::hash::{BuildHasher, Hash, Hasher};

#[cfg(feature = "concurrent")]
use std::{
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

#[cfg(feature = "concurrent")]
use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError};

#[cfg(feature = "concurrent")]
use thread_token::TokenBucket;

type Resolution = ResolutionInner<CCC>;
type FMH = FreeModuleHomomorphism<FreeModule<SteenrodAlgebra>>;

const TWO: ValidPrime = ValidPrime::new(2);

// The normal unwrap requires implementing debug
#[cfg(feature = "concurrent")]
fn unwrap<T, S>(x: Result<T, S>) -> T {
    match x {
        Ok(x) => x,
        _ => panic!(),
    }
}

/// An element in the Milnor algebra
pub struct MilnorClass {
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
pub fn compute_delta(res: &Resolution, max_s: u32, max_t: i32) -> Vec<FMH> {
    if max_s < 2 {
        return vec![];
    }
    let deltas = (3..=max_s)
        .map(|s| FreeModuleHomomorphism::new(res.module(s), res.module(s - 2), 1))
        .collect::<Vec<_>>();

    let mut scratch = FpVector::new(TWO, 0);
    for s in 3..=max_s {
        let delta = &deltas[s as usize - 3];
        let d = res.differential(s - 2);
        let m = res.module(s);

        delta.extend_by_zero_safe(res.min_degree());
        for t in res.min_degree() + 1..=max_t {
            let num_gens = m.number_of_gens_in_degree(t);
            let target_dim = res.module(s - 2).dimension(t - 1);
            let mut results = vec![FpVector::new(TWO, target_dim); num_gens];

            scratch.set_scratch_vector_size(res.module(s - 3).dimension(t - 1));
            for (idx, result) in results.iter_mut().enumerate() {
                d_delta_g(res, s, t, idx, &mut scratch);

                if s > 3 {
                    deltas[s as usize - 4].apply(
                        &mut scratch,
                        1,
                        t,
                        res.differential(s).output(t, idx),
                    );
                }

                #[cfg(debug_assertions)]
                if s > 3 {
                    let mut r = FpVector::new(TWO, res.module(s - 4).dimension(t - 1));
                    res.differential(s - 3).apply(&mut r, 1, t - 1, &scratch);
                    assert!(r.is_zero(), "dd != 0 at s = {}, t = {}", s, t);
                }

                d.quasi_inverse(t - 1).apply(result, 1, &scratch);
                scratch.set_to_zero_pure();
            }
            delta.add_generators_from_rows(&delta.lock(), t, results);
        }
    }

    deltas
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
    res: &Arc<Resolution>,
    max_s: u32,
    max_t: i32,
    bucket: &Arc<TokenBucket>,
    save_file_path: &str,
) -> Vec<FMH> {
    if max_s < 2 {
        return vec![];
    }
    let min_degree = res.min_degree();

    let start = Instant::now();
    let mut handles = Vec::with_capacity(bucket.max_threads + 1);

    // Pretty print progress of first step
    let res_ = Arc::clone(res);
    let (p_sender, p_receiver) = unbounded();
    let mut processed: HashMap<(u32, i32), u32> = HashMap::default();

    for s in 3..=max_s {
        let m = res.module(s);
        for t in min_degree + 1..=max_t {
            processed.insert((s, t), m.number_of_gens_in_degree(t) as u32);
        }
    }

    handles.push(thread::spawn(move || {
        let mut prev = Instant::now();
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
            print!("\x1b[2J\x1b[H");
            println!(
                "Time elapsed: {:.2?}; Processed bidegrees:",
                start.elapsed()
            );
            crate::utils::print_resolution_color(
                &*res_,
                std::cmp::min(max_s, ((max_t - min_degree) as u32 + 2) / 3),
                max_t,
                &processed,
            );
            println!();
            prev = Instant::now();
        }
    }));

    // We now compute the A terms of dδg. There are no dependencies between the different
    // bidegrees, so we use a thread pool for this. The ddeltas store the results as a vector:
    // source_s -> source_t -> gen_idx -> value. Since they are not populated in order of source_s
    // and source_t, we pre-populate with None and replace with Some.
    let mut ddeltas: Vec<BiVec<Vec<Option<FpVector>>>> = Vec::with_capacity(max_s as usize - 2);
    for s in 3..=max_s {
        let m = res.module(s);
        let mut v = BiVec::with_capacity(min_degree + 1, max_t + 1);
        for t in min_degree + 1..=max_t {
            v.push(vec![None; m.number_of_gens_in_degree(t)]);
        }
        ddeltas.push(v);
    }

    if save_file_path != "-" && Path::new(save_file_path).exists() {
        let f = std::fs::File::open(save_file_path).unwrap();
        let mut f = BufReader::new(f);
        loop {
            match read_saved_data(&mut f) {
                Ok((s, t, idx, data)) => {
                    if s <= max_s && t <= max_t {
                        ddeltas[s as usize - 3][t][idx] = Some(data);
                        p_sender.send((s, t)).unwrap();
                    }
                }
                Err(_) => break,
            }
        }
    }

    let ddeltas = Arc::new(Mutex::new(ddeltas));

    let save_file = if save_file_path == "-" {
        Arc::new(None)
    } else {
        let f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(save_file_path)
            .unwrap();
        Arc::new(Some(Mutex::new(BufWriter::new(f))))
    };

    let (sender, receiver) = unbounded::<(u32, i32, usize)>();
    let receiver = Arc::new(Mutex::new(receiver));

    for _ in 0..bucket.max_threads {
        let ddeltas = Arc::clone(&ddeltas);
        let receiver = Arc::clone(&receiver);
        let res = Arc::clone(res);
        let save_file = Arc::clone(&save_file);

        let p_sender = p_sender.clone();
        handles.push(thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().ok();

            if let Some((s, t, idx)) = job {
                if ddeltas.lock().unwrap()[s as usize - 3][t][idx].is_some() {
                    continue;
                }
                let target_dim = res.module(s - 3).dimension(t - 1);
                let mut result = FpVector::new(TWO, target_dim);

                d_delta_g(&*res, s, t, idx, &mut result);

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
        }));
    }
    drop(p_sender);

    // Iterate in reverse order to do the slower ones first
    for t in (min_degree + 1..=max_t).rev() {
        for s in 3..=max_s {
            for idx in 0..res.module(s).number_of_gens_in_degree(t) {
                sender.send((s, t, idx)).unwrap();
            }
        }
    }
    drop(sender);

    for handle in handles {
        handle.join().unwrap();
    }

    let ddeltas = unwrap(Arc::try_unwrap(ddeltas)).into_inner().unwrap();

    println!("Computed A terms in {:.2?}", start.elapsed());

    // We now compute the rest of the terms. This step is substantially faster than the previous
    // step, so we don't have to be so careful about optimization (the cost is the cost of
    // computing one product, which is not too much). To compute delta[s][t], we need to know
    // delta[s - 1][k] for k < t. However, it is easier to require computing everything to the
    // bottom and left of delta[s][t], so might as well do it that way.

    let start = std::time::Instant::now();

    let deltas = (3..=max_s)
        .map(|s| FreeModuleHomomorphism::new(res.module(s), res.module(s - 2), 1))
        .collect::<Vec<_>>();

    let deltas = Arc::new(deltas);

    let mut last_receiver: Option<Receiver<()>> = None;
    let mut handles = Vec::with_capacity(ddeltas.len());
    for (s, ddeltas_) in ddeltas.into_iter().enumerate() {
        let s = s as u32 + 3;

        let (sender, receiver) = unbounded();

        let deltas = Arc::clone(&deltas);
        let bucket = Arc::clone(bucket);

        let source_d = res.differential(s);
        let target_d = res.differential(s - 2);
        let source_module = res.module(s);
        let target_module = res.module(s - 2);

        #[cfg(debug_assertions)]
        let res = Arc::clone(res);

        handles.push(thread::spawn(move || {
            let delta = &deltas[s as usize - 3];

            delta.extend_by_zero_safe(min_degree);
            let mut token = bucket.take_token();
            for (t, mut ddelta) in ddeltas_.into_iter_enum() {
                token = bucket.recv_or_release(token, &last_receiver);

                let num_gens = source_module.number_of_gens_in_degree(t);
                let target_dim = target_module.dimension(t - 1);
                let mut results = vec![FpVector::new(TWO, target_dim); num_gens];

                for (idx, result) in results.iter_mut().enumerate() {
                    let row: &mut FpVector = ddelta[idx].as_mut().unwrap();
                    if s > 3 {
                        deltas[s as usize - 4].apply(row, 1, t, source_d.output(t, idx));
                    }

                    #[cfg(debug_assertions)]
                    if s > 3 {
                        let mut r = FpVector::new(TWO, res.module(s - 4).dimension(t - 1));
                        res.differential(s - 3).apply(&mut r, 1, t - 1, &row);
                        assert!(r.is_zero(), "dd != 0 at s = {}, t = {}", s, t);
                    }

                    target_d.quasi_inverse(t - 1).apply(result, 1, row);
                }
                delta.add_generators_from_rows(&delta.lock(), t, results);
                sender.send(()).unwrap();
            }
        }));
        last_receiver = Some(receiver);
    }
    for handle in handles {
        handle.join().unwrap();
    }
    println!("Computed δd terms in {:.2?}", start.elapsed());
    unwrap(Arc::try_unwrap(deltas))
}

/// Computes d(delta(g));
pub fn d_delta_g(res: &Resolution, gen_s: u32, gen_t: i32, gen_idx: usize, result: &mut FpVector) {
    let m = res.module(gen_s - 1);

    let d = res.differential(gen_s);
    let dg = d.output(gen_t, gen_idx);

    for t in 0..gen_t {
        for idx in 0..m.number_of_gens_in_degree(t) {
            let mut a_list = MilnorClass::from_module_row(&dg, &m, gen_t, t, idx);

            if !a_list.elements.is_empty() {
                a_dd(res, &mut a_list, gen_s - 1, t, idx, result);
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

/// Computes A(a, ddg)
pub fn a_dd(
    res: &Resolution,
    a_list: &mut MilnorClass,
    gen_s: u32,
    gen_t: i32,
    gen_idx: usize,
    result: &mut FpVector,
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

    let process_mu0 = a_list.iter().any(|x| x.p_part[0] > 0);
    let mut allocation = PPartAllocation::default();

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

            a_tau_y(
                algebra,
                a_list,
                &mut b,
                &mut c,
                &mut *result.borrow_slice(offset, offset + num_ops),
            );

            if process_mu0 {
                let (mut temp, mut multiplier) = PPartMultiplier::<true>::new_from_allocation(
                    TWO, &b.p_part, &c.p_part, allocation,
                );
                temp.degree = b.degree + c.degree;
                while let Some(c_) = multiplier.next(&mut temp) {
                    let mut hasher = coefs.hasher().build_hasher();
                    elt2.generator_degree.hash(&mut hasher);
                    elt2.generator_index.hash(&mut hasher);
                    temp.hash(&mut hasher);
                    let entry = coefs.raw_entry_mut().from_hash(hasher.finish(), |v| {
                        v.0 == elt2.generator_degree && v.1 == elt2.generator_index && v.2 == temp
                    });

                    entry
                        .and_modify(|_k, v| *v = (*v + c_) % 4)
                        .or_insert_with(|| {
                            (
                                (elt2.generator_degree, elt2.generator_index, temp.clone()),
                                c_,
                            )
                        });
                }
                allocation = multiplier.into_allocation(temp);
            }
        }
    }
    if process_mu0 {
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
                    &mut *result.borrow_slice(offset, offset + num_ops),
                    1,
                    &a,
                    &elt,
                    allocation,
                );
                unsub!(a, 1, 0);
            }
        }
    }
}

/// Compute the Y terms of A(a, τ(b, c))
fn a_tau_y(
    algebra: &Algebra,
    a: &mut MilnorClass,
    b: &mut MilnorElt,
    c: &mut MilnorElt,
    result: &mut FpVector,
) {
    let mut u = MilnorElt::default();
    let mut scratch = FpVector::new(TWO, 0);
    let mut scratch2 = FpVector::new(TWO, 0);
    let mut allocation = algebra::milnor_algebra::PPartAllocation::default();

    // First compute τ(b, c)
    for k in 0..c.p_part.len() {
        sub!(c, k + 1, 0);
        for n in 1..b.p_part.len() + 1 {
            sub!(b, n, k);

            for m in 0..n {
                sub!(b, m, k);
                u.degree = b.degree + c.degree;

                let ay_degree = a.degree + (1 << (m + k)) + (1 << (n + k)) - 2;
                scratch.set_scratch_vector_size(algebra.dimension(ay_degree, 0));
                a_y(algebra, a, m + k, n + k, &mut scratch);

                scratch2.set_scratch_vector_size(algebra.dimension(ay_degree + b.degree, 0));

                allocation = algebra.multiply_element_by_basis_with_allocation(
                    &mut scratch2,
                    1,
                    ay_degree,
                    &scratch,
                    &b,
                    allocation,
                );
                allocation = algebra.multiply_element_by_basis_with_allocation(
                    result,
                    1,
                    ay_degree + b.degree,
                    &scratch2,
                    &c,
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

/// Computes A(a, Y_{k, l}) using a thread_local cache. This dispatches to a_y_cached that acts on
/// individual Sq(R) instead of a list of them
fn a_y(algebra: &Algebra, a_list: &mut MilnorClass, k: usize, l: usize, result: &mut FpVector) {
    for a in a_list.iter_mut() {
        a_y_cached(algebra, a, k, l, result);
    }
}

/// Compute A(Sq(R), Y_{k, l}) where a = Sq(R). This queries the cache and computes it using
/// a_y_inner if not available.
fn a_y_cached(algebra: &Algebra, a: &mut MilnorElt, k: usize, l: usize, result: &mut FpVector) {
    AY_CACHE.with(|cache| {
        let cache = &mut *cache.try_borrow_mut().unwrap();
        match cache.get_tuple(a, &(k, l)) {
            Some(v) => result.add_shift_none_pure(v, 1),
            None => {
                let v = a_y_inner(algebra, a, k, l);
                result.add_shift_none_pure(&v, 1);
                cache.insert((a.clone(), (k, l)), v);
            }
        }
    });
}

/// Actually computes A(a, Y_{k, l}) and returns the result.
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

            algebra.multiply(&mut result, 1, &t, &a);

            unsub!(a, j, l);
        }
        unsub!(a, i, k);
    }
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::construct_s_2;
    use expect_test::{expect, Expect};
    use std::fmt::Write;

    fn from_p_part(p_part: &[u32]) -> MilnorElt {
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

        let mut check = |p_part: &[u32], k, l, ans: Expect| {
            let mut a = MilnorClass::from_elements(vec![from_p_part(p_part)]);

            let target_deg = a.degree + (1 << k) + (1 << l) - 2;
            algebra.compute_basis(target_deg + 1);
            result.set_scratch_vector_size(algebra.dimension(target_deg, 0));
            a_y(&algebra, &mut a, k, l, &mut result);
            ans.assert_eq(&algebra.element_to_string(target_deg, &result));
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

        let mut check = |a: &[u32], b: &[u32], c: &[u32], ans: Expect| {
            let mut a = MilnorClass::from_elements(vec![from_p_part(a)]);
            let mut b = from_p_part(b);
            let mut c = from_p_part(c);

            let target_deg = a.degree + b.degree + c.degree - 1;
            algebra.compute_basis(target_deg + 1);
            result.set_scratch_vector_size(algebra.dimension(target_deg, 0));
            a_tau_y(&algebra, &mut a, &mut b, &mut c, &mut result);
            ans.assert_eq(&algebra.element_to_string(target_deg, &result))
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
        let bundle = construct_s_2("milnor");
        let resolution = &*bundle.resolution.read();

        let mut result = FpVector::new(TWO, 0);

        let mut check = |a: &[u32], gen_s: u32, gen_t: i32, gen_idx, ans: &str| {
            let mut a = MilnorClass::from_elements(vec![from_p_part(a)]);

            let target_deg = a.degree + gen_t - 1;
            resolution.resolve_through_bidegree(gen_s, target_deg);
            let m = resolution.module(gen_s - 2);

            result.set_scratch_vector_size(m.dimension(target_deg));
            a_dd(
                &*resolution.inner,
                &mut a,
                gen_s,
                gen_t,
                gen_idx,
                &mut result,
            );
            assert_eq!(
                &m.element_to_string(target_deg, &result),
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
        let bundle = construct_s_2("milnor");
        let resolution = &*bundle.resolution.read();

        let max_s = 7;
        let max_t = 30;

        #[cfg(feature = "concurrent")]
        let deltas = {
            let bucket = std::sync::Arc::new(TokenBucket::new(2));
            resolution.resolve_through_bidegree_concurrent(max_s, max_t, &bucket);
            compute_delta_concurrent(&resolution.inner, max_s, max_t, &bucket, "-")
        };

        #[cfg(not(feature = "concurrent"))]
        let deltas = {
            resolution.resolve_through_bidegree(max_s, max_t);
            compute_delta(&resolution.inner, max_s, max_t)
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
