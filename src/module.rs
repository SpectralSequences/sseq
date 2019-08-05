use std::rc::Rc;
// use enum_dispatch::enum_dispatch;

use crate::fp_vector::{FpVector, FpVectorT};
use crate::algebra::Algebra;

// enum Module_Type {

// }

pub trait Module {
    fn get_prime(&self) -> u32 {
        self.get_algebra().get_prime()
    }
    fn get_algebra(&self) -> Rc<dyn Algebra>;
    fn get_name(&self) -> &str;
    fn get_min_degree(&self) -> i32;
    fn compute_basis(&self, _degree : i32) {}
    fn get_dimension(&self, degree : i32) -> usize;
    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize);
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String;

    fn act(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, input_degree : i32, input : &FpVector){
        assert!(input.get_dimension() == self.get_dimension(input_degree));
        let p = self.get_algebra().get_prime();
        for (i, v) in input.iter().enumerate() {
            if v == 0 {
                continue;
            }
            self.act_on_basis(result, (coeff * v) % p, op_degree, op_index, input_degree, i);
        }
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

pub struct ZeroModule {
    algebra : Rc<dyn Algebra>,
    name : String
}

impl ZeroModule {
    pub fn new(algebra : Rc<dyn Algebra>) -> Self {
        let name = format!("Zero Module over {}", algebra.get_name());
        ZeroModule {
            algebra,
            name
        }
    }
}

impl Module for ZeroModule {
    fn get_algebra(&self) -> Rc<dyn Algebra> {
        Rc::clone(&self.algebra)
    }
    
    fn get_name(&self) -> &str{
        &self.name
    }

    fn get_dimension(&self, _degree : i32) -> usize {
        0
    }

    fn get_min_degree(&self) -> i32 {
        0
    }

    // Since the dimension is 0, the input of this function is an element of the basis which is the empty set.
    fn act_on_basis(&self, _result : &mut FpVector, _coeff : u32, _op_degree : i32, _op_index : usize, _mod_degree : i32, _mod_index : usize){
        assert!(false);
    }

    fn basis_element_to_string(&self, _degree : i32, _index : usize) -> String{
        assert!(false);
        "".to_string()
    }
}

// #[enum_dispatch(Module)]
// pub enum ModuleChoice<L : Module, R : Module> {
//     IntroL(L),
//     IntroR(R)
// }

// impl<L : Module, R : Module> Module for ModuleChoice<L, R> {
//     fn get_algebra(&self) -> Rc<dyn Algebra> {
//         match self {
//             ModuleChoice::IntroL(l) => l.get_algebra(),
//             ModuleChoice::IntroR(r) => r.get_algebra()
//         }
//     }

//     fn get_name(&self) -> &str {
//         match self {
//             ModuleChoice::IntroL(l) => l.get_name(),
//             ModuleChoice::IntroR(r) => r.get_name()
//         }
//     }

//     fn get_min_degree(&self) -> i32 {
//         match self {
//             ModuleChoice::IntroL(l) => l.get_min_degree(),
//             ModuleChoice::IntroR(r) => r.get_min_degree()
//         }
//     }

//     fn compute_basis(&mut self, degree : i32) {
//         match self {
//             ModuleChoice::IntroL(l) => l.compute_basis(degree),
//             ModuleChoice::IntroR(r) => r.compute_basis(degree)
//         }
//     }

//     fn get_dimension(&self, degree : i32) -> usize {
//         match self {
//             ModuleChoice::IntroL(l) => l.get_dimension(degree),
//             ModuleChoice::IntroR(r) => r.get_dimension(degree)
//         }
//     }

//     fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize){
//         match self {
//             ModuleChoice::IntroL(l) => l.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index),
//             ModuleChoice::IntroR(r) => r.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index)
//         }
//     }

//     fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
//         match self {
//             ModuleChoice::IntroL(l) => l.basis_element_to_string(degree, idx),
//             ModuleChoice::IntroR(r) => r.basis_element_to_string(degree, idx)
//         }
//     }
// }


pub enum OptionModule<M : Module> {
    Some(Rc<M>),
    Zero(Rc<ZeroModule>)
}

impl<M : Module> Module for OptionModule<M> {
    fn get_algebra(&self) -> Rc<dyn Algebra> {
        match self {
            OptionModule::Some(l) => l.get_algebra(),
            OptionModule::Zero(r) => r.get_algebra()
        }
    }

    fn get_name(&self) -> &str {
        match self {
            OptionModule::Some(l) => l.get_name(),
            OptionModule::Zero(r) => r.get_name()
        }
    }

    fn get_min_degree(&self) -> i32 {
        match self {
            OptionModule::Some(l) => l.get_min_degree(),
            OptionModule::Zero(r) => r.get_min_degree()
        }
    }

    fn compute_basis(&self, degree : i32) {
        match self {
            OptionModule::Some(l) => l.compute_basis(degree),
            OptionModule::Zero(r) => r.compute_basis(degree)
        }
    }

    fn get_dimension(&self, degree : i32) -> usize {
        match self {
            OptionModule::Some(l) => l.get_dimension(degree),
            OptionModule::Zero(r) => r.get_dimension(degree)
        }
    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize){
        match self {
            OptionModule::Some(l) => l.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index),
            OptionModule::Zero(r) => r.act_on_basis(result, coeff, op_degree, op_index, mod_degree, mod_index)
        }
    }

    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        match self {
            OptionModule::Some(l) => l.basis_element_to_string(degree, idx),
            OptionModule::Zero(r) => r.basis_element_to_string(degree, idx)
        }
    }
}