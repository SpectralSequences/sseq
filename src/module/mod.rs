use std::sync::Arc;
use std::error::Error;
use enum_dispatch::enum_dispatch;

use super::fp_vector::{FpVector, FpVectorT};
use super::algebra::{Algebra, AlgebraAny};

mod finite_dimensional_module;
mod finitely_presented_module;
mod truncated_module;
mod quotient_module;
mod free_module;
mod hom_module;
mod tensor_module;
mod sum_module;

pub mod homomorphism;

pub use finite_dimensional_module::FiniteDimensionalModule as FDModule;
pub use hom_module::HomModule;
pub use finitely_presented_module::FinitelyPresentedModule as FPModule;
pub use truncated_module::TruncatedModule;
pub use quotient_module::QuotientModule;
pub use free_module::FreeModule;
pub use free_module::FreeModuleTableEntry;
pub use tensor_module::TensorModule;
pub use sum_module::SumModule;

use bivec::BiVec;

pub trait BoundedModule : Module {
    /// `max_degree` is the a degree such that if t > `max_degree`, then `self.dimension(t) = 0`.
    fn max_degree(&self) -> i32;

    fn total_dimension(&self) -> usize {
        let mut sum = 0;
        for i in 0 ..= self.max_degree() {
            sum += self.dimension(i);
        }
        sum
    }

    fn to_fd_module(&self) -> FDModule {
        let min_degree = self.min_degree();
        let max_degree = self.max_degree();
        self.compute_basis(max_degree);

        let mut graded_dimension = BiVec::with_capacity(min_degree, max_degree + 1);
        for i in min_degree ..= max_degree {
            graded_dimension.push(self.dimension(i));
        }
        let mut result = FDModule::new(self.algebra(), self.name().to_string(), graded_dimension);

        let algebra = self.algebra();
        for input_degree in min_degree ..= max_degree {
            for output_degree in (input_degree + 1) ..= max_degree {
                let output_dimension = result.dimension(output_degree);
                if output_dimension == 0 {
                    continue;
                }
                let op_degree = output_degree - input_degree;

                for input_idx in 0 .. result.dimension(input_degree){
                    for op_idx in 0 .. algebra.dimension(op_degree, -1) {
                        let output_vec : &mut FpVector = result.action_mut(op_degree, op_idx, input_degree, input_idx);
                        self.act_on_basis(output_vec, 1, op_degree, op_idx, input_degree, input_idx);
                    }
                }
            }
        }
        result
    }
}

#[enum_dispatch(FiniteModule)]
pub trait Module : Send + Sync + 'static {
    fn algebra(&self) -> Arc<AlgebraAny>;
    fn name(&self) -> &str;
    fn min_degree(&self) -> i32;
    fn compute_basis(&self, _degree : i32) {}
    fn dimension(&self, degree : i32) -> usize;
    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize);

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String;
    // Whether this is the unit module.
    fn is_unit(&self) -> bool { false }

    fn prime(&self) -> u32 {
        self.algebra().prime()
    }

    /// Whether act_on_basis_borrow is available.
    fn borrow_output(&self) -> bool { false }

    /// Returns a borrow of the value of the corresponding action on the basis element. This
    /// FpVector must be "pure", i.e. it is not sliced and the limbs are zero in indices greater
    /// than the dimension of the vector.
    fn act_on_basis_borrow(&self, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize) -> &FpVector { unimplemented!() }

    fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector){
        assert!(input.dimension() == self.dimension(input_degree));
        let p = self.algebra().prime();
        for (i, v) in input.iter().enumerate() {
            if v == 0 {
                continue;
            }
            self.act_on_basis(result, (coeff * v) % p, op_degree, op_index, input_degree, i);
        }
    }

    fn act_by_element(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op : &FpVector, input_degree : i32, input : &FpVector){
        assert_eq!(input.dimension(), self.dimension(input_degree));
        let p = self.algebra().prime();
        for (i, v) in op.iter().enumerate() {
            if v == 0 {
                continue;
            }
            self.act(result, (coeff * v) % p, op_degree, i, input_degree, input);
        }
    }

    fn generator_list_string(&self, degree : i32) -> String {
        let mut result = String::from("[");
        result += &(0..self.dimension(degree))
            .map(|idx| self.basis_element_to_string(degree, idx))
            .collect::<Vec<String>>()
            .join(", ");
        result += "]";
        result
    }

    fn element_to_string(&self, degree : i32, element : &FpVector) -> String {
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
        return result;
    } 
}

#[enum_dispatch]
#[derive(PartialEq, Eq)]
pub enum FiniteModule {
    FDModule,
    FPModule
}

impl FiniteModule {
    pub fn from_json(algebra : Arc<AlgebraAny>, json : &mut serde_json::Value) -> Result<Self, Box<dyn Error>> {
        let module_type = &json["type"].as_str().unwrap();
        match module_type {
            &"finite dimensional module" => Ok(FiniteModule::from(FDModule::from_json(algebra, json))),
            &"finitely presented module" => Ok(FiniteModule::from(FPModule::from_json(algebra, json))),
            _ => Err(Box::new(UnknownModuleTypeError { module_type : module_type.to_string() }))
        }
    }

    pub fn as_fp_module(self) -> Option<FPModule> {
        match self {
            FiniteModule::FDModule(_) => None,
            FiniteModule::FPModule(m) => Some(m)
        }
    }

    pub fn as_fd_module(self) -> Option<FDModule> {
        match self {
            FiniteModule::FDModule(m) => Some(m),
            FiniteModule::FPModule(_) => None
        }
    }
}

#[derive(Debug)]
pub struct UnknownModuleTypeError {
    pub module_type : String
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
    pub relation : String,
    pub value : String
}

impl std::fmt::Display for ModuleFailedRelationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Relation failed:\n    {}  !=  0\nInstead it is equal to {}\n", &self.relation, &self.value)
    }
}

impl Error for ModuleFailedRelationError {
    fn description(&self) -> &str {
        "Module failed a relation"
    }
}

pub trait ZeroModule : Module {
    fn zero_module(algebra : Arc<AlgebraAny>, min_degree : i32) -> Self;
}

impl ZeroModule for FiniteModule {
    fn zero_module(algebra : Arc<AlgebraAny>, min_degree : i32) -> Self {
        FiniteModule::FDModule(FDModule::zero_module(algebra, min_degree))
    }
}
