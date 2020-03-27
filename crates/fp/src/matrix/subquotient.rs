#![allow(unused_imports)]

use super::{Matrix, Subspace};
use crate::vector::{FpVector, FpVectorT};

pub struct Subquotient {
    sub : Subspace,
    quotient : Subspace
}

impl Subquotient {
    /// Given a vector `elt`, project `elt` to the complement and express
    /// as a linear combination of the basis. The result is returned as a list of coefficients.
    /// If elt is nonzero after 
    pub fn reduce(&mut self, elt : &mut FpVector) -> Vec<u32> {
        self.quotient.reduce(elt);
        let mut result = Vec::with_capacity(self.sub.columns());
        for i in 0 .. self.sub.columns() {
            if self.sub.pivots()[i] < 0 {
                continue;
            }
            let c = elt.entry(i);
            result.push(c);
            if c != 0 {
                elt.add(&self.sub[self.sub.pivots()[i] as usize], ((*elt.prime() - 1) * c) % *elt.prime());
            }
        }
        result
    }


    
    /// Given a chain of subspaces `subspace` < `space` < k^`ambient_dimension`, compute the
    /// subquotient `space`/`subspace`. The answer is expressed as a list of basis vectors of
    /// `space` whose image in `space`/`subspace` forms a basis, and a basis vector of `space` is
    /// described by its index in the list of basis vectors of `space` (not the ambient space).
    ///
    /// # Arguments
    ///  * `space` - If this is None, it is the whole space k^`ambient_dimension`
    ///  * `subspace` - If this is None, it is empty
    pub fn subquotient(space : Option<&Subspace>, subspace : Option<&Subspace>, ambient_dimension : usize) -> Vec<usize> {
        match subspace {
            None => {
                if let Some(sp) = space {
                    sp.pivots().iter().filter( |i| **i >= 0).map(|i| *i as usize).collect()
                } else {
                    (0..ambient_dimension).collect()
                }
            },
            Some(subsp) => {
                if let Some(sp) = space {
                    sp.pivots().iter().zip(subsp.pivots().iter())
                      .filter(|(x,y)| {
                          debug_assert!(**x >= 0 || **y < 0);
                          **x >= 0 && **y < 0
                        }).map(|(x,_)| *x as usize).collect()
                } else {
                    (0..ambient_dimension).filter( |i| subsp.pivots()[*i] < 0).collect()
                }
            }
        }
    }
}

