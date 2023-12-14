use fp::vector::FpVector;
use once::OnceVec;

use fp::prime::{minus_one_to_the_n, Binomial, Prime, ValidPrime};
use fp::{const_for, MAX_MULTINOMIAL_LEN, NUM_PRIMES, PRIMES, PRIME_TO_INDEX_MAP};

pub const MAX_XI_TAU: usize = MAX_MULTINOMIAL_LEN;

/// If p is the nth prime, then `XI_DEGREES[n][i - 1]` is the degree of $ξ_i$ at the prime p divided by
/// q, where q = 2p - 2 if p != 2 and 1 if p = 2.
const XI_DEGREES: [[i32; MAX_XI_TAU]; NUM_PRIMES] = {
    let mut res = [[0; MAX_XI_TAU]; NUM_PRIMES];
    const_for! { p_idx in 0 .. NUM_PRIMES {
        let p = PRIMES[p_idx];
        let mut p_to_the_i = p;
        const_for! { x in 0 .. MAX_XI_TAU {
            res[p_idx][x] = ((p_to_the_i - 1) / (p - 1)) as i32;
            // At some point the powers overflow. The values are not going to be useful, so use
            // something replacement that is suitable for const evaluation.
            p_to_the_i = p_to_the_i.overflowing_mul(p).0;
        }}
    }}
    res
};

/// If p is the nth prime, then `TAU_DEGREES[n][i]` is the degree of $τ_i$ at the prime p. Its value is
/// nonsense at the prime 2
const TAU_DEGREES: [[i32; MAX_XI_TAU]; NUM_PRIMES] = {
    let mut res = [[0; MAX_XI_TAU]; NUM_PRIMES];
    const_for! { p_idx in 0 .. NUM_PRIMES {
        let p = PRIMES[p_idx];
        let mut p_to_the_i: u32 = 1;
        const_for! { x in 0 .. MAX_XI_TAU {
            res[p_idx][x] = (2_u32.overflowing_mul(p_to_the_i).0 - 1) as i32;
            p_to_the_i = p_to_the_i.overflowing_mul(p).0;
        }}
    }}
    res
};

pub fn adem_relation_coefficient(p: ValidPrime, x: u32, y: u32, j: u32, e1: u32, e2: u32) -> u32 {
    let pi32 = p.as_i32();
    let x = x as i32;
    let y = y as i32;
    let j = j as i32;
    let e1 = e1 as i32;
    let e2 = e2 as i32;
    let mut c = i32::binomial(p, (y - j) * (pi32 - 1) + e1 - 1, x - pi32 * j - e2) as u32;
    if c == 0 {
        return 0;
    }
    c *= minus_one_to_the_n(p, (x + j) + e2);
    c % p
}

pub fn inadmissible_pairs(p: ValidPrime, generic: bool, degree: i32) -> Vec<(u32, u32, u32)> {
    let degree = degree as u32;
    let q = if generic { 2 * p - 2 } else { 1 };
    // (i, b, j) means P^i P^j if b = 0, or P^i b P^j if b = 1.
    let mut inadmissible_pairs = Vec::new();

    // Since |P^i| is always a multiple of q, we have a relation only if degree = 0 or 1 mod q.
    // If it is 0, then there is no Bockstein. Otherwise, there is.
    if degree % q == 0 {
        let degq = degree / q;
        // We want P^i P^j to be inadmissible, so i < p * j. This translates to
        // i < p * degq /(p + 1). Since Rust automatically rounds *down*, but we want to round
        // up instead, we use i < (p * degq + p)/(p + 1).
        for i in 1..(p * degq + p) / (p + 1) {
            inadmissible_pairs.push((i, 0, degq - i));
        }
    } else if degree % q == 1 {
        let degq = degree / q; // Since we round down, this is actually (degree - 1)/q
                               // We want P^i b P^j to be inadmissible, so i < p * j + 1. This translates to
                               // i < (p * degq + 1)/(p + 1). Since Rust automatically rounds *down*, but we want to round
                               // up instead, we use i < (p * degq + p + 1)/(p + 1).
        for i in 1..(p * degq + p + 1) / (p + 1) {
            inadmissible_pairs.push((i, 1, degq - i));
        }
    }
    inadmissible_pairs
}

pub fn tau_degrees(p: ValidPrime) -> &'static [i32] {
    &TAU_DEGREES[PRIME_TO_INDEX_MAP[p.as_usize()]]
}

pub fn xi_degrees(p: ValidPrime) -> &'static [i32] {
    &XI_DEGREES[PRIME_TO_INDEX_MAP[p.as_usize()]]
}

