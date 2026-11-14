//! Replay a resolution of the sphere step by step and print the trace as JSON.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use algebra::SteenrodKind;
use groebner::{
    Bounds, GbElement, Level, ModMono, Mono, Outcome, Pair, PairEvent, PairObserver, Params, Poly,
    Resolution,
};
use serde_json::{Value, json};

/// One reduction as the program reports it.
#[derive(Clone)]
struct Ev {
    s: i32,
    pair: Option<Pair>,
    outcome: Outcome,
}

/// Collects the reductions of each degree.
struct Rec(Arc<Mutex<Vec<Ev>>>);

impl PairObserver for Rec {
    /// Record the event.
    fn observe(&mut self, e: &PairEvent) {
        self.0.lock().unwrap().push(Ev {
            s: e.s,
            pair: e.context.map(|c| *c.pair),
            outcome: e.outcome,
        });
    }
}

/// The Milnor basis element $Q(E) P(R)$ as `{"e": E, "r": R}`, with $Q_k$ in bit `k` of `E`.
fn mono_json(params: &Params, m: Mono) -> Value {
    let (e, r) = params.decode(m);
    json!({"e": e, "r": r})
}

/// The terms of `x`, as `[generator, monomial, coefficient]`.
fn poly_json(params: &Params, x: &Poly) -> Value {
    Value::Array(
        x.terms()
            .map(|(m, c)| json!([m.generator(), mono_json(params, m.mono()), c]))
            .collect(),
    )
}

/// A term, as `[generator, monomial]`.
fn term_json(params: &Params, m: ModMono) -> Value {
    json!([m.generator(), mono_json(params, m.mono())])
}

/// Reduce `x` modulo the first `n` elements of `level`, carrying `companion` along, recording
/// every step.
fn reduce(
    params: &Params,
    level: &Level,
    n: usize,
    x: &mut Poly,
    mut companion: Option<&mut Poly>,
    steps: &mut Vec<Value>,
) {
    let p = params.prime();
    let elements = &level.elements()[..n];
    let leads = &level.leads()[..n];
    let mut out: Vec<(ModMono, u32)> = Vec::new();
    let mut rest = std::mem::take(x);
    while let Some((m, c)) = rest.lead() {
        let Some(k) = (0..n).find(|&k| {
            leads[k].generator() == m.generator() && params.divides(leads[k].mono(), m.mono())
        }) else {
            out.push((m, c));
            rest.add_scaled(&Poly::monomial(m, c), params.neg(1), p);
            continue;
        };
        let u = params.quotient(m.mono(), leads[k].mono());
        let px1 = params.mul_poly(u, 1, &elements[k].x1);
        let px2 = params.mul_poly(u, 1, &elements[k].x2);
        let (lead, lambda) = px1.lead().unwrap();
        assert_eq!(lead, m);
        let factor = params.neg(c * params.inverse(lambda) % p);
        Poly::add_scaled(&mut rest, &px1, factor, p);
        if let Some(comp) = companion.as_deref_mut() {
            comp.add_scaled(&px2, factor, p);
        }
        let current = {
            let mut all = Poly::from_terms(out.clone(), p);
            Poly::add_scaled(&mut all, &rest, 1, p);
            all
        };
        steps.push(json!({
            "term": term_json(params, m),
            "coef": c,
            "k": k,
            "quotient": mono_json(params, u),
            "factor": factor,
            "prod_x1": poly_json(params, &px1),
            "prod_x2": poly_json(params, &px2),
            "after_x1": poly_json(params, &current),
            "after_x2": companion.as_deref().map(|c| poly_json(params, c)),
        }));
    }
    *x = Poly::from_terms(out, p);
}

/// The components of `e`.
fn elem_json(params: &Params, e: &GbElement) -> Value {
    json!({"x1": poly_json(params, &e.x1), "x2": poly_json(params, &e.x2)})
}

/// Remove `flag` from `args`, and return whether it was there.
fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    let len = args.len();
    args.retain(|a| a != flag);
    args.len() != len
}

