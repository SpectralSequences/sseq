use crate::algebra::Algebra;

/// An [`Algebra`] equipped with a coproduct operation that makes it into a
/// bialgebra.
pub trait Bialgebra: Algebra {
    /// Computes a coproduct $\Delta(x)$, expressed as
    ///
    /// $$ Delta(x)_i = \sum_j A_{ij} \otimes B_{ij}. $$
    ///
    /// The return value is a list of these pairs of basis elements.
    ///
    /// `x` must have been returned by [`Bialgebra::decompose()`].
    fn coproduct(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize, i32, usize)>;

    /// Decomposes an element of the algebra into a product of elements, each of
    /// which we can compute a coproduct on efficiently.
    ///
    /// The product is laid out such that the first element of the vector is
    /// applied to a module element first when acting on it.
    ///
    /// This function is to be used with [`Bialgebra::coproduct()`].
    ///
    /// This API is motivated by the fact that, in the admissible basis for the Adem algebra,
    /// an element naturally decomposes into a product of Steenrod squares, each of which has an
    /// easy coproduct formula.
    fn decompose(&self, op_deg: i32, op_idx: usize) -> Vec<(i32, usize)>;
}