pub struct TruncatedPolynomialMonomialBasis {
    p: ValidPrime,
    /// degree => (first_index, number_of_gens)
    pub gens: OnceVec<(usize, usize)>,
    /// index ==> degree
    pub gen_degrees: OnceVec<i32>,
    /// degree => max_part => list of partitions with maximum part max_part
    parts_by_max: OnceVec<Vec<Vec<FpVector>>>,
    pub parts: OnceVec<Vec<FpVector>>,
}

impl TruncatedPolynomialMonomialBasis {
    pub fn new(p: ValidPrime) -> Self {
        let gens = OnceVec::new();
        gens.push((0, 0));
        let parts_by_max = OnceVec::new();
        parts_by_max.push(vec![vec![FpVector::new(p, 0)]]);
        let parts = OnceVec::new();
        parts.push(vec![FpVector::new(p, 0)]);
        Self {
            p,
            gens,
            gen_degrees: OnceVec::new(),
            parts_by_max,
            parts,
        }
    }

    pub fn generators(&self, degree: i32) -> usize {
        self.gens[degree as usize].1
    }

    pub fn generators_up_to_degree(&self, degree: i32) -> usize {
        self.gens[degree as usize].0 + self.gens[degree as usize].1
    }

    pub fn gen_deg_idx_to_internal_idx(&self, degree: i32, idx: usize) -> usize {
        self.gens[degree as usize].0 + idx
    }

    pub fn internal_idx_to_gen_deg(&self, idx: usize) -> (i32, usize) {
        let degree = self.gen_degrees[idx];
        let idx = idx - self.gens[degree as usize].0;
        (degree, idx)
    }

    pub fn parts(&self, degree: i32) -> &Vec<FpVector> {
        &self.parts[degree as usize]
    }

    pub fn add_gens_and_calculate_parts(&self, degree: i32, new_gens: usize) {
        assert!(degree as usize == self.gens.len());
        let p = self.p;
        let idx = self.gens[degree as usize - 1].0 + self.gens[degree as usize - 1].1;
        self.gens.push((idx, new_gens));
        let mut new_parts_by_max = Vec::new();
        let mut new_parts = Vec::new();
        new_parts_by_max.push(vec![]);
        for _ in 0..new_gens {
            self.gen_degrees.push(degree);
        }
        // println!("degree : {}", degree);
        for last_deg in 1..=degree {
            let mut partitions_cur_max_part = Vec::new();
            let (offset, num_gens) = self.gens[last_deg as usize];
            if num_gens == 0 {
                new_parts_by_max.push(partitions_cur_max_part);
                continue;
            }
            let rest_deg = degree - last_deg;
            // println!("  last_deg : {} rest_deg : {}", last_deg, rest_deg );
            for (max_part, part_list) in self.parts_by_max[rest_deg as usize].iter().enumerate() {
                // println!("    max_part : {}", max_part);
                if max_part > last_deg as usize {
                    break;
                }
                for part in part_list {
                    let mut last_nonzero_entry = 0;
                    for d in (0..num_gens).rev() {
                        let idx = offset + d;
                        if idx >= part.len() {
                            continue;
                        }
                        if part.entry(idx) != 0 {
                            last_nonzero_entry = d;
                            break;
                        }
                    }
                    // println!("      part : {}", part);
                    // println!("      lnze : {}", last_nonzero_entry);
                    if part.len() <= offset + last_nonzero_entry
                        || part.entry(offset + last_nonzero_entry) < p - 1
                    {
                        let mut new_part = part.clone();
                        new_part.extend_len(offset + num_gens);
                        new_part.add_basis_element(offset + last_nonzero_entry, 1);
                        new_parts.push(new_part.clone());
                        // println!("        new_part A: {}", new_part);
                        partitions_cur_max_part.push(new_part);
                    }
                    for d in last_nonzero_entry + 1..num_gens {
                        // println!()
                        let mut new_part = part.clone();
                        new_part.extend_len(offset + num_gens);
                        new_part.add_basis_element(offset + d, 1);
                        new_parts.push(new_part.clone());
                        // println!("        new_part B: {}", new_part);
                        partitions_cur_max_part.push(new_part);
                    }
                }
            }
            new_parts_by_max.push(partitions_cur_max_part);
        }
        self.parts.push(new_parts);
        self.parts_by_max.push(new_parts_by_max);
    }
}

