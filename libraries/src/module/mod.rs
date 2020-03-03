use std::error::Error;
use std::sync::Arc;

use crate::algebra::{Algebra, SteenrodAlgebra};
use fp::prime::ValidPrime;
use fp::vector::{FpVector, FpVectorT};

mod finite_dimensional_module;
mod finitely_presented_module;
mod free_module;
mod hom_module;
mod quotient_module;
mod rpn;
mod sum_module;
mod tensor_module;
mod truncated_module;

pub mod homomorphism;

pub use finite_dimensional_module::FiniteDimensionalModule as FDModule;
pub use finitely_presented_module::FinitelyPresentedModule as FPModule;
pub use free_module::FreeModule;
pub use free_module::FreeModuleTableEntry;
pub use hom_module::HomModule;
pub use quotient_module::QuotientModule;
pub use rpn::RealProjectiveSpace;
pub use sum_module::SumModule;
pub use tensor_module::TensorModule;
pub use truncated_module::TruncatedModule;

use bivec::BiVec;

pub trait BoundedModule: Module {
    /// `max_degree` is the a degree such that if t > `max_degree`, then `self.dimension(t) = 0`.
    fn max_degree(&self) -> i32;

    fn total_dimension(&self) -> usize {
        let mut sum = 0;
        for i in 0..=self.max_degree() {
            sum += self.dimension(i);
        }
        sum
    }

    fn to_fd_module(&self) -> FDModule<Self::Algebra> {
        let min_degree = self.min_degree();
        let max_degree = self.max_degree();
        self.compute_basis(max_degree);

        let mut graded_dimension = BiVec::with_capacity(min_degree, max_degree + 1);
        for t in min_degree..=max_degree {
            graded_dimension.push(self.dimension(t));
        }
        let mut result = FDModule::new(self.algebra(), self.name().to_string(), graded_dimension);
        for t in min_degree..=max_degree {
            for idx in 0..result.dimension(t) {
                result.set_basis_element_name(t, idx, self.basis_element_to_string(t, idx));
            }
        }

        let algebra = self.algebra();
        for input_degree in min_degree..=max_degree {
            for output_degree in (input_degree + 1)..=max_degree {
                let output_dimension = result.dimension(output_degree);
                if output_dimension == 0 {
                    continue;
                }
                let op_degree = output_degree - input_degree;

                for input_idx in 0..result.dimension(input_degree) {
                    for op_idx in 0..algebra.dimension(op_degree, -1) {
                        let output_vec: &mut FpVector =
                            result.action_mut(op_degree, op_idx, input_degree, input_idx);
                        self.act_on_basis(
                            output_vec,
                            1,
                            op_degree,
                            op_idx,
                            input_degree,
                            input_idx,
                        );
                    }
                }
            }
        }
        result
    }
}

pub trait Module: Send + Sync + 'static {
    type Algebra: Algebra;

    fn algebra(&self) -> Arc<Self::Algebra>;
    fn name(&self) -> &str;
    fn min_degree(&self) -> i32;
    fn compute_basis(&self, _degree: i32) {}
    fn dimension(&self, degree: i32) -> usize;
    fn act_on_basis(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    );

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String;
    // Whether this is the unit module.
    fn is_unit(&self) -> bool {
        false
    }

    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    /// Whether act_on_basis_borrow is available.
    fn borrow_output(&self) -> bool {
        false
    }

    /// Returns a borrow of the value of the corresponding action on the basis element. This
    /// FpVector must be "pure", i.e. it is not sliced and the limbs are zero in indices greater
    /// than the dimension of the vector.
    fn act_on_basis_borrow(
        &self,
        _op_degree: i32,
        _op_index: usize,
        _mod_degree: i32,
        _mod_index: usize,
    ) -> &FpVector {
        unimplemented!()
    }

    fn act(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: &FpVector,
    ) {
        assert!(input.dimension() == self.dimension(input_degree));
        let p = self.prime();
        for (i, v) in input.iter().enumerate() {
            if v == 0 {
                continue;
            }
            self.act_on_basis(
                result,
                (coeff * v) % *p,
                op_degree,
                op_index,
                input_degree,
                i,
            );
        }
    }

    fn act_by_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op: &FpVector,
        input_degree: i32,
        input: &FpVector,
    ) {
        assert_eq!(input.dimension(), self.dimension(input_degree));
        let p = self.prime();
        for (i, v) in op.iter().enumerate() {
            if v == 0 {
                continue;
            }
            self.act(result, (coeff * v) % *p, op_degree, i, input_degree, input);
        }
    }

    fn generator_list_string(&self, degree: i32) -> String {
        let mut result = String::from("[");
        result += &(0..self.dimension(degree))
            .map(|idx| self.basis_element_to_string(degree, idx))
            .collect::<Vec<String>>()
            .join(", ");
        result += "]";
        result
    }

    fn element_to_string(&self, degree: i32, element: &FpVector) -> String {
        let mut result = String::new();
        let mut zero = true;
        for (idx, value) in element.iter().enumerate() {
            if value == 0 {
                continue;
            }
            zero = false;
            if value != 1 {
                result.push_str(&format!("{} * ", value));
            }
            let b = self.basis_element_to_string(degree, idx);
            result.push_str(&format!("{} + ", b));
        }
        if zero {
            result.push_str("0");
        } else {
            // Remove trailing " + "
            result.pop();
            result.pop();
            result.pop();
        }
        result
    }
}

