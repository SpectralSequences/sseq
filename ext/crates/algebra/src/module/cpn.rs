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

/// Complex projective space $\mathbb{CP}_{\mathrm{min}}^{\mathrm{max}}$.
///
/// The cohomology is the subquotient of $\mathbb{F}_p[u^\pm]$ (where $|u| = 2$) given by elements
/// of polynomial degree between `min` and `max` (inclusive). The topological degree of $u^k$ is
/// $2k$.
pub struct ComplexProjectiveSpace<A: Algebra> {
    algebra: Arc<A>,
    pub min: i32,
    pub max: Option<i32>,
}

impl<A: Algebra> std::fmt::Display for ComplexProjectiveSpace<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(max) = self.max {
            write!(f, "CP^{max}_{}", self.min)
        } else {
            write!(f, "CP_{}", self.min)
        }
    }
}

impl<A: Algebra> PartialEq for ComplexProjectiveSpace<A> {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

impl<A: Algebra> Eq for ComplexProjectiveSpace<A> {}

impl<A: Algebra> Module for ComplexProjectiveSpace<A>
where
    for<'a> &'a A: TryInto<&'a SteenrodAlgebra>,
{
    type Algebra = A;

    fn algebra(&self) -> Arc<A> {
        Arc::clone(&self.algebra)
    }

    fn min_degree(&self) -> i32 {
        2 * self.min
    }

    fn max_computed_degree(&self) -> i32 {
        i32::MAX
    }

    fn dimension(&self, degree: i32) -> usize {
        if degree % 2 != 0 {
            return 0;
        }
        let poly_deg = degree / 2;
        if poly_deg < self.min {
            return 0;
        }
        if let Some(m) = self.max {
            if poly_deg > m {
                return 0;
            }
        }
        1
    }

    fn basis_element_to_string(&self, degree: i32, _idx: usize) -> String {
        format!("u^{{{}}}", degree / 2)
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
        let p_u32 = p.as_u32();

        match (&*self.algebra()).try_into() {
            Ok(SteenrodAlgebra::AdemAlgebra(a)) => {
                let c = coef_adem_cp(a, p, op_degree, op_index, mod_degree);
                if c != 0 {
                    result.add_basis_element(0, (coeff * c) % p_u32);
                }
            }
            Ok(SteenrodAlgebra::MilnorAlgebra(a)) => {
                let c = coef_milnor_cp(a, p, op_degree, op_index, mod_degree);
                if c != 0 {
                    result.add_basis_element(0, (coeff * c) % p_u32);
                }
            }
            Err(_) => unreachable!(),
        }
    }

    fn max_degree(&self) -> Option<i32> {
        self.max.map(|m| 2 * m)
    }
}

/// Compute the coefficient of the Adem operation on u^{poly_deg} in CP.
///
/// At p=2: Sq^i(u^k) = C(k, i/2) * u^{k + i/2} (and 0 if i is odd).
/// At odd p: P^i(u^k) = C(k, i) * u^{k + i(p-1)}, and β acts as 0.
fn coef_adem_cp(
    algebra: &AdemAlgebra,
    p: ValidPrime,
    op_deg: i32,
    op_idx: usize,
    mod_degree: i32,
) -> u32 {
    let elt: &AdemBasisElement = algebra.basis_element_from_index(op_deg, op_idx);
    let mut poly_deg = mod_degree / 2;
    let p_u32 = p.as_u32();

    if p == 2 {
        for i in elt.ps.iter().rev() {
            let i = *i;
            if i % 2 != 0 {
                return 0;
            }
            let half_i = i as i32 / 2;
            let c = if poly_deg >= 0 {
                i32::binomial(p, poly_deg, half_i)
            } else {
                i32::binomial(p, -poly_deg + half_i - 1, half_i)
            };
            if c == 0 {
                return 0;
            }
            poly_deg += half_i;
        }
        1
    } else {
        if elt.bocksteins != 0 {
            return 0;
        }
        let mut running_coeff: u32 = 1;
        for i in elt.ps.iter().rev() {
            let i = *i as i32;
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
        running_coeff
    }
}

/// Compute the coefficient of the Milnor operation on u^{poly_deg} in CP.
fn coef_milnor_cp(
    algebra: &MilnorAlgebra,
    p: ValidPrime,
    op_deg: i32,
    op_idx: usize,
    mod_degree: i32,
) -> u32 {
    let elt: &MilnorBasisElement = algebra.basis_element_from_index(op_deg, op_idx);
    let p_u32 = p.as_u32();

    if p == 2 {
        if op_deg % 2 != 0 {
            return 0;
        }

        let mut top_deg = mod_degree;
        if top_deg == 0 {
            return 0;
        }

        let sum: PPartEntry = elt.p_part.iter().sum();
        if top_deg < 0 {
            top_deg = sum as i32 - top_deg - 1;
        } else if top_deg < sum as i32 {
            return 0;
        }

        let top_deg = top_deg as PPartEntry;

        let mut list = Vec::with_capacity(elt.p_part.len() + 1);
        list.push(top_deg - sum);
        list.extend_from_slice(&elt.p_part);

        PPartEntry::multinomial2(&list)
    } else {
        if elt.q_part != 0 {
            return 0;
        }

        if elt.p_part.len() <= 1 {
            let r = if elt.p_part.is_empty() {
                0
            } else {
                elt.p_part[0]
            };
            if r == 0 {
                return 1;
            }
            let poly_deg = mod_degree / 2;
            return if poly_deg >= 0 {
                i32::binomial_odd(p, poly_deg, r as i32) as u32
            } else {
                let i = r as i32;
                let sign = if i % 2 == 0 { 1u32 } else { p_u32 - 1 };
                let binom = i32::binomial_odd(p, -poly_deg + i - 1, i) as u32;
                (sign * binom) % p_u32
            };
        }

        coef_milnor_cp_decompose(algebra, p, op_deg, op_idx, mod_degree)
    }
}

/// Use decompose_basis_element to compute the Milnor action on CP at odd primes.
fn coef_milnor_cp_decompose(
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
        let right_coeff = coef_milnor_cp(algebra, p, right_deg, right_idx, mod_degree);
        if right_coeff == 0 {
            continue;
        }

        let intermediate_degree = mod_degree + right_deg;
        if intermediate_degree % 2 != 0 {
            continue;
        }

        let left_coeff = coef_milnor_cp(algebra, p, left_deg, left_idx, intermediate_degree);
        if left_coeff == 0 {
            continue;
        }

        total = (total + c * right_coeff * left_coeff) % p_u32;
    }
    total
}

impl<A: Algebra> ZeroModule for ComplexProjectiveSpace<A>
where
    for<'a> &'a A: TryInto<&'a SteenrodAlgebra>,
{
    fn zero_module(algebra: Arc<A>, min_degree: i32) -> Self {
        Self::new(algebra, min_degree / 2, Some(min_degree / 2 - 1))
    }
}

impl<A: Algebra> ComplexProjectiveSpace<A>
where
    for<'a> &'a A: TryInto<&'a SteenrodAlgebra>,
{
    pub fn new(algebra: Arc<A>, min: i32, max: Option<i32>) -> Self {
        assert!(
            (&*algebra).try_into().is_ok(),
            "Complex Projective Space only supports Steenrod Algebra"
        );

        if let Some(max) = max {
            assert!(max >= min);
        }
        Self { algebra, min, max }
    }
}

#[derive(Deserialize, Debug)]
struct CPSpec {
    min: i32,
    max: Option<i32>,
}

impl<A: Algebra> ComplexProjectiveSpace<A> {
    pub fn from_json(algebra: Arc<A>, json: &Value) -> anyhow::Result<Self> {
        let spec: CPSpec = CPSpec::deserialize(json)?;

        Ok(Self {
            algebra,
            min: spec.min,
            max: spec.max,
        })
    }

    pub fn to_json(&self, json: &mut Value) {
        json["name"] = Value::String(self.to_string());
        json["type"] = Value::from("complex projective space");
        json["min"] = Value::from(self.min);
        if let Some(max) = self.max {
            json["max"] = Value::from(max);
        }
    }
}
