use super::{Matrix, Subspace};

// struct SubquotientBasis {
//     basis : Matrix,
//     // quotient : Option<&Subspace>, 
// }

// /// Given a vector `elt`, a subspace `zeros` of the total space (with a specified choice of
// /// complement) and a basis `basis` of a subspace of the complement, project `elt` to the complement and express
// /// as a linear combination of the basis. This assumes the projection of `elt` is indeed in the
// /// span of `basis`. The result is returned as a list of coefficients.
// ///
// /// If `zeros` is none, then the initial projection is not performed.
// pub fn express_basis(mut elt : &mut FpVector, zeros : Option<&Subspace>, basis : &(Vec<isize>, Vec<FpVector>)) -> Vec<u32>{
//     if let Some(z) = zeros {
//         z.reduce(&mut elt);
//     }
//     let mut result = Vec::with_capacity(basis.0.len());
//     for i in 0 .. basis.0.len() {
//         if basis.0[i] < 0 {
//             continue;
//         }
//         let c = elt.entry(i);
//         result.push(c);
//         if c != 0 {
//             elt.add(&basis.1[basis.0[i] as usize], ((*elt.prime() - 1) * c) % *elt.prime());
//         }
//     }
// //    assert!(elt.is_zero());
//     result
// }
