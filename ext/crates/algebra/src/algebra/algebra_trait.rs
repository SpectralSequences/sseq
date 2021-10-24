use fp::prime::ValidPrime;
use fp::vector::{Slice, SliceMut};

use std::marker::PhantomData;
use std::fmt;
use std::cmp;
use std::any;

/// A basis element in an [`Algebra`] $A$.
pub struct BasisElem<A> {
    degree: i32,
    index: usize,
    _alg: PhantomData<*const A>,
}

impl<A> BasisElem<A> {
    /// Creates a new basis element for $A$ with the given data.
    pub fn new(degree: i32, index: usize) -> Self {
        Self {degree, index, _alg: PhantomData}
    }

    /// Returns the degree of $A$ this is a basis element of.
    pub fn degree(&self) -> i32 {
        self.degree
    }

    /// Returns the index of this basis element within the ordered basis of
    /// $A_i$.
    pub fn index(&self) -> usize {
        self.index
    }
}

// Manual impls, since we want to avoid a bound involving `A`...
impl<A> Clone for BasisElem<A> {
    fn clone(&self) -> Self {
        Self::new(self.degree, self.index)
    }
}

impl<A> Copy for BasisElem<A> {}

impl<A> PartialEq for BasisElem<A> {
    fn eq(&self, that: &Self) -> bool {
        cmp::Ord::cmp(self, that).is_eq()
    }
}

impl<A> Eq for BasisElem<A> {}

impl<A> PartialOrd for BasisElem<A> {
    fn partial_cmp(&self, that: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(that))
    }
}

impl<A> Ord for BasisElem<A> {
    fn cmp(&self, that: &Self) -> cmp::Ordering {
        (self.degree, self.index).cmp(&(that.degree, that.index))
    }
}

impl<A> fmt::Debug for BasisElem<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BasisElem<{}>({}, {})", any::type_name::<A>(), self.degree, self.index)
    }
}

/// A general element of fixed degree in an [`Algebra`] $A$.
/// 
/// The coefficients of the element are given by the `fp` vector type `V`, where
/// the `i`th element of the vector is the coefficient for the `i`th basis
/// element.
pub struct VectorElem<A, V> {
    degree: i32,
    coeffs: V,
    _alg: PhantomData<*const A>,
}

impl<A, V> VectorElem<A, V> {
    /// Creates a new basis element for `A` with the given data.
    pub fn new(degree: i32, coeffs: V) -> Self {
        Self {degree, coeffs, _alg: PhantomData}
    }

    /// Returns the degree of $A$ this element sit in.
    pub fn degree(&self) -> i32 {
        self.degree
    }

    /// Returns the coefficients that give this element in terms of the
    /// distinguished basis of $A_i$.
    pub fn coeffs(&self) -> &V {
        &self.coeffs
    }

    /// Returns the mutable coefficients that give this element in terms of the
    /// distinguished basis of $A_i$.
    pub fn coeffs_mut(&mut self) -> &mut V {
        &mut self.coeffs
    }
}

// Manual impls, since we want to avoid a bound involving `A`...
impl<A, V: Clone> Clone for VectorElem<A, V> {
    fn clone(&self) -> Self {
        Self::new(self.degree, self.coeffs.clone())
    }
}

impl<A, V: Copy> Copy for VectorElem<A, V> {}

impl<A, V: PartialEq<W>, W> PartialEq<VectorElem<A, W>> for VectorElem<A, V> {
    fn eq(&self, that: &VectorElem<A, W>) -> bool {
        self.degree == that.degree && self.coeffs == that.coeffs
    }
}

impl<A, V: Eq> Eq for VectorElem<A, V> {}

impl<A, V: PartialOrd> cmp::PartialOrd for VectorElem<A, V> {
    fn partial_cmp(&self, that: &Self) -> Option<cmp::Ordering> {
        (self.degree, &self.coeffs).partial_cmp(&(that.degree, &that.coeffs))
    }
}

impl<A, V: Ord> Ord for VectorElem<A, V> {
    fn cmp(&self, that: &Self) -> cmp::Ordering {
        (self.degree, &self.coeffs).cmp(&(that.degree, &that.coeffs))
    }
}

impl<A, V: fmt::Debug> fmt::Debug for VectorElem<A, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VectorElem<{}>({}, {:?})", any::type_name::<A>(), self.degree, self.coeffs)
    }
}

/// A general element of fixed degree in an [`Algebra`] $A$.
/// 
/// This enum combines `BasisElem` and `VectorElem` into one.
pub enum Elem<A, V> {
    Basis(BasisElem<A>),
    Vector(VectorElem<A, V>),
}

// Manual impls, since we want to avoid a bound involving `A`...
impl<A, V: Clone> Clone for Elem<A, V> {
    fn clone(&self) -> Self {
        match self {
            Self::Basis(b) => Self::Basis(*b),
            Self::Vector(v) => Self::Vector(v.clone()),
        }
    }
}

