use std::sync::Arc;

use fp::{
    prime::{Binomial, Prime, ValidPrime},
    vector::FpSliceMut,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    algebra::{
        adem_algebra::AdemBasisElement,
        milnor_algebra::{MilnorBasisElement, PPartEntry},
        AdemAlgebra, Algebra, GeneratedAlgebra, MilnorAlgebra, SteenrodAlgebra,
    },
    module::{Module, ZeroModule},
};

/// The classifying space $B\mathbb{Z}/p$.
///
/// At $p = 2$, this is equivalent to `RealProjectiveSpace` ($BC_2 = \mathbb{RP}^\infty$).
///
/// At odd $p$, $H^*(B\mathbb{Z}/p; \mathbb{F}_p) = E[y] \otimes \mathbb{F}_p[x]$ where
/// $|y| = 1$ and $|x| = 2$. There is one basis element in each degree:
/// - Even degree $2k$: $x^k$
/// - Odd degree $2k+1$: $x^k y$
pub struct BClassifyingSpace<A: Algebra> {
    algebra: Arc<A>,
    pub min: i32,
    pub max: Option<i32>,
}

impl<A: Algebra> std::fmt::Display for BClassifyingSpace<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(max) = self.max {
            write!(f, "BCp^{max}_{}", self.min)
        } else {
            write!(f, "BCp_{}", self.min)
        }
    }
}

impl<A: Algebra> PartialEq for BClassifyingSpace<A> {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

impl<A: Algebra> Eq for BClassifyingSpace<A> {}

impl<A: Algebra> Module for BClassifyingSpace<A>
where
    for<'a> &'a A: TryInto<&'a SteenrodAlgebra>,
{
    type Algebra = A;

    fn algebra(&self) -> Arc<A> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        self.min
    }

    fn max_computed_degree(&self) -> i32 {
        i32::MAX
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree < self.min {
            return 0;
        }
        if let Some(m) = self.max {
            if degree > m {
                return 0;
            }
        }
        1
    }

    fn basis_element_to_string(&self, degree: i32, _idx: usize) -> String {
        if degree % 2 == 0 {
            format!("x^{{{}}}", degree / 2)
        } else {
            let k = (degree - 1) / 2;
            if k == 0 {
                "y".to_string()
            } else {
                format!("x^{{{}}} y", k)
            }
        }
    }

    fn act_on_basis(
        &self,
        mut result: FpSliceMut,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        _mod_index: usize,
    ) {
        assert!(op_index < self.algebra().dimension(op_degree));
        assert!(_mod_index < self.dimension(mod_degree));

        let output_degree = mod_degree + op_degree;

        if op_degree == 0 || coeff == 0 || self.dimension(output_degree) == 0 {
            return;
        }

        let p = self.prime();

        if p == 2 {
            let nonzero = match (&*self.algebra()).try_into() {
                Ok(SteenrodAlgebra::AdemAlgebra(a)) => {
                    coef_adem_rp(a, op_degree, op_index, mod_degree)
                }
                Ok(SteenrodAlgebra::MilnorAlgebra(a)) => {
                    coef_milnor_rp(a, op_degree, op_index, mod_degree)
                }
                Err(_) => unreachable!(),
            };
            if nonzero {
                result.add_basis_element(0, 1);
            }
        } else {
            let p_u32 = p.as_u32();
            let c = match (&*self.algebra()).try_into() {
                Ok(SteenrodAlgebra::AdemAlgebra(a)) => {
                    coef_adem_bcp(a, p, op_degree, op_index, mod_degree)
                }
                Ok(SteenrodAlgebra::MilnorAlgebra(a)) => {
                    coef_milnor_bcp(a, p, op_degree, op_index, mod_degree)
                }
                Err(_) => unreachable!(),
            };
            if c != 0 {
                result.add_basis_element(0, (coeff * c) % p_u32);
            }
        }
    }

    fn max_degree(&self) -> Option<i32> {
        self.max
    }
}

// ---- p=2 helpers (same as RealProjectiveSpace) ----