/// Usage: `groebner-trace [--summary] [--prime <p>] <s_max> <n_max>`.
///
/// With `--summary`, the resolution is computed in one go and only the results are printed.
fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let summary = take_flag(&mut args, "--summary");
    let p: u32 = match args.iter().position(|a| a == "--prime") {
        Some(i) => {
            args.remove(i);
            args.remove(i).parse().unwrap()
        }
        None => 2,
    };
    let s_max: u32 = args[0].parse().unwrap();
    let n_max: i32 = args[1].parse().unwrap();
    let t_max = n_max + s_max as i32;
    let params = Arc::new(Params::new(p, SteenrodKind::Classical));
    let mut res = Resolution::sphere(Arc::clone(&params), Bounds { s_max, n_max });
    let log = Arc::new(Mutex::new(Vec::new()));
    if !summary {
        res.set_observer(Some(Box::new(Rec(Arc::clone(&log)))));
    }
    let num_levels = s_max as usize;
    // The relations of the sphere, as `Resolution::sphere` makes them.
    let mut relations: BTreeMap<i32, Poly> = BTreeMap::new();
    let unit = |m: Mono| Poly::monomial(ModMono::new(0, m), 1);
    if params.has_exterior() {
        let m = params.from_digits(1, []);
        relations.insert(params.degree(m), unit(m));
    }
    for exp in 0.. {
        let Some(idx) = params.position_of(1, exp) else {
            break;
        };
        let m = params.from_digits(0, [(idx, 1)]);
        relations.insert(params.degree(m), unit(m));
    }

    let mut degrees = Vec::new();
    if summary {
        res.compute_through(t_max);
    }
    for t in (1..=t_max).filter(|_| !summary) {
        let before: Vec<usize> = (0..num_levels)
            .map(|s| res.level(s as u32).elements().len())
            .collect();
        let gens_before: Vec<usize> = (0..=num_levels)
            .map(|s| res.generators(s as u32).len())
            .collect();
        // Elements are only appended, so the state before degree `t` is a prefix of each level.
        log.lock().unwrap().clear();
        res.compute_through(t);
        let events = log.lock().unwrap().clone();

        // Processing order: per level, singles by (i2, annihilator order), then pairs by (i2, i1).
        let mut by_level: BTreeMap<i32, Vec<Ev>> = BTreeMap::new();
        for e in events {
            by_level.entry(e.s).or_default().push(e);
        }
        for (&s, evs) in by_level.iter_mut() {
            if s < 0 {
                continue;
            }
            let level = res.level(s as u32);
            evs.sort_by_key(|e| {
                let pair = e.pair.unwrap();
                if pair.is_single() {
                    let lead = level.leads()[pair.i2 as usize].mono();
                    let pos = params
                        .annihilators(lead)
                        .position(|m| m == pair.m2)
                        .unwrap();
                    (0, pair.i2, pos as u32)
                } else {
                    (1, pair.i2, pair.i1)
                }
            });
        }

        // Reduce, as `Resolution::step` does.
        /// A reduction, with the steps that brought it to its state before sorting.
        struct Item {
            ev: Ev,
            elem: GbElement,
            start: Value,
            steps1: Vec<Value>,
            steps2: Vec<Value>,
        }
        let mut relation_idx = 0;
        let mut items: BTreeMap<i32, Vec<Item>> = BTreeMap::new();
        for (&s, evs) in &by_level {
            for ev in evs {
                let mut elem = match ev.pair {
                    None => {
                        relation_idx += 1;
                        GbElement::new(Poly::zero(), relations[&t].clone())
                    }
                    Some(pair) => res.level(s as u32).pair_element(&params, &pair, false),
                };
                let start = elem_json(&params, &elem);
                let mut steps1 = Vec::new();
                let mut steps2 = Vec::new();
                if s >= 0 {
                    let GbElement { x1, x2, .. } = &mut elem;
                    reduce(
                        &params,
                        res.level(s as u32),
                        before[s as usize],
                        x1,
                        Some(x2),
                        &mut steps1,
                    );
                }
                if ((s + 1) as usize) < num_levels {
                    reduce(
                        &params,
                        res.level((s + 1) as u32),
                        before[(s + 1) as usize],
                        &mut elem.x2,
                        None,
                        &mut steps2,
                    );
                }
                items.entry(s).or_default().push(Item {
                    ev: ev.clone(),
                    elem,
                    start,
                    steps1,
                    steps2,
                });
            }
        }
        assert!(relation_idx <= 1);

        // Sort, from the top level down, checking each outcome against the program's.
        let mut new: Vec<Vec<GbElement>> = vec![Vec::new(); num_levels];
        let mut new_gens: Vec<usize> = gens_before.clone();
        let mut out_items: Vec<Value> = Vec::new();
        for s in (-1..num_levels as i32).rev() {
            let Some(list) = items.remove(&s) else {
                continue;
            };
            let mut kernels: Vec<(Item, usize)> = Vec::new();
            for mut it in list {
                let mut tri1 = Vec::new();
                let mut tri2 = Vec::new();
                if s >= 0 {
                    for (j, g) in new[s as usize].iter().enumerate() {
                        let (lead, _) = g.x1.lead().unwrap();
                        let c = it.elem.x1.coefficient(lead);
                        if c == 0 {
                            continue;
                        }
                        let c = params.neg(c);
                        Poly::add_scaled(&mut it.elem.x1, &g.x1, c, p);
                        Poly::add_scaled(&mut it.elem.x2, &g.x2, c, p);
                        tri1.push(json!({"k": before[s as usize] + j, "factor": c, "after": elem_json(&params, &it.elem)}));
                    }
                }
                if let Some(next) = new.get((s + 1) as usize) {
                    for (j, g) in next.iter().enumerate() {
                        let (lead, _) = g.x1.lead().unwrap();
                        let c = it.elem.x2.coefficient(lead);
                        if c != 0 {
                            Poly::add_scaled(&mut it.elem.x2, &g.x1, params.neg(c), p);
                            tri2.push(json!({"k": before[(s + 1) as usize] + j, "factor": params.neg(c), "after": poly_json(&params, &it.elem.x2)}));
                        }
                    }
                }
                let mut v = json!({
                    "s": s,
                    "kind": match it.ev.pair { None => "relation", Some(p) if p.is_single() => "single", _ => "pair" },
                    "i1": it.ev.pair.filter(|p| !p.is_single()).map(|p| p.i1),
                    "i2": it.ev.pair.map(|p| p.i2),
                    "m1": it.ev.pair.map(|p| mono_json(&params, p.m1)),
                    "m2": it.ev.pair.map(|p| mono_json(&params, p.m2)),
                    "lcm": it.ev.pair.filter(|p| !p.is_single()).map(|p| {
                        let leads = res.level(s as u32).leads();
                        mono_json(&params, params.lcm(leads[p.i1 as usize].mono(), leads[p.i2 as usize].mono()))
                    }),
                    // The leading coefficients of m1 * b_i1 and m2 * b_i2, which the S-element
                    // cross-multiplies.
                    "lambda": it.ev.pair.filter(|p| !p.is_single()).map(|p| {
                        let elements = res.level(s as u32).elements();
                        let lc = |k: u32, m: Mono| {
                            params.mul_poly(m, 1, &elements[k as usize].x1).lead().unwrap().1
                        };
                        [lc(p.i1, p.m1), lc(p.i2, p.m2)]
                    }),
                    "start": it.start,
                    "steps1": it.steps1,
                    "steps2": it.steps2,
                    "tri1": tri1,
                    "tri2": tri2,
                    "program_outcome": it.ev.outcome.name(),
                });
                if it.elem.x1.lead().is_some() {
                    assert_eq!(it.ev.outcome, Outcome::NewElement, "t={t} s={s}");
                    v["scale"] = json!(params.inverse(it.elem.x1.lead().unwrap().1));
                    it.elem.normalize(&params);
                    v["outcome"] = json!("element");
                    v["index"] = json!(before[s as usize] + new[s as usize].len());
                    v["result"] = elem_json(&params, &it.elem);
                    new[s as usize].push(it.elem);
                    out_items.push(v);
                } else if it.elem.x2.is_zero() {
                    assert_eq!(it.ev.outcome, Outcome::Useless, "t={t} s={s}");
                    v["outcome"] = json!("useless");
                    out_items.push(v);
                } else {
                    v["cycle"] = poly_json(&params, &it.elem.x2);
                    v["outcome"] = json!("cycle");
                    out_items.push(v);
                    kernels.push((it, out_items.len() - 1));
                }
            }
            let mut cycles = Vec::new();
            for (mut it, vi) in kernels {
                let mut v = json!({"item": vi});
                let s2 = (s + 2) as usize;
                let mut tri3 = Vec::new();
                if let Some(next) = new.get((s + 1) as usize) {
                    for (j, g) in next.iter().enumerate() {
                        let (lead, _) = g.x1.lead().unwrap();
                        let c = it.elem.x2.coefficient(lead);
                        if c != 0 {
                            Poly::add_scaled(&mut it.elem.x2, &g.x1, params.neg(c), p);
                            tri3.push(json!({"k": before[(s + 1) as usize] + j, "factor": params.neg(c), "after": poly_json(&params, &it.elem.x2)}));
                        }
                    }
                }
                v["tri3"] = json!(tri3);
                let expected = if it.elem.x2.is_zero() {
                    v["outcome"] = json!("boundary");
                    Outcome::Boundary
                } else if s2 > s_max as usize {
                    v["outcome"] = json!("beyond");
                    v["cycle"] = poly_json(&params, &it.elem.x2);
                    Outcome::Beyond
                } else {
                    v["outcome"] = json!("generator");
                    let c = it.elem.x2.lead().unwrap().1;
                    v["scale"] = json!(params.inverse(c));
                    it.elem.x2.scale(params.inverse(c), p);
                    v["cycle"] = poly_json(&params, &it.elem.x2);
                    let index = new_gens[s2];
                    new_gens[s2] += 1;
                    v["gen"] = json!([s2, index]);
                    assert_eq!(res.differential(s2 as u32, index), &it.elem.x2);
                    if s2 - 1 < num_levels {
                        v["index"] = json!(before[s2 - 1] + new[s2 - 1].len());
                        new[s2 - 1].push(GbElement::new(
                            it.elem.x2.clone(),
                            Poly::monomial(ModMono::new(index as u32, Mono::ONE), 1),
                        ));
                    }
                    Outcome::NewGenerator
                };
                assert_eq!(it.ev.outcome, expected, "t={t} s={s}");
                cycles.push(v);
            }
            if !cycles.is_empty() {
                out_items.push(json!({"s": s, "kind": "cycles", "cycles": cycles}));
            }
        }
        // Check the new elements against the program's.
        for s in 0..num_levels {
            let prog = &res.level(s as u32).elements()[before[s]..];
            assert_eq!(prog.len(), new[s].len(), "t={t} s={s}");
            for (a, b) in prog.iter().zip(&new[s]) {
                assert_eq!(a.x1, b.x1, "t={t} s={s}");
                assert_eq!(a.x2, b.x2, "t={t} s={s}");
            }
        }
        for (s, &n) in new_gens.iter().enumerate() {
            assert_eq!(res.generators(s as u32).len(), n);
        }
        let counts: Vec<Value> = (0..num_levels as i32)
            .filter_map(|s| res.stats().steps().get(&(s, t)).map(|st| (s, st)))
            .map(|(s, st)| {
                json!({"s": s, "formed": st.pairs.formed, "divisible": st.pairs.divisible,
                       "chain": st.pairs.chain, "singles": st.pairs.singles})
            })
            .collect();
        let active: Vec<i32> = (-1..num_levels as i32)
            .filter(|&s| t <= n_max + s + 2)
            .collect();
        degrees.push(json!({"t": t, "items": out_items, "counts": counts, "active": active}));
    }

    let gens: Vec<Value> = (0..=num_levels)
        .map(|s| {
            Value::Array(
                res.generators(s as u32)
                    .iter()
                    .enumerate()
                    .map(|(i, g)| {
                        json!({"t": g.degree, "d": if s == 0 { Value::Null } else { poly_json(&params, res.differential(s as u32, i)) }})
                    })
                    .collect(),
            )
        })
        .collect();
    let levels: Vec<Value> = (0..num_levels)
        .map(|s| {
            Value::Array(
                res.level(s as u32)
                    .elements()
                    .iter()
                    .map(|e| {
                        let lead = e.x1.lead().unwrap().0;
                        let mut v = elem_json(&params, e);
                        v["t"] = json!(
                            params.degree(lead.mono())
                                + res.generators(s as u32)[lead.generator() as usize].degree
                        );
                        v
                    })
                    .collect(),
            )
        })
        .collect();
    let by_level: Vec<Value> = res
        .stats()
        .by_level()
        .iter()
        .chain(std::iter::once((&99, &res.stats().total())))
        .map(|(&s, st)| {
            json!({"s": s, "formed": st.pairs.formed, "divisible": st.pairs.divisible, "chain": st.pairs.chain,
                   "singles": st.pairs.singles, "reduced": st.reduced, "useless": st.useless,
                   "elements": st.new_elements, "boundary": st.boundaries, "gens": st.new_generators,
                   "beyond": st.beyond, "steps": st.steps})
        })
        .collect();
    // Replay the divisibility criterion of `PairStore::add_lead` to see which pairs it prunes.
    let mut pruned = Vec::new();
    for s in 0..num_levels {
        let level = res.level(s as u32);
        let leads = level.leads();
        let t_trunc = n_max + s as i32 + 2;
        for n in 0..leads.len() {
            let lead = leads[n];
            let gdeg = res.generators(s as u32)[lead.generator() as usize].degree;
            let mut new_pairs: Vec<Option<(i32, Mono, Mono)>> = leads[..n]
                .iter()
                .map(|l| {
                    if l.generator() != lead.generator() {
                        return None;
                    }
                    let lcm = params.lcm(l.mono(), lead.mono());
                    let t = params.degree(lcm) + gdeg;
                    (t <= t_trunc).then(|| {
                        (
                            t,
                            params.quotient(lcm, l.mono()),
                            params.quotient(lcm, lead.mono()),
                        )
                    })
                })
                .collect();
            for j in 1..n {
                let Some((tj, m1j, m2j)) = new_pairs[j] else {
                    continue;
                };
                for i in 0..j {
                    let Some((ti, m1i, m2i)) = new_pairs[i] else {
                        continue;
                    };
                    if params.divides(m2i, m2j) {
                        pruned.push(json!({"s": s, "t": tj, "i": j, "n": n, "by": i,
                            "m1": mono_json(&params, m1j), "m2": mono_json(&params, m2j),
                            "by_m1": mono_json(&params, m1i), "by_m2": mono_json(&params, m2i), "by_t": ti}));
                        new_pairs[j] = None;
                        break;
                    } else if params.divides(m2j, m2i) {
                        pruned.push(json!({"s": s, "t": ti, "i": i, "n": n, "by": j,
                            "m1": mono_json(&params, m1i), "m2": mono_json(&params, m2i),
                            "by_m1": mono_json(&params, m1j), "by_m2": mono_json(&params, m2j), "by_t": tj}));
                        new_pairs[i] = None;
                    }
                }
            }
        }
    }
    for s in 0..num_levels as i32 {
        for t in 1..=t_max {
            let expected = res
                .stats()
                .steps()
                .get(&(s, t))
                .map_or(0, |st| st.pairs.divisible);
            let found = pruned.iter().filter(|p| p["s"] == s && p["t"] == t).count() as u64;
            assert_eq!(expected, found, "divisible pairs at s={s} t={t}");
        }
    }
    let boundaries: Vec<Value> = res
        .stats()
        .steps()
        .iter()
        .filter(|(_, st)| st.boundaries > 0)
        .map(|(&(s, t), st)| json!([s, t, st.boundaries]))
        .collect();
    println!(
        "{}",
        json!({"p": p, "relations": relations.keys().collect::<Vec<_>>(), "s_max": s_max, "n_max": n_max, "degrees": degrees, "generators": gens,
               "levels": levels, "stats": by_level, "pruned": pruned, "boundaries": boundaries})
    );
}