pub struct PartitionIterator<'a> {
    remaining: i32,      // leftover degree
    parts: &'a [i32],    // list of part sizes to use
    partition: Vec<u32>, // current partition
    initial: bool,       //
}

impl<'a> PartitionIterator<'a> {
    pub fn new(max_degree: i32, num_parts: u32, parts: &'a [i32]) -> Self {
        let mut remaining = max_degree;
        let mut partition = vec![0; parts.len()];
        if parts.is_empty() {
            return Self {
                remaining: -1,
                parts,
                partition,
                initial: true,
            };
        }

        let idx = (parts.len() != 1) as usize;
        partition[idx] = num_parts;
        remaining -= num_parts as i32 * parts[idx];
        Self {
            remaining,
            parts,
            partition,
            initial: true,
        }
    }

    pub fn search(&mut self) -> bool {
        let mut cur_idx = 0;
        for i in (1..self.partition.len()).rev() {
            if self.partition[i] != 0 {
                cur_idx = i;
                break;
            }
        }
        if cur_idx == 0 {
            return false;
        }
        let dec_step = if self.remaining < 0 {
            let part_delta = self.parts[cur_idx] - self.parts[0];
            (-self.remaining + part_delta - 1) / part_delta
        } else {
            1
        };

        self.partition[cur_idx] -= dec_step as u32;
        self.remaining += dec_step * self.parts[cur_idx];

        self.partition[0] += dec_step as u32;
        self.remaining -= dec_step * self.parts[0];

        match self.remaining.cmp(&0) {
            std::cmp::Ordering::Less => {
                unreachable!()
            }
            std::cmp::Ordering::Equal => {
                return true;
            }
            std::cmp::Ordering::Greater => {}
        }
        if cur_idx + 1 == self.parts.len() {
            true
        } else {
            let next_part = self.parts[cur_idx + 1];
            let inc_step =
                (self.partition[0] as i32).min((self.remaining + next_part - 1) / next_part);
            self.partition[cur_idx + 1] = inc_step as u32;
            self.remaining -= inc_step * self.parts[cur_idx + 1];

            self.partition[0] -= inc_step as u32;
            self.remaining += inc_step * self.parts[0];
            if self.remaining >= 0 {
                true
            } else {
                self.search()
            }
        }
    }
}

