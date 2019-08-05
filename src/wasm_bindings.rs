use wasm_bindgen::prelude::*;

use std::rc::Rc;
use std::mem::transmute;
use serde_json::value::Value;

use crate::algebra::Algebra;
use crate::adem_algebra::AdemAlgebra;
use crate::module::Module;
use crate::finite_dimensional_module::FiniteDimensionalModule as FDModule;
// use crate::module_homomorphism::ModuleHomomorphism;
use crate::chain_complex::{ChainComplex, ChainComplexConcentratedInDegreeZero as CCDZ};
use crate::resolution::{Resolution, FDModuleResolution};


// use web_sys::console;

#[wasm_bindgen]
pub struct WasmAlgebra {
    pimpl : *const AdemAlgebra
}

#[wasm_bindgen]
impl WasmAlgebra {
    pub fn new_adem_algebra(p : u32, generic : bool, max_degree : i32) -> Self {
        let algebra = AdemAlgebra::new(p, generic, false);
        let boxed_algebra = Rc::new(algebra);
        Self {
            pimpl : Rc::into_raw(boxed_algebra)
        }
    }

    pub fn compute_basis(&self, degree : i32) {
        self.to_adem_algebra().compute_basis(degree);
    }

    fn to_adem_algebra(&self) -> Rc<AdemAlgebra> {
        let raw = unsafe { Rc::from_raw(self.pimpl) };
        let clone = Rc::clone(&raw);
        std::mem::forget(raw);
        clone
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}

#[wasm_bindgen]
pub struct WasmFDModule {
    pimpl : *const FDModule
}

#[wasm_bindgen]
impl WasmFDModule {
    pub fn new_adem_module(algebra : &WasmAlgebra, json_string : String) -> WasmFDModule {
        let mut json : Value = serde_json::from_str(&json_string).unwrap();
        let module = FDModule::from_json(algebra.to_adem_algebra(), "adem", &mut json);
        let boxed_module = Rc::new(module);
        Self {
            pimpl : Rc::into_raw(boxed_module)
        }
    }

    fn to_module(&self) -> Rc<FDModule> {
        unsafe { Rc::clone(&Rc::from_raw(self.pimpl)) }
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}

#[wasm_bindgen]
pub struct WasmCCDZ {
    pimpl : *const CCDZ<FDModule>
}


#[wasm_bindgen]
impl WasmCCDZ {
    pub fn new_ccdz(module : &WasmFDModule) -> Self {
        let cc = CCDZ::new(module.to_module());
        let boxed_cc : Rc<CCDZ<FDModule>> = Rc::new(cc);
        Self {
            pimpl : Rc::into_raw(boxed_cc)
        }
    }

    fn to_chain_complex(&self) -> Rc<CCDZ<FDModule>> {
        unsafe { Rc::clone(&Rc::from_raw(self.pimpl)) }
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}


#[wasm_bindgen]
pub struct WasmResolution {
   pimpl : *const FDModuleResolution
}

#[wasm_bindgen]
impl WasmResolution {
    pub fn new(chain_complex : &WasmCCDZ, max_degree : i32, add_class : js_sys::Function, add_structline : js_sys::Function) -> Self {
        let chain_complex = chain_complex.to_chain_complex();
        let algebra = chain_complex.get_algebra();
        algebra.compute_basis(max_degree);
        
        let add_class_wrapper = move |hom_deg : u32, int_deg : i32, name : &str| {
            let this = JsValue::NULL;
            let js_hom_deg = JsValue::from(hom_deg);
            let js_int_deg = JsValue::from(int_deg);
            let js_name = JsValue::from(name);
            add_class.call3(&this, &js_hom_deg, &js_int_deg, &js_name).unwrap();
        };
        let add_class_wrapper_box = Box::new(add_class_wrapper);
        let add_stuctline_wrapper = 
            move | name : &str, 
                source_hom_deg : u32, source_int_deg : i32, source_idx : usize,
                target_hom_deg : u32, target_int_deg : i32, target_idx : usize |
        {
            let this = JsValue::NULL;
            let args_array = js_sys::Array::new();
            args_array.push(&JsValue::from(name));
            args_array.push(&JsValue::from(source_hom_deg));
            args_array.push(&JsValue::from(source_int_deg));
            args_array.push(&JsValue::from(source_idx as u32));
            args_array.push(&JsValue::from(target_hom_deg));
            args_array.push(&JsValue::from(target_int_deg));
            args_array.push(&JsValue::from(target_idx as u32));
            add_structline.apply(&this, &args_array).unwrap_throw();
        };
        let add_stuctline_wrapper_box = Box::new(add_stuctline_wrapper);
        let res = Resolution::new(chain_complex, max_degree, Some(add_class_wrapper_box), Some(add_stuctline_wrapper_box)); 
        let boxed_res = Box::new(res);
        let pimpl : *const FDModuleResolution = Box::into_raw(boxed_res);
        Self {
            pimpl
        }
    }
 
    // pub fn step(&self, hom_deg : u32, int_deg : i32) {

    // }

    pub fn resolve_through_degree(&self, degree : i32) {
        let res = unsafe { &*self.pimpl };
        res.resolve_through_degree(degree);
    }

    pub fn get_cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> String {
        let res = unsafe { &*self.pimpl };
        return res.get_cocycle_string(hom_deg, int_deg, idx);
    }

    pub fn free(self) {
         let _drop_me :  Box<FDModuleResolution> = unsafe {
              transmute(self.pimpl)
         };
    }
}