fn coef_adem_rp(algebra: &AdemAlgebra, op_deg: i32, op_idx: usize, mut j: i32) -> bool {
    let p = ValidPrime::new(2);
    let elt: &AdemBasisElement = algebra.basis_element_from_index(op_deg, op_idx);
    for i in elt.ps.iter().rev() {
        let c = if j >= 0 {
            i32::binomial(p, j, *i as i32)
        } else {
            i32::binomial(p, -j + (*i as i32) - 1, *i as i32)
        };
        if c == 0 {
            return false;
        }
        j += *i as i32;
    }
    true
}

fn coef_milnor_rp(
    algebra: &MilnorAlgebra,
    op_deg: i32,
    op_idx: usize,
    mut mod_degree: i32,
) -> bool {
    if mod_degree == 0 {
        return false;
    }

    let elt: &MilnorBasisElement = algebra.basis_element_from_index(op_deg, op_idx);

    let sum: PPartEntry = elt.p_part.iter().sum();
    if mod_degree < 0 {
        mod_degree = sum as i32 - mod_degree - 1;
    } else if mod_degree < sum as i32 {
        return false;
    }

    let mod_degree = mod_degree as PPartEntry;

    let mut list = Vec::with_capacity(elt.p_part.len() + 1);
    list.push(mod_degree - sum);
    list.extend_from_slice(&elt.p_part);

    PPartEntry::multinomial2(&list) == 1
}

// ---- Odd prime helpers for BCp ----

/// Compute the coefficient of the Adem operation on BCp at odd primes.
///
/// Tracks parity: even = x^k, odd = x^k·y.
/// P^i on even/odd: C(k, i) * x^{k+i(p-1)} (parity unchanged)
/// β on even: 0
/// β on odd x^k·y: x^{k+1} (becomes even)
fn coef_adem_bcp(
    algebra: &AdemAlgebra,
    p: ValidPrime,
    op_deg: i32,
    op_idx: usize,
    mod_degree: i32,
) -> u32 {
    let elt: &AdemBasisElement = algebra.basis_element_from_index(op_deg, op_idx);
    let p_u32 = p.as_u32();
    let mut poly_deg: i32 = mod_degree / 2;
    let mut is_odd = mod_degree % 2 != 0;

    let num_ps = elt.ps.len();
    let mut running_coeff: u32 = 1;

    // Trailing bockstein (bit index = num_ps)
    if elt.bocksteins & (1 << num_ps) != 0 {
        if !is_odd {
            return 0;
        }
        poly_deg += 1;
        is_odd = false;
    }

    // Process P^{i_j} and bockstein bit j, right-to-left
    for j in (0..num_ps).rev() {
        let i = elt.ps[j] as i32;
        if i > 0 {
            let c = if poly_deg >= 0 {
                i32::binomial_odd(p, poly_deg, i) as u32
            } else {
                let sign = if i % 2 == 0 { 1u32 } else { p_u32 - 1 };
                let binom = i32::binomial_odd(p, -poly_deg + i - 1, i) as u32;
                (sign * binom) % p_u32
            };
            if c == 0 {
                return 0;
            }
            running_coeff = (running_coeff * c) % p_u32;
            poly_deg += i * (p.as_i32() - 1);
        }

        if elt.bocksteins & (1 << j) != 0 {
            if !is_odd {
                return 0;
            }
            poly_deg += 1;
            is_odd = false;
        }
    }

    running_coeff
}

/// Compute the coefficient of the Milnor operation on BCp at odd primes.
fn coef_milnor_bcp(
    algebra: &MilnorAlgebra,
    p: ValidPrime,
    op_deg: i32,
    op_idx: usize,
    mod_degree: i32,
) -> u32 {
    let elt: &MilnorBasisElement = algebra.basis_element_from_index(op_deg, op_idx);
    let is_odd = mod_degree % 2 != 0;
    let poly_deg = mod_degree / 2;
    let q_bits = elt.q_part.count_ones();

    if q_bits >= 2 {
        return 0;
    }

    if q_bits == 1 {
        if !is_odd {
            return 0;
        }
        // Apply P-part to x^{poly_deg} (the polynomial part, ignoring parity)
        let p_coeff = coef_p_part_bcp(algebra, p, &elt.p_part, poly_deg);
        if p_coeff == 0 {
            return 0;
        }
        // Q_j on the resulting odd element gives coefficient 1, so total is p_coeff
        p_coeff
    } else {
        // q_part = 0: pure P operations
        coef_p_part_bcp(algebra, p, &elt.p_part, poly_deg)
    }
}