impl<'a> Iterator for PartitionIterator<'a> {
    type Item = (i32, &'a Vec<u32>);
    fn next(&mut self) -> Option<Self::Item> {
        let found;
        if self.initial {
            if self.remaining < 0 {
                return None;
            }
            self.initial = false;
            found = true;
        } else {
            found = self.search();
        }
        if found {
            Some(unsafe { std::mem::transmute::<_, Self::Item>((self.remaining, &self.partition)) })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trunc_poly_partitions() {
        let p = ValidPrime::new(3);
        let tp = TruncatedPolynomialMonomialBasis::new(p);
        tp.add_gens_and_calculate_parts(1, 2);
        tp.add_gens_and_calculate_parts(2, 1);
        tp.add_gens_and_calculate_parts(3, 0);
        tp.add_gens_and_calculate_parts(4, 0);
        tp.add_gens_and_calculate_parts(5, 0);
        tp.add_gens_and_calculate_parts(6, 0);
        tp.add_gens_and_calculate_parts(7, 0);
        tp.add_gens_and_calculate_parts(8, 0);
        println!("\n\n");
        for d in 0..tp.parts.len() {
            println!("Partitions of {d}");
            for i in &tp.parts[d] {
                println!("      {i}");
            }
        }
    }

    #[test]
    fn test_trunc_poly_partitions2() {
        let p = ValidPrime::new(2);
        let tp = TruncatedPolynomialMonomialBasis::new(p);
        tp.add_gens_and_calculate_parts(1, 0);
        tp.add_gens_and_calculate_parts(2, 0);
        tp.add_gens_and_calculate_parts(3, 1);
        tp.add_gens_and_calculate_parts(4, 1);
        tp.add_gens_and_calculate_parts(5, 1);
        tp.add_gens_and_calculate_parts(6, 2);
        tp.add_gens_and_calculate_parts(7, 1);
        tp.add_gens_and_calculate_parts(8, 1);
        tp.add_gens_and_calculate_parts(9, 1);
        tp.add_gens_and_calculate_parts(10, 2);
        tp.add_gens_and_calculate_parts(11, 1);
        tp.add_gens_and_calculate_parts(12, 2);
        println!("\n\n");
        let expected = vec![
            vec!["[]"], // 0
            vec![],     // 1
            vec![],     // 2
            vec![
                // 3
                "[1]",
            ],
            vec![
                // 4
                "[0, 1]",
            ],
            vec![
                // 5
                "[0, 0, 1]",
            ],
            vec![
                // 6
                "[0, 0, 0, 1, 0]",
                "[0, 0, 0, 0, 1]",
            ],
            vec![
                // 7
                "[1, 1]",
                "[0, 0, 0, 0, 0, 1]",
            ],
            vec![
                // 8
                "[1, 0, 1]",
                "[0, 0, 0, 0, 0, 0, 1]",
            ],
            vec![
                // 9
                "[0, 1, 1]",
                "[1, 0, 0, 1, 0]",
                "[1, 0, 0, 0, 1]",
                "[0, 0, 0, 0, 0, 0, 0, 1]",
            ],
            vec![
                // 10
                "[0, 1, 0, 1, 0]",
                "[0, 1, 0, 0, 1]",
                "[1, 0, 0, 0, 0, 1]",
                "[0, 0, 0, 0, 0, 0, 0, 0, 1, 0]",
                "[0, 0, 0, 0, 0, 0, 0, 0, 0, 1]",
            ],
            vec![
                // 11
                "[0, 0, 1, 1, 0]",
                "[0, 0, 1, 0, 1]",
                "[0, 1, 0, 0, 0, 1]",
                "[1, 0, 0, 0, 0, 0, 1]",
                "[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]",
            ],
            vec![
                // 12
                "[1, 1, 1]",
                "[0, 0, 0, 1, 1]",
                "[0, 0, 1, 0, 0, 1]",
                "[0, 1, 0, 0, 0, 0, 1]",
                "[1, 0, 0, 0, 0, 0, 0, 1]",
                "[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0]",
                "[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]",
            ],
        ];
        for d in 0..tp.parts.len() {
            println!("Partitions of {d}");
            for part in &tp.parts[d] {
                println!("      {part}");
            }
        }
        for (d, parts) in tp.parts.iter().enumerate() {
            for (idx, part) in parts.iter().enumerate() {
                assert!(
                    expected[d][idx] == format!("{part}"),
                    "Discrepancy : degree : {}, idx : {} expected : {}, got : {}",
                    d,
                    idx,
                    expected[d][idx],
                    part
                );
            }
        }
    }

    #[test]
    fn test_partition_iterator() {
        let result = PartitionIterator::new(3, 1, &[0, 1, 2, 3, 4])
            .map(|(x, v)| (x, v.clone()))
            .collect::<Vec<_>>();
        let expected = vec![
            (2, vec![0, 1, 0, 0, 0]),
            (1, vec![0, 0, 1, 0, 0]),
            (0, vec![0, 0, 0, 1, 0]),
            (3, vec![1, 0, 0, 0, 0]),
        ];
        assert_eq!(result, expected);

        let expected = vec![
            (5, vec![0, 1, 0, 0, 0]),
            (4, vec![0, 0, 1, 0, 0]),
            (3, vec![0, 0, 0, 1, 0]),
            (2, vec![0, 0, 0, 0, 1]),
            (6, vec![1, 0, 0, 0, 0]),
        ];
        let result = PartitionIterator::new(6, 1, &[0, 1, 2, 3, 4])
            .map(|(x, v)| (x, v.clone()))
            .collect::<Vec<_>>();
        assert_eq!(result, expected);

        let expected = vec![
            (7, vec![0, 5, 0, 0]),
            (2, vec![0, 4, 1, 0]),
            (0, vec![0, 4, 0, 1]),
            (9, vec![1, 4, 0, 0]),
            (4, vec![1, 3, 1, 0]),
            (2, vec![1, 3, 0, 1]),
            (11, vec![2, 3, 0, 0]),
            (6, vec![2, 2, 1, 0]),
            (4, vec![2, 2, 0, 1]),
            (13, vec![3, 2, 0, 0]),
            (1, vec![2, 1, 2, 0]),
            (8, vec![3, 1, 1, 0]),
            (6, vec![3, 1, 0, 1]),
            (15, vec![4, 1, 0, 0]),
            (3, vec![3, 0, 2, 0]),
            (1, vec![3, 0, 1, 1]),
            (10, vec![4, 0, 1, 0]),
            (8, vec![4, 0, 0, 1]),
            (17, vec![5, 0, 0, 0]),
        ];
        let result = PartitionIterator::new(17, 5, &[0, 2, 7, 9])
            .map(|(x, v)| (x, v.clone()))
            .collect::<Vec<_>>();
        assert_eq!(result, expected);
    }
}
