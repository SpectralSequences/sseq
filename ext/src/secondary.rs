use crate::chain_complex::ChainComplex;
use crate::resolution::ResolutionInner;
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
#[cfg(feature = "concurrent")]
use saveload::{Load, Save};
use rustc_hash::FxHashMap as HashMap;

#[cfg(feature = "concurrent")]
use std::{
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Instant,
};

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
fn read_saved_data(buffer: &mut impl Read) -> std::io::Result<(u32, i32, Vec<FpVector>)> {
    let s = u32::load(buffer, &())?;
    let t = i32::load(buffer, &())?;
    let data = Vec::<FpVector>::load(buffer, &TWO)?;

    Ok((s, t, data))
}

#[cfg(feature = "concurrent")]
pub fn compute_delta_concurrent(
    res: &Arc<Resolution>,
    max_s: u32,
    max_t: i32,
    bucket: &Arc<TokenBucket>,
) -> Vec<FMH> {
    if max_s < 2 {
        return vec![];
    }
    let min_degree = res.min_degree();

    let start = Instant::now();
    let mut handles = Vec::with_capacity(bucket.max_threads + 1);

    // Pretty print progress of first step
    let res_ = Arc::clone(res);
    let (p_sender, p_receiver) = mpsc::channel();
    handles.push(thread::spawn(move || {
        let mut processed = rustc_hash::FxHashSet::default();
        for s in 0..3 {
            for t in min_degree..=max_t {
                processed.insert((s, t));
            }
        }
        let mut prev = Instant::now();
        loop {
            match p_receiver.recv_timeout(std::time::Duration::from_secs(1)) {
                Ok(data) => {
                    processed.insert(data);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(_) => break,
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
    let mut ddeltas =
        vec![
            BiVec::from_vec(min_degree + 1, vec![None; (max_t - min_degree) as usize]);
            max_s as usize - 2
        ];
    if Path::new("ddelta.save").exists() {
        let f = std::fs::File::open("ddelta.save").unwrap();
        let mut f = BufReader::new(f);
        loop {
            match read_saved_data(&mut f) {
                Ok((s, t, data)) => {
                    if s <= max_s && t <= max_t {
                        ddeltas[s as usize - 3][t] = Some(data);
                        p_sender.send((s, t)).unwrap();
                    }
                }
                Err(_) => break,
            }
        }
    }

    let ddeltas = Arc::new(Mutex::new(ddeltas));

    let save_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("ddelta.save")
        .unwrap();
    let save_file = Arc::new(Mutex::new(BufWriter::new(save_file)));

    let (sender, receiver) = mpsc::channel();
    let receiver = Arc::new(Mutex::new(receiver));

    for _ in 0..bucket.max_threads {
        let ddeltas = Arc::clone(&ddeltas);
        let receiver = Arc::clone(&receiver);
        let res = Arc::clone(res);
        let save_file = Arc::clone(&save_file);

        let p_sender = p_sender.clone();
        handles.push(thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv().ok();

            if let Some((s, t)) = job {
                if ddeltas.lock().unwrap()[s as usize - 3][t].is_some() {
                    continue;
                }
                let m = res.module(s);

                let num_gens = m.number_of_gens_in_degree(t);
                let target_dim = res.module(s - 3).dimension(t - 1);
                let mut results = vec![FpVector::new(TWO, target_dim); num_gens];

                for (idx, result) in results.iter_mut().enumerate() {
                    d_delta_g(&*res, s, t, idx, result);
                }

                let mut sf = save_file.lock().unwrap();
                s.save(&mut *sf).unwrap();
                t.save(&mut *sf).unwrap();
                results.save(&mut *sf).unwrap();
                sf.flush().unwrap();
                drop(sf);

                ddeltas.lock().unwrap()[s as usize - 3][t] = Some(results);

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
            sender.send((s, t)).unwrap();
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

    let mut last_receiver: Option<mpsc::Receiver<()>> = None;
    let mut handles = Vec::with_capacity(ddeltas.len());
    for (s, ddeltas_) in ddeltas.into_iter().enumerate() {
        let s = s as u32 + 3;

        let (sender, receiver) = mpsc::channel();

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
            for (t, ddelta) in ddeltas_.into_iter_enum() {
                let mut ddelta = ddelta.unwrap();
                token = bucket.recv_or_release(token, &last_receiver);

                let num_gens = source_module.number_of_gens_in_degree(t);
                let target_dim = target_module.dimension(t - 1);
                let mut results = vec![FpVector::new(TWO, target_dim); num_gens];

                for (idx, result) in results.iter_mut().enumerate() {
                    let row: &mut FpVector = &mut ddelta[idx];
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
    let mut temp = MilnorElt::default();

    let mut b = MilnorElt::default();
    let mut c = MilnorElt::default();

    let process_mu0 = a_list.elements.iter().any(|x| x.p_part[0] > 0);
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
                let mut multiplier = PPartMultiplier::<true>::new_from_allocation(
                    TWO, &b.p_part, &c.p_part, allocation,
                );
                temp.degree = b.degree + c.degree;
                while let Some(c_) = multiplier.next(&mut temp) {
                    let key = (elt2.generator_degree, elt2.generator_index, temp.clone());
                    let val = (c_ + coefs.get(&key).copied().unwrap_or(0)) % 4;
                    coefs.insert(key, val);
                }
                allocation = multiplier.into_allocation();
            }
        }
    }
    if process_mu0 {
        for ((gen_t, gen_idx, elt), c) in coefs {
            if c == 0 {
                continue;
            }
            debug_assert_eq!(c, 2);

            for a in &mut a_list.elements {
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

// Computes A(a, Y_{k, l})
fn a_y(algebra: &Algebra, a_list: &mut MilnorClass, k: usize, l: usize, result: &mut FpVector) {
    let mut t = MilnorElt {
        q_part: 0,
        p_part: vec![],
        degree: 0,
    };

    for a in a_list.iter_mut() {
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

                algebra.multiply(result, 1, &t, &a);

                unsub!(a, j, l);
            }
            unsub!(a, i, k);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::construct_s_2;

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

        let mut check = |p_part: &[u32], k, l, ans: &str| {
            let mut a = MilnorClass::from_elements(vec![from_p_part(p_part)]);

            let target_deg = a.degree + (1 << k) + (1 << l) - 2;
            algebra.compute_basis(target_deg + 1);
            result.set_scratch_vector_size(algebra.dimension(target_deg, 0));
            a_y(&algebra, &mut a, k, l, &mut result);
            assert_eq!(
                &algebra.element_to_string(target_deg, &result),
                ans,
                "{} U_({},{})",
                a.elements[0],
                k,
                l
            );
        };

        check(&[1], 0, 1, "P(2)");
        check(&[1], 1, 2, "0");
        check(&[0, 1], 0, 1, "P(1, 1)");
        check(&[0, 2], 1, 3, "P(0, 0, 2)");
        check(&[1, 2], 0, 1, "P(2, 2)");
    }

    #[test]
    fn test_a_tau_y() {
        let algebra = Algebra::new(TWO);

        let mut result = FpVector::new(TWO, 0);

        let mut check = |a: &[u32], b: &[u32], c: &[u32], ans: &str| {
            let mut a = MilnorClass::from_elements(vec![from_p_part(a)]);
            let mut b = from_p_part(b);
            let mut c = from_p_part(c);

            let target_deg = a.degree + b.degree + c.degree - 1;
            algebra.compute_basis(target_deg + 1);
            result.set_scratch_vector_size(algebra.dimension(target_deg, 0));
            a_tau_y(&algebra, &mut a, &mut b, &mut c, &mut result);
            assert_eq!(
                &algebra.element_to_string(target_deg, &result),
                ans,
                "A({}, τ({},{}))",
                a.elements[0],
                b,
                c
            );
        };

        check(&[1], &[1], &[1], "P(2)");
        check(&[0, 2], &[0, 2], &[0, 2], "P(0, 1, 2)");
        check(&[0, 0, 4], &[0, 0, 4], &[0, 0, 4], "P(0, 0, 3, 0, 2)");
        check(&[1], &[2, 1], &[0, 1], "0");
        check(&[1], &[1], &[8], "P(6, 1)");
        check(&[1], &[2, 1], &[4], "0");
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

        let s = 7;
        let t = 30;

        #[cfg(feature = "concurrent")]
        let deltas = {
            let bucket = std::sync::Arc::new(TokenBucket::new(2));
            resolution.resolve_through_bidegree_concurrent(s, t, &bucket);
            compute_delta_concurrent(&resolution.inner, s, t, &bucket)
        };

        #[cfg(not(feature = "concurrent"))]
        let deltas = {
            resolution.resolve_through_bidegree(s, t);
            compute_delta(&resolution.inner, s, t)
        };

        for s_ in 3..=s {
            for t_ in s_ as i32..=t {
                let module = resolution.module(s_);
                let module2 = resolution.module(s_ - 2);
                if module2.number_of_gens_in_degree(t_ - 1) == 0 {
                    continue;
                }

                let start = module2.generator_offset(t_ - 1, t_ - 1, 0);
                for idx in 0..module.number_of_gens_in_degree(t_) {
                    result.push_str(&format!(
                        "d_2* (x_({}, {})^({})]) = {:?}\n",
                        t_ - s_ as i32,
                        s_,
                        idx,
                        deltas[s_ as usize - 3]
                            .output(t_, idx)
                            .iter()
                            .skip(start)
                            .collect::<Vec<_>>()
                    ));
                }
            }
        }
        assert_eq!(
            result,
            r"d_2* (x_(0, 3)^(0)]) = [0]
d_2* (x_(14, 3)^(0)]) = [1]
d_2* (x_(7, 4)^(0)]) = [0]
d_2* (x_(14, 4)^(0)]) = [0]
d_2* (x_(15, 4)^(0)]) = [0]
d_2* (x_(17, 4)^(0)]) = [0]
d_2* (x_(14, 5)^(0)]) = [0]
d_2* (x_(17, 5)^(0)]) = [0]
d_2* (x_(18, 5)^(0)]) = [0]
d_2* (x_(20, 5)^(0)]) = [0]
d_2* (x_(14, 6)^(0)]) = [0]
d_2* (x_(16, 6)^(0)]) = [1]
d_2* (x_(17, 6)^(0)]) = [0, 1]
d_2* (x_(16, 7)^(0)]) = [0]
d_2* (x_(17, 7)^(0)]) = [1]
d_2* (x_(23, 7)^(0)]) = [0]
"
        );
    }
}
