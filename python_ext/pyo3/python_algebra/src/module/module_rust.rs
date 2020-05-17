use std::sync::Arc;

use fp::prime::ValidPrime;
use fp::vector::FpVector;
use algebra::module::{
    Module as ModuleT,
    FDModule as FDModuleRust,
    // FPModule as FPModuleRust,
    // FreeModule as FreeModuleRust,
    FreeUnstableModule as FreeUnstableModuleRust,
    RealProjectiveSpace as RealProjectiveSpaceRust,
    KFpn as KFpnRust,
    BCp as BCpRust,
    Dickson2 as Dickson2Rust,
    ZeroModule
};

use pyo3::{prelude::*};//, exceptions, PyErr};
use crate::algebra::AlgebraRust;
use crate::module::{
    FDModule,
    FreeUnstableModule,
    RealProjectiveSpace,
    KFpn,
    BCp,
    Dickson2
};

// For escaping macro definition inside macro, see https://github.com/rust-lang/rust/issues/35853
macro_rules! with_dollar_sign {
    ($($body:tt)*) => {
        macro_rules! __with_dollar_sign { $($body)* }
        __with_dollar_sign!($);
    }
}


macro_rules! export_modules {
    ($(register($module_name : ident)),+) => {
        paste::item!{
            pub enum ModuleRust {
                $( $module_name([<$module_name Rust>]<AlgebraRust>) ),+
            }
        }
        with_dollar_sign!{
            ($d : tt) => {
                macro_rules! because_enum_dispatch_doesnt_work_for_me {
                    ($method : ident, $self_ : expr, $d( $d args : ident ),*) => { 
                        match $self_ {
                            $( ModuleRust::$module_name(module) =>  ModuleT::$method(module, $d($d args),*)),+ 
                        }
                    };
                }
            }
        }

        impl ModuleRust {
            pub fn into_py_object(module : Arc<ModuleRust>) -> PyObject {
                let gil = Python::acquire_gil();
                let py = gil.python();
                match *module {
                    $( ModuleRust::$module_name(_) =>  $module_name::wrap_immutable(module).into_py(py)),+ 
                }
            }

            pub fn from_py_object(module : PyObject) -> PyResult<Arc<ModuleRust>> {
                let gil = Python::acquire_gil();
                let py = gil.python();
                Err(python_utils::exception!(RuntimeError, "Dummy"))
                    $(.or_else(|_err : PyErr| Ok((&module).extract::<$module_name>(py)?.to_arc()?)))+
                    .map( |a| a.clone())
                    .map_err(|_err : PyErr| { python_utils::exception!(TypeError,
                        "Invalid module for from_py_object!"
                    )})
            }    
        }        
    };
}

export_modules! {
    register(FDModule),
    // register(FPModule),
    register(FreeUnstableModule),
    register(RealProjectiveSpace),
    register(KFpn),
    register(BCp),
    register(Dickson2)
}

impl ZeroModule for ModuleRust {
    fn zero_module(algebra: Arc<AlgebraRust>, min_degree: i32) -> Self {
        ModuleRust::FDModule(FDModuleRust::zero_module(algebra, min_degree))
    }
}

impl ModuleT for ModuleRust {
    type Algebra = AlgebraRust;

    fn algebra(&self) -> Arc<Self::Algebra> {
        because_enum_dispatch_doesnt_work_for_me!(algebra, self, )   
    }

    fn name(&self) -> String {
        because_enum_dispatch_doesnt_work_for_me!(name, self, )
    }

    fn min_degree(&self) -> i32 {
        because_enum_dispatch_doesnt_work_for_me!(min_degree, self, )
    }

    fn max_computed_degree(&self) -> i32 {
        because_enum_dispatch_doesnt_work_for_me!(max_computed_degree, self, )
    }

    fn compute_basis(&self, degree: i32) {
        because_enum_dispatch_doesnt_work_for_me!(compute_basis, self, degree)
    }

    fn dimension(&self, degree: i32) -> usize {
        because_enum_dispatch_doesnt_work_for_me!(dimension, self, degree)
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
        because_enum_dispatch_doesnt_work_for_me!(act_on_basis, self, result, coeff, op_degree, op_index, mod_degree, mod_index)
    }

    fn basis_element_to_string(&self, degree: i32, idx: usize) -> String {
        because_enum_dispatch_doesnt_work_for_me!(basis_element_to_string, self, degree, idx)
    }

    fn is_unit(&self) -> bool {
        because_enum_dispatch_doesnt_work_for_me!(is_unit, self, )
    }

    fn prime(&self) -> ValidPrime {
        because_enum_dispatch_doesnt_work_for_me!(prime, self, )
    }

    fn borrow_output(&self) -> bool {
        because_enum_dispatch_doesnt_work_for_me!(borrow_output, self, )
    }

    fn act_on_basis_borrow(
        &self,
        op_degree: i32,
        op_index: usize,
        mod_degree: i32,
        mod_index: usize,
    ) -> &FpVector {
        because_enum_dispatch_doesnt_work_for_me!(act_on_basis_borrow, self, op_degree, op_index, mod_degree, mod_index)
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
        because_enum_dispatch_doesnt_work_for_me!(act, self, result, coeff, op_degree, op_index, input_degree, input)
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
        because_enum_dispatch_doesnt_work_for_me!(act_by_element, self, result, coeff, op_degree, op, input_degree, input)
    }

    fn basis_string_list(&self, degree: i32) -> Vec<String> {
        because_enum_dispatch_doesnt_work_for_me!(basis_string_list, self, degree)
    }

    fn element_to_string(&self, degree: i32, element: &FpVector) -> String {
        because_enum_dispatch_doesnt_work_for_me!(element_to_string, self, degree, element)
    }
}