impl<A: Algebra> Module for Box<dyn Module<Algebra = A>> {
    type Algebra = A;

    fn algebra(&self) -> Arc<Self::Algebra> {
        (&**self).algebra()
    }

    fn name(&self) -> &str {
        (&**self).name()
    }

    fn min_degree(&self) -> i32 {
        (&**self).min_degree()
    }

    fn compute_basis(&self, degree: i32) {
        (&**self).compute_basis(degree);
    }
    fn dimension(&self, degree: i32) -> usize {
        (&**self).dimension(degree)
    }

    fn act_on_basis(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        (&**self).act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index);
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        (&**self).basis_element_to_string(degree, idx)
    }

    // Whether this is the unit module.
    fn is_unit(&self) -> bool {
        (&**self).is_unit()
    }

    fn prime(&self) -> ValidPrime {
        (&**self).prime()
    }

    /// Whether act_on_basis_borrow is available.
    fn borrow_output(&self) -> bool {
        (&**self).borrow_output()
    }

    /// Returns a borrow of the value of the corresponding action on the basis element. This
    /// FpVector must be "pure", i.e. it is not sliced and the limbs are zero in indices greater
    /// than the dimension of the vector.
    fn act_on_basis_borrow(
        &self,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) -> &FpVector {
        (&**self).act_on_basis_borrow(op_degree, op_index, mod_degree, mod_index)
    }

    fn act(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: &FpVector,
    ) {
        (&**self).act(result, coeff, op_degree, op_index, input_degree, input);
    }

    fn act_by_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op: &FpVector,
        input_degree: i32,
        input: &FpVector,
    ) {
        (&**self).act_by_element(result, coeff, op_degree, op, input_degree, input);
    }

    fn generator_list_string(&self, degree: i32) -> String {
        (&**self).generator_list_string(degree)
    }

    fn element_to_string(&self, degree: i32, element: &FpVector) -> String {
        (&**self).element_to_string(degree, element)
    }
}

// Poor man's trait alias
pub trait SteenrodModule: Module<Algebra = SteenrodAlgebra> {}
impl<M: Module<Algebra = SteenrodAlgebra>> SteenrodModule for M {}

#[derive(PartialEq, Eq)]
pub enum FiniteModule {
    FDModule(FDModule<SteenrodAlgebra>),
    FPModule(FPModule<SteenrodAlgebra>),
    RealProjectiveSpace(RealProjectiveSpace),
}

impl Module for FiniteModule {
    type Algebra = SteenrodAlgebra;

    fn algebra(&self) -> Arc<Self::Algebra> {
        match self {
            FiniteModule::FDModule(m) => m.algebra(),
            FiniteModule::FPModule(m) => m.algebra(),
            FiniteModule::RealProjectiveSpace(m) => m.algebra(),
        }
    }

    fn name(&self) -> &str {
        match self {
            FiniteModule::FDModule(m) => m.name(),
            FiniteModule::FPModule(m) => m.name(),
            FiniteModule::RealProjectiveSpace(m) => m.name(),
        }
    }

    fn min_degree(&self) -> i32 {
        match self {
            FiniteModule::FDModule(m) => m.min_degree(),
            FiniteModule::FPModule(m) => m.min_degree(),
            FiniteModule::RealProjectiveSpace(m) => m.min_degree(),
        }
    }
    fn compute_basis(&self, degree: i32) {
        match self {
            FiniteModule::FDModule(m) => m.compute_basis(degree),
            FiniteModule::FPModule(m) => m.compute_basis(degree),
            FiniteModule::RealProjectiveSpace(m) => m.compute_basis(degree),
        }
    }

    fn dimension(&self, degree: i32) -> usize {
        match self {
            FiniteModule::FDModule(m) => m.dimension(degree),
            FiniteModule::FPModule(m) => m.dimension(degree),
            FiniteModule::RealProjectiveSpace(m) => m.dimension(degree),
        }
    }

