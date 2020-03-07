use crate::algebra::Algebra;

pub trait Bialgebra : Algebra {
    /// This function decomposes an element of the algebra as a product of elements, each of whose
    /// coproduct is easy to calculate. The product is laid out such that the first element of the
    /// vector is applied to the module element first. This is to be used in conjunction with
    /// `coproduct`.
    ///
    /// This structure is motivated by the fact that in the admissible basis for the Adem algebra,
    /// an element naturally decomposes into a product of Steenrod squares, each of which has an
    /// easy coproduct formula.
    fn decompose (&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize)>;

    /// Expresses Delta(x) as sum_j (A_{ij} (x) B_{ij}). Here x must be one of the elements
    /// returned by `decompose`.
    fn coproduct (&self, op_deg : i32, op_idx : usize) -> Vec<(i32, usize, i32, usize)>;
}
