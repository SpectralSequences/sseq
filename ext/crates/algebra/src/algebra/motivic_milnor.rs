//! The C-motivic (over $\mathbb{C}$, prime 2) Steenrod algebra in the Milnor basis.
//!
//! This follows Kong–Lin, *Product formulas for motivic Milnor basis* (arXiv:2411.12890),
//! specialized to the C-motivic point where $\rho = 0$ (the motivic cohomology of a point
//! over $\mathbb{C}$ is $\mathbb{F}_2[\tau]$). We use the *conjugate* generators of that
//! paper, so the coproduct and Milnor-matrix product match Milnor's classical formulas on
//! the polynomial (`ξ`) part.
//!
//! The dual algebra is $A_{**} = \mathbb{F}_2[\tau][\xi_1, \xi_2, \dots] \otimes
//! E(\tau_0, \tau_1, \dots)$ with $\tau_i^2 = \tau \xi_{i+1}$ (the $\rho = 0$ reduction of
//! $\tau_i^2 = \tau\xi_{i+1} + \rho\tau_{i+1}$). Bidegrees: $|\xi_i| = (2(2^i-1), 2^i-1)$,
//! $|\tau_i| = (2^{i+1}-1, 2^i-1)$, $|\tau| = (0, -1)$; see
//! [`combinatorics::motivic_bidegree`](super::combinatorics::motivic_bidegree).
//!
//! Basis elements are $Q(E)P(R)$ with $E \in \mathrm{Seq}_1$ (entries in $\{0,1\}$, encoded
//! as a bitmask: bit $i$ is $\tau_i$) and $R \in \mathrm{Seq}$ (the exponent vector of the
//! $\xi_i$). This module currently implements the combinatorial core and the $\tau(S)$
//! rewrite (Theorem 3.4); the coproduct (Cor. 4.4) and the product (Theorem 5.1) build on
//! it.

use fp::prime::Binomial;

/// $\Sigma(R) = \sum_i r_i$.
fn sigma(r: &[u32]) -> u32 {
    r.iter().sum()
}

/// $\Sigma_2(R) = \sum_i r_i 2^i$.
fn sigma2(r: &[u32]) -> u64 {
    r.iter().enumerate().map(|(i, &v)| (v as u64) << i).sum()
}

/// The coefficient $c(S, R)$ of Notation 3.3 / Theorem 3.4, reduced mod 2.
///
/// $c(S, R) = \prod_{n \ge 1} \binom{\lfloor \sum_{i=0}^{n-1} 2^{i-n}(s_i - r_i)\rfloor}{r_n}$
/// when $r_0 = 0$, and $0$ otherwise. The inner floor is computed exactly with integer
/// (Euclidean) division: $\lfloor \sum_{i=0}^{n-1} 2^{i-n}(s_i-r_i)\rfloor
/// = \lfloor (\sum_{i=0}^{n-1} (s_i-r_i)2^i) / 2^n \rfloor$. Each factor is
/// [`Binomial::binomial2`], which is $0$ when the top is negative or below the bottom.
fn c_coeff(s: &[u32], r: &[u32]) -> u32 {
    if r.first().copied().unwrap_or(0) > 0 {
        return 0;
    }
    let get = |seq: &[u32], i: usize| seq.get(i).copied().unwrap_or(0) as i32;
    let len = s.len().max(r.len());
    let mut prod = 1u32;
    for n in 1..len {
        let mut num: i32 = 0;
        for i in 0..n {
            num += (get(s, i) - get(r, i)) << i;
        }
        let floor = num.div_euclid(1i32 << n);
        prod *= i32::binomial2(floor, get(r, n)) as u32;
        if prod == 0 {
            return 0;
        }
    }
    prod
}

/// A term $\tau^{\text{tau\_pow}} \cdot Q(E) P(R)$ (mod 2, C-motivic so no $\rho$), the output
/// of rewriting a monomial into the Milnor basis. `e_mask` encodes $E \in \mathrm{Seq}_1$.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotivicTerm {
    pub tau_pow: u32,
    pub e_mask: u32,
    pub r: Vec<u32>,
}

