use super::{Matrix, Subspace};
use crate::vector::{FpVector, FpVectorT};

struct Subquotient {
    sub : Subspace,
    quotient : Option<Subspace>, 
}

impl Subquotient {
    /// Given a vector `elt`, project `elt` to the complement and express
    /// as a linear combination of the basis. The result is returned as a list of coefficients.
    /// If elt is nonzero after 
    pub fn reduce(&mut self, elt : &mut FpVector) -> Vec<u32> {
        if let Some(z) = &self.quotient {
            z.reduce(elt);
        }
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
}