impl<A, V: Copy> Copy for Elem<A, V> {}

impl<A, V: PartialEq<W>, W> PartialEq<Elem<A, W>> for Elem<A, V> {
    fn eq(&self, that: &Elem<A, W>) -> bool {
        match (self, that) {
            (Self::Basis(b1), Elem::Basis(b2)) => b1 == b2,
            (Self::Vector(v1), Elem::Vector(v2)) => v1 == v2,
            _ => false,
        }
    }
}

impl<A, V: Eq> Eq for Elem<A, V> {}

impl<A, V: fmt::Debug> fmt::Debug for Elem<A, V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Basis(b) => b.fmt(f),
            Self::Vector(v) => v.fmt(f),
        }
    }
}

impl<A, V> From<BasisElem<A>> for Elem<A, V> {
    fn from(b: BasisElem<A>) -> Self {
        Self::Basis(b)
    }
}

impl<A, V> From<VectorElem<A, V>> for Elem<A, V> {
    fn from(v: VectorElem<A, V>) -> Self {
        Self::Vector(v)
    }
}

/// A graded algebra over F_p, finite dimensional in each degree, equipped with a choice of ordered
/// basis in each dimension. Basis elements of the algebra are referred to by their degree and
/// index, and general elements are referred to by the degree and an `FpVector` listing the
/// coefficients of the element in terms of the basis.
///
/// Since the graded algebra is often infinite dimensional, we cannot construct a complete
/// description of the algebra. Instead, we use the function `compute_basis(degree)`. When called,
/// the algebra should compute relevant data to be able to perform calculations up to degree
/// `degree`. It is the responsibility of users to ensure `compute_degree(degree)` is called before
/// calling other functions with the `degree` parameter.
///
/// The algebra should also come with a specified choice of algebra generators, which are
/// necessarily basis elements. It gives us a simpler way of describing finite modules by only
/// specifying the action of the generators.
pub trait Algebra: std::fmt::Display + Send + Sync {
    /// Returns the prime the algebra is over.
    fn prime(&self) -> ValidPrime;

    /// Computes the list of basis elements up to and including degree `degree`. This should include any
    /// other preparation needed to evaluate all the other functions that involve a degree
    /// parameter. One should be able to call compute_basis multiple times, and there should be
    /// little overhead when calling `compute_basis(degree)` multiple times with the same `degree`.
    fn compute_basis(&self, degree: i32);

    /// Gets the dimension of the algebra in degree `degree`.
    fn dimension(&self, degree: i32, excess: i32) -> usize;

    /// Computes the product `r * s` of the two basis elements, and *adds* the result to `result`.
    ///
    /// result is not required to be aligned.
    fn multiply_basis_elements(
        &self,
        result: SliceMut,
        coeff: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s_idx: usize,
        excess: i32,
    );

    /// result and s are not required to be aligned.
    fn multiply_basis_element_by_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        r_degree: i32,
        r_idx: usize,
        s_degree: i32,
        s: Slice,
        excess: i32,
    ) {
        let p = self.prime();
        for (i, v) in s.iter_nonzero() {
            self.multiply_basis_elements(
                result.copy(),
                (coeff * v) % *p,
                r_degree,
                r_idx,
                s_degree,
                i,
                excess,
            );
        }
    }

    /// result and r are not required to be aligned.
    fn multiply_element_by_basis_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        r_degree: i32,
        r: Slice,
        s_degree: i32,
        s_idx: usize,
        excess: i32,
    ) {
        let p = self.prime();
        for (i, v) in r.iter_nonzero() {
            self.multiply_basis_elements(
                result.copy(),
                (coeff * v) % *p,
                r_degree,
                i,
                s_degree,
                s_idx,
                excess,
            );
        }
    }

    /// result, r and s are not required to be aligned.
    fn multiply_element_by_element(
        &self,
        mut result: SliceMut,
        coeff: u32,
        r_degree: i32,
        r: Slice,
        s_degree: i32,
        s: Slice,
        excess: i32,
    ) {
        let p = self.prime();
        for (i, v) in s.iter_nonzero() {
            self.multiply_element_by_basis_element(
                result.copy(),
                (coeff * v) % *p,
                r_degree,
                r,
                s_degree,
                i,
                excess,
            );
        }
    }

    /// A filtration one element in Ext(k, k) is the same as an indecomposable element of the
    /// algebra.  This function returns a default list of such elements in the format `(name,
    /// degree, index)` for whom we want to compute products with in the resolutions.
    fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)> {
        Vec::new()
    }

    /// Converts a basis element into a string for display.
    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String;

    /// Converts an element into a string for display.
    fn element_to_string(&self, degree: i32, element: Slice) -> String {
        let mut result = String::new();
        let mut zero = true;
        for (idx, value) in element.iter_nonzero() {
            zero = false;
            if value != 1 {
                result.push_str(&format!("{} * ", value));
            }
            let b = self.basis_element_to_string(degree, idx);
            result.push_str(&format!("{} + ", b));
        }
        if zero {
            result.push('0');
        } else {
            // Remove trailing " + "
            result.pop();
            result.pop();
            result.pop();
        }
        result
    }
}