    fn act_on_basis(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) {
        match self {
            FiniteModule::FDModule(m) => {
                m.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index)
            }
            FiniteModule::FPModule(m) => {
                m.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index)
            }
            FiniteModule::RealProjectiveSpace(m) => {
                m.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index)
            }
        }
    }

    // Dispatch these as well so that we don't have to match on the type every loop.
    // Experimentally, not doing so causes a significant performance on some runs (while having no
    // impact on the others)
    fn act(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op_index: usize,
        input_degree: i32,
        input: &FpVector,
    ) {
        match self {
            FiniteModule::FDModule(m) => {
                m.act(result, coeff, op_degree, op_index, input_degree, input)
            }
            FiniteModule::FPModule(m) => {
                m.act(result, coeff, op_degree, op_index, input_degree, input)
            }
            FiniteModule::RealProjectiveSpace(m) => {
                m.act(result, coeff, op_degree, op_index, input_degree, input)
            }
        }
    }

    fn act_by_element(
        &self,
        result: &mut FpVector,
        coeff: u32,
        op_degree: i32,
        op: &FpVector,
        input_degree: i32,
        input: &FpVector,
    ) {
        match self {
            FiniteModule::FDModule(m) => {
                m.act_by_element(result, coeff, op_degree, op, input_degree, input)
            }
            FiniteModule::FPModule(m) => {
                m.act_by_element(result, coeff, op_degree, op, input_degree, input)
            }
            FiniteModule::RealProjectiveSpace(m) => {
                m.act_by_element(result, coeff, op_degree, op, input_degree, input)
            }
        }
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        match self {
            FiniteModule::FDModule(m) => m.basis_element_to_string(degree, idx),
            FiniteModule::FPModule(m) => m.basis_element_to_string(degree, idx),
            FiniteModule::RealProjectiveSpace(m) => m.basis_element_to_string(degree, idx),
        }
    }

    fn is_unit(&self) -> bool {
        match self {
            FiniteModule::FDModule(m) => m.is_unit(),
            _ => false,
        }
    }

    fn prime(&self) -> ValidPrime {
        self.algebra().prime()
    }

    /// Whether act_on_basis_borrow is available.
    fn borrow_output(&self) -> bool {
        match self {
            FiniteModule::FDModule(_) => true,
            _ => false,
        }
    }

    fn act_on_basis_borrow(
        &self,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) -> &FpVector {
        match self {
            FiniteModule::FDModule(m) => {
                m.act_on_basis_borrow(op_degree, op_index, mod_degree, mod_index)
            }
            _ => unimplemented!(),
        }
    }
}

impl From<FPModule<SteenrodAlgebra>> for FiniteModule {
    fn from(m: FPModule<SteenrodAlgebra>) -> Self {
        Self::FPModule(m)
    }
}
impl From<FDModule<SteenrodAlgebra>> for FiniteModule {
    fn from(m: FDModule<SteenrodAlgebra>) -> Self {
        Self::FDModule(m)
    }
}
impl From<RealProjectiveSpace> for FiniteModule {
    fn from(m: RealProjectiveSpace) -> Self {
        Self::RealProjectiveSpace(m)
    }
}

impl FiniteModule {
    pub fn from_json(
        algebra: Arc<SteenrodAlgebra>,
        json: &mut serde_json::Value,
    ) -> Result<Self, Box<dyn Error>> {
        let module_type = &json["type"].as_str().unwrap();
        match *module_type {
            "real projective space" => Ok(FiniteModule::from(RealProjectiveSpace::from_json(
                algebra, json,
            )?)),
            "finite dimensional module" => {
                Ok(FiniteModule::from(FDModule::from_json(algebra, json)))
            }
            "finitely presented module" => {
                Ok(FiniteModule::from(FPModule::from_json(algebra, json)))
            }
            _ => Err(Box::new(UnknownModuleTypeError {
                module_type: (*module_type).to_string(),
            })),
        }
    }

    pub fn into_real_projective_space(self) -> Option<RealProjectiveSpace> {
        match self {
            FiniteModule::RealProjectiveSpace(m) => Some(m),
            _ => None,
        }
    }

    pub fn into_fp_module(self) -> Option<FPModule<SteenrodAlgebra>> {
        match self {
            FiniteModule::FPModule(m) => Some(m),
            _ => None,
        }
    }

    pub fn into_fd_module(self) -> Option<FDModule<SteenrodAlgebra>> {
        match self {
            FiniteModule::FDModule(m) => Some(m),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct UnknownModuleTypeError {
    pub module_type: String,
}

impl std::fmt::Display for UnknownModuleTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unknown module type: {}", &self.module_type)
    }
}

impl Error for UnknownModuleTypeError {
    fn description(&self) -> &str {
        "Unknown module type"
    }
}

#[derive(Debug)]
pub struct ModuleFailedRelationError {
    pub relation: String,
    pub value: String,
}

impl std::fmt::Display for ModuleFailedRelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Relation failed:\n    {}  !=  0\nInstead it is equal to {}\n",
            &self.relation, &self.value
        )
    }
}

impl Error for ModuleFailedRelationError {
    fn description(&self) -> &str {
        "Module failed a relation"
    }
}

pub trait ZeroModule: Module {
    fn zero_module(algebra: Arc<Self::Algebra>, min_degree: i32) -> Self;
}

impl ZeroModule for FiniteModule {
    fn zero_module(algebra: Arc<SteenrodAlgebra>, min_degree: i32) -> Self {
        FiniteModule::FDModule(FDModule::zero_module(algebra, min_degree))
    }
}