/// Compute the coefficient of P(r_1, r_2, ...) acting on x^{poly_deg} (ignoring parity).
fn coef_p_part_bcp(
    algebra: &MilnorAlgebra,
    p: ValidPrime,
    p_part: &[PPartEntry],
    poly_deg: i32,
) -> u32 {
    let p_u32 = p.as_u32();

    if p_part.is_empty() || p_part.iter().all(|&x| x == 0) {
        return 1;
    }

    if p_part.len() == 1 || p_part[1..].iter().all(|&x| x == 0) {
        let r = p_part[0];
        if r == 0 {
            return 1;
        }
        let i = r as i32;
        return if poly_deg >= 0 {
            i32::binomial_odd(p, poly_deg, i) as u32
        } else {
            let sign = if i % 2 == 0 { 1u32 } else { p_u32 - 1 };
            let binom = i32::binomial_odd(p, -poly_deg + i - 1, i) as u32;
            (sign * binom) % p_u32
        };
    }

    // General case: use decomposition
    let mut elt = MilnorBasisElement {
        q_part: 0,
        p_part: p_part.to_vec(),
        degree: 0,
    };
    elt.compute_degree(p);
    let op_deg = elt.degree;
    let op_idx = algebra.basis_element_to_index(&elt);

    let mod_degree = 2 * poly_deg;

    coef_milnor_bcp_decompose(algebra, p, op_deg, op_idx, mod_degree)
}

/// Use decompose_basis_element to compute the Milnor action on BCp at odd primes.
fn coef_milnor_bcp_decompose(
    algebra: &MilnorAlgebra,
    p: ValidPrime,
    op_deg: i32,
    op_idx: usize,
    mod_degree: i32,
) -> u32 {
    let decomposition = algebra.decompose_basis_element(op_deg, op_idx);
    let p_u32 = p.as_u32();

    let mut total: u32 = 0;

    for (c, (left_deg, left_idx), (right_deg, right_idx)) in decomposition {
        let right_coeff = coef_milnor_bcp(algebra, p, right_deg, right_idx, mod_degree);
        if right_coeff == 0 {
            continue;
        }

        let intermediate_degree = mod_degree + right_deg;

        let left_coeff = coef_milnor_bcp(algebra, p, left_deg, left_idx, intermediate_degree);
        if left_coeff == 0 {
            continue;
        }

        total = (total + c * right_coeff * left_coeff) % p_u32;
    }
    total
}

impl<A: Algebra> ZeroModule for BClassifyingSpace<A>
where
    for<'a> &'a A: TryInto<&'a SteenrodAlgebra>,
{
    fn zero_module(algebra: Arc<A>, min_degree: i32) -> Self {
        Self::new(algebra, min_degree, Some(min_degree - 1))
    }
}

impl<A: Algebra> BClassifyingSpace<A>
where
    for<'a> &'a A: TryInto<&'a SteenrodAlgebra>,
{
    pub fn new(algebra: Arc<A>, min: i32, max: Option<i32>) -> Self {
        assert!(
            (&*algebra).try_into().is_ok(),
            "BClassifyingSpace only supports Steenrod Algebra"
        );

        if let Some(max) = max {
            assert!(max >= min);
        }
        Self { algebra, min, max }
    }
}

#[derive(Deserialize, Debug)]
struct BCpSpec {
    min: i32,
    max: Option<i32>,
}

impl<A: Algebra> BClassifyingSpace<A> {
    pub fn from_json(algebra: Arc<A>, json: &Value) -> anyhow::Result<Self> {
        let spec: BCpSpec = BCpSpec::deserialize(json)?;

        Ok(Self {
            algebra,
            min: spec.min,
            max: spec.max,
        })
    }

    pub fn to_json(&self, json: &mut Value) {
        json["name"] = Value::String(self.to_string());
        json["type"] = Value::from("classifying space BCp");
        json["min"] = Value::from(self.min);
        if let Some(max) = self.max {
            json["max"] = Value::from(max);
        }
    }
}