#[cfg(feature = "json")]
pub trait JsonAlgebra: Algebra {
    fn prefix(&self) -> &str;

    /// Converts a JSON object into a basis element. The way basis elements are represented by JSON
    /// objects is to be specified by the algebra itself, and will be used by module
    /// specifications.
    fn json_to_basis(&self, json: &serde_json::Value) -> anyhow::Result<(i32, usize)>;

    fn json_from_basis(&self, degree: i32, idx: usize) -> serde_json::Value;
}

/// An algebra with a specified list of generators and generating relations. This data can be used
/// to specify modules by specifying the actions of the generators.
pub trait GeneratedAlgebra: Algebra {
    /// Given a degree `degree`, the function returns a list of algebra generators in that degree.
    /// This return value is the list of indices of the basis elements that are generators. The
    /// list need not be in any particular order.
    ///
    /// This method need not be fast, because they will only be performed when constructing the module,
    /// and will often only involve low dimensional elements.
    fn generators(&self, degree: i32) -> Vec<usize>;

    /// This returns the name of a generator. Note that the index is the index of the generator
    /// in the list of all basis elements. It is undefined behaviour to call this function with a
    /// (degree, index) pair that is not a generator.
    ///
    /// The default implementation calls `self.basis_element_to_string`, but occassionally the
    /// generators might have alternative, more concise names that are preferred.
    ///
    /// This function MUST be inverse to `string_to_generator`.
    fn generator_to_string(&self, degree: i32, idx: usize) -> String {
        self.basis_element_to_string(degree, idx)
    }

    /// This parses a string and returns the generator described by the string. The signature of
    /// this function is the same `nom` combinators.
    ///
    /// This function MUST be inverse to `string_to_generator` (and not `basis_element_to_string`).
    fn string_to_generator<'a, 'b>(&'a self, input: &'b str)
        -> nom::IResult<&'b str, (i32, usize)>;

    /// Given a non-generator basis element of the algebra, decompose it in terms of algebra
    /// generators. Recall each basis element is given by a pair $(d, i))$, where $d$ is the degree of
    /// the generator, and $i$ is the index of the basis element. Given a basis element $A$, the
    /// function returns a list of triples $(c_i, A_i, B_i)$ where each $A_i$ and $B_i$ are basis
    /// elements of strictly smaller degree than the original, and
    /// $$ A = \sum_i c_i A_i B_i.$$
    /// This allows us to recursively compute the action of the algebra.
    ///
    /// This method need not be fast, because they will only be performed when constructing the module,
    /// and will often only involve low dimensional elements.
    fn decompose_basis_element(
        &self,
        degree: i32,
        idx: usize,
    ) -> Vec<(u32, (i32, usize), (i32, usize))>;

    /// Get any relations that the algebra wants checked to ensure the consistency of module.
    fn generating_relations(&self, degree: i32) -> Vec<Vec<(u32, (i32, usize), (i32, usize))>>;
}

#[macro_export]
macro_rules! dispatch_algebra {
    ($struct:ty, $dispatch_macro: ident) => {
        impl Algebra for $struct {
            $dispatch_macro! {
                fn prime(&self) -> ValidPrime;
                fn compute_basis(&self, degree: i32);
                fn dimension(&self, degree: i32, excess: i32) -> usize;
                fn multiply_basis_elements(
                    &self,
                    result: SliceMut,
                    coeff: u32,
                    r_degree: i32,
                    r_idx: usize,
                    s_degree: i32,
                    s_idx: usize,
                    excess: i32,
                );

                fn multiply_basis_element_by_element(
                    &self,
                    result: SliceMut,
                    coeff: u32,
                    r_degree: i32,
                    r_idx: usize,
                    s_degree: i32,
                    s: Slice,
                    excess: i32,
                );

                fn multiply_element_by_basis_element(
                    &self,
                    result: SliceMut,
                    coeff: u32,
                    r_degree: i32,
                    r: Slice,
                    s_degree: i32,
                    s_idx: usize,
                    excess: i32,
                );

                fn multiply_element_by_element(
                    &self,
                    result: SliceMut,
                    coeff: u32,
                    r_degree: i32,
                    r: Slice,
                    s_degree: i32,
                    s: Slice,
                    excess: i32,
                );

                fn default_filtration_one_products(&self) -> Vec<(String, i32, usize)>;

                fn basis_element_to_string(&self, degree: i32, idx: usize) -> String;

                fn element_to_string(&self, degree: i32, element: Slice) -> String;
            }
        }
    };
}