/// Rewrite $\tau(S) = \tau_0^{s_0}\tau_1^{s_1}\cdots$ (with `s` an arbitrary exponent vector)
/// into the C-motivic Milnor basis, using $\tau_i^2 = \tau\xi_{i+1}$ (Theorem 3.4 at
/// $\rho = 0$):
/// $$\tau(S) = \sum_{\substack{E \in \mathrm{Seq}_1,\ R \in \mathrm{Seq}\\ \Sigma_2(E)+\Sigma_2(R)=\Sigma_2(S)\\ \Sigma(E)+2\Sigma(R)=\Sigma(S)}} c(S,R)\, \tau^{\Sigma(R)}\, \tau(E)\xi(R),$$
/// where the second constraint is the vanishing of the $\rho$-exponent
/// $\Sigma(S) - \Sigma(E) - 2\Sigma(R)$.
pub fn rewrite_tau(s: &[u32]) -> Vec<MotivicTerm> {
    let target2 = sigma2(s);
    let target_sum = sigma(s);
    // Highest index we must consider: any nonzero e_i or r_i contributes at least 2^i, so
    // 2^i <= target2.
    let max_idx = if target2 == 0 {
        0
    } else {
        (63 - target2.leading_zeros()) as usize
    };

    let mut out = Vec::new();
    let mut e = vec![0u32; max_idx + 1];
    let mut r = vec![0u32; max_idx + 1];
    rewrite_tau_dfs(s, 0, max_idx, target2, target_sum, &mut e, &mut r, &mut out);
    out
}

#[allow(clippy::too_many_arguments)]
fn rewrite_tau_dfs(
    s: &[u32],
    idx: usize,
    max_idx: usize,
    rem2: u64,       // remaining Σ2(E) + Σ2(R) budget
    target_sum: u32, // Σ(S), for the ρ = 0 constraint Σ(E) + 2Σ(R) = Σ(S)
    e: &mut [u32],
    r: &mut [u32],
    out: &mut Vec<MotivicTerm>,
) {
    if idx > max_idx {
        if rem2 != 0 {
            return;
        }
        // ρ-exponent must vanish (C-motivic): Σ(E) + 2Σ(R) = Σ(S).
        if sigma(e) + 2 * sigma(r) != target_sum {
            return;
        }
        let coeff = c_coeff(s, r);
        if coeff == 0 {
            return;
        }
        let mut e_mask = 0u32;
        for (i, &ei) in e.iter().enumerate() {
            if ei != 0 {
                e_mask |= 1 << i;
            }
        }
        let mut r_trimmed = r.to_vec();
        while let Some(&0) = r_trimmed.last() {
            r_trimmed.pop();
        }
        out.push(MotivicTerm {
            tau_pow: sigma(r),
            e_mask,
            r: r_trimmed,
        });
        return;
    }

    let weight = 1u64 << idx;
    // e_idx ∈ {0, 1} (E ∈ Seq_1), r_idx ≥ 0, with (e_idx + r_idx) * 2^idx ≤ rem2.
    for e_idx in 0..=1u32 {
        if (e_idx as u64) * weight > rem2 {
            break;
        }
        let after_e = rem2 - (e_idx as u64) * weight;
        let max_r = (after_e / weight) as u32;
        for r_idx in 0..=max_r {
            e[idx] = e_idx;
            r[idx] = r_idx;
            rewrite_tau_dfs(
                s,
                idx + 1,
                max_idx,
                after_e - (r_idx as u64) * weight,
                target_sum,
                e,
                r,
                out,
            );
        }
    }
    e[idx] = 0;
    r[idx] = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_tau_square_free_is_identity() {
        // A square-free τ(E) is already a basis element: τ_0 τ_2 → Q(E)P(0), τ^0.
        let terms = rewrite_tau(&[1, 0, 1]);
        assert_eq!(
            terms,
            vec![MotivicTerm {
                tau_pow: 0,
                e_mask: 0b101,
                r: vec![],
            }]
        );
    }

    #[test]
    fn test_rewrite_tau_example_3_1() {
        // Kong–Lin Example 3.1, specialized to ρ = 0 (C-motivic):
        //   τ_0^2 τ_1 = τ · τ_1 ξ_1
        // Encoded: S = (2, 1); expected single term τ^1 · Q({1}) P((0,1)).
        let terms = rewrite_tau(&[2, 1]);
        assert_eq!(
            terms,
            vec![MotivicTerm {
                tau_pow: 1,
                e_mask: 0b10,  // τ_1
                r: vec![0, 1], // ξ_1^1
            }],
            "τ_0^2 τ_1 = τ τ_1 ξ_1 (ρ=0)"
        );

        //   τ_0^4 = τ^2 ξ_1^2
        // Encoded: S = (4); expected single term τ^2 · Q(∅) P((0,2)).
        let terms = rewrite_tau(&[4]);
        assert_eq!(
            terms,
            vec![MotivicTerm {
                tau_pow: 2,
                e_mask: 0,
                r: vec![0, 2], // ξ_1^2
            }],
            "τ_0^4 = τ^2 ξ_1^2 (ρ=0)"
        );
    }

    #[test]
    fn test_rewrite_tau_unit() {
        assert_eq!(
            rewrite_tau(&[]),
            vec![MotivicTerm {
                tau_pow: 0,
                e_mask: 0,
                r: vec![],
            }]
        );
    }
}
