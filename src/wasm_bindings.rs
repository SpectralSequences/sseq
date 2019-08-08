use wasm_bindgen::prelude::*;

use std::rc::Rc;
use serde_json::value::Value;

use crate::algebra::{Algebra, AlgebraAny};
use crate::adem_algebra::AdemAlgebra;
use crate::milnor_algebra::MilnorAlgebra;
use crate::module::Module;
use crate::finite_dimensional_module::FiniteDimensionalModule as FDModule;
use crate::finitely_presented_module::FinitelyPresentedModule as FPModule;
// use crate::module_homomorphism::ModuleHomomorphism;
use crate::chain_complex::{ChainComplex, ChainComplexConcentratedInDegreeZero as CCDZ};
use crate::resolution::{Resolution, ModuleResolution};
use crate::resolution_with_chain_maps::{ResolutionWithChainMaps, ModuleResolutionWithChainMaps};


// use web_sys::console;

#[wasm_bindgen]
pub struct WasmAlgebra {
    pimpl : *const AlgebraAny
}

#[wasm_bindgen]
impl WasmAlgebra {
    pub fn new_adem_algebra(p : u32, generic : bool) -> Self {
        let mut algebra = AlgebraAny::from(AdemAlgebra::new(p, generic, false));
        algebra.set_default_filtration_one_products();
        let boxed_algebra = Rc::new(algebra);
        Self {
            pimpl : Rc::into_raw(boxed_algebra)
        }
    }

    pub fn new_milnor_algebra(p : u32) -> Self {
        let mut algebra = AlgebraAny::from(MilnorAlgebra::new(p));
        algebra.set_default_filtration_one_products();
        let boxed_algebra = Rc::new(algebra);
        Self {
            pimpl : Rc::into_raw(boxed_algebra)
        }
    }

    pub fn compute_basis(&self, degree : i32) {
        self.to_algebra().compute_basis(degree);
    }

    fn to_algebra(&self) -> Rc<AlgebraAny> {
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
        let module = FDModule::from_json(algebra.to_algebra(), "adem", &mut json);
        let boxed_module = Rc::new(module);
        Self {
            pimpl : Rc::into_raw(boxed_module)
        }
    }

    fn to_module(&self) -> Rc<FDModule> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}


#[wasm_bindgen]
pub struct WasmFPModule {
    pimpl : *const FPModule
}

#[wasm_bindgen]
impl WasmFPModule {
    pub fn new_adem_module(algebra : &WasmAlgebra, json_string : String) -> WasmFPModule {
        let mut json : Value = serde_json::from_str(&json_string).unwrap();
        let module = FPModule::from_json(algebra.to_algebra(), "adem", &mut json);
        let boxed_module = Rc::new(module);
        Self {
            pimpl : Rc::into_raw(boxed_module)
        }
    }

    fn to_module(&self) -> Rc<FPModule> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}

#[wasm_bindgen]
pub struct WasmCCDZFDModule {
    pimpl : *const CCDZ<FDModule>
}


#[wasm_bindgen]
impl WasmCCDZFDModule {
    pub fn new_ccdz(module : &WasmFDModule) -> Self {
        let cc = CCDZ::new(module.to_module());
        let boxed_cc : Rc<CCDZ<FDModule>> = Rc::new(cc);
        Self {
            pimpl : Rc::into_raw(boxed_cc)
        }
    }

    fn to_chain_complex(&self) -> Rc<CCDZ<FDModule>> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}

#[wasm_bindgen]
pub struct WasmCCDZFPModule {
    pimpl : *const CCDZ<FPModule>
}


#[wasm_bindgen]
impl WasmCCDZFPModule {
    pub fn new_ccdz(module : &WasmFPModule) -> Self {
        let cc = CCDZ::new(module.to_module());
        let boxed_cc : Rc<CCDZ<FPModule>> = Rc::new(cc);
        Self {
            pimpl : Rc::into_raw(boxed_cc)
        }
    }

    fn to_chain_complex(&self) -> Rc<CCDZ<FPModule>> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    pub fn free(self) {
        let _drop_me = unsafe { Rc::from_raw(self.pimpl) };
    }
}


#[wasm_bindgen]
pub struct WasmResolutionCCDZFDModule {
   pimpl : *const ModuleResolution<FDModule>
}

#[wasm_bindgen]
impl WasmResolutionCCDZFDModule {
    pub fn new(chain_complex : &WasmCCDZFDModule, max_degree : i32, add_class : js_sys::Function, add_structline : js_sys::Function) -> Self {
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
        let boxed_res = Rc::new(res);
        let pimpl : *const ModuleResolution<FDModule> = Rc::into_raw(boxed_res);
        Self {
            pimpl
        }
    }
 
    fn to_resolution(&self) -> Rc<ModuleResolution<FDModule>> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    // pub fn step(&self, hom_deg : u32, int_deg : i32) {

    // }

    pub fn resolve_through_degree(&self, degree : i32) {
        self.to_resolution().resolve_through_degree(degree);
    }

    pub fn get_cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> String {
        self.to_resolution().get_cocycle_string(hom_deg, int_deg, idx)
    }

    pub fn free(self) {
         let _drop_me :  Rc<ModuleResolution<FDModule>> 
            = unsafe { Rc::from_raw(self.pimpl) };
    }
}


#[wasm_bindgen]
pub struct WasmResolutionCCDZFPModule {
   pimpl : *const ModuleResolution<FPModule>
}

#[wasm_bindgen]
impl WasmResolutionCCDZFPModule {
    pub fn new(chain_complex : &WasmCCDZFPModule, max_degree : i32, add_class : js_sys::Function, add_structline : js_sys::Function) -> Self {
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
        let boxed_res = Rc::new(res);
        let pimpl : *const ModuleResolution<FPModule> = Rc::into_raw(boxed_res);
        Self {
            pimpl
        }
    }
 
    fn to_resolution(&self) -> Rc<ModuleResolution<FPModule>> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }
 
    // pub fn step(&self, hom_deg : u32, int_deg : i32) {

    // }

    pub fn resolve_through_degree(&self, degree : i32) {
        self.to_resolution().resolve_through_degree(degree);
    }

    pub fn get_cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> String {
        self.to_resolution().get_cocycle_string(hom_deg, int_deg, idx)
    }

    pub fn free(self) {
         let _drop_me : Rc<ModuleResolution<FPModule>> 
            = unsafe { Rc::from_raw(self.pimpl) };
    }
}

#[wasm_bindgen]
pub struct WasmResolutionWithChainMapsCCDZFDModule {
   pimpl : *const ModuleResolutionWithChainMaps<FDModule, FDModule>
}

// use crate::fp_vector::FpVectorT;
#[wasm_bindgen]
impl WasmResolutionWithChainMapsCCDZFDModule {
    pub fn new(source : &WasmResolutionCCDZFDModule, target : &WasmResolutionCCDZFDModule, json_string : String) -> Self {
        let source_res = source.to_resolution();
        let target_res = target.to_resolution();
        let mut res_with_maps = ResolutionWithChainMaps::new(source_res, target_res);
        let mut json : Value = serde_json::from_str(&json_string).unwrap();
        res_with_maps.add_from_json(&mut json);
        // res_with_maps.add_product(2, 9, 0, "\\alpha_{2}".to_string());
        // res_with_maps.add_product(2, 12, 0, "\\beta".to_string());
        // let mut map_data = crate::matrix::Matrix::new(2, 1, 1);
        // map_data[0].set_entry(0, 1);
        // res_with_maps.add_self_map(4, 12, "v_1".to_string(), map_data);
        let boxed_res_with_maps = Rc::new(res_with_maps);
        let pimpl : *const ModuleResolutionWithChainMaps<FDModule, FDModule> = Rc::into_raw(boxed_res_with_maps);
        Self {
            pimpl
        }
    }

    fn to_res_with_maps(&self) -> Rc<ModuleResolutionWithChainMaps<FDModule, FDModule>> {
        unsafe { 
            let raw = Rc::from_raw(self.pimpl);
            let result = Rc::clone(&raw);
            std::mem::forget(raw);
            return result;
        }
    }

    // pub fn add_product(&mut self, homological_degree : u32, internal_degree : i32, index : usize, name : String) {
    //     self.to_res_with_maps().add_product(homological_degree, internal_degree, index, name)
    // }

    pub fn resolve_through_degree(&self, degree : i32) {
        self.to_res_with_maps().resolve_through_degree(degree);
    }

    pub fn get_cocycle_string(&self, hom_deg : u32, int_deg : i32, idx : usize) -> String {
        self.to_res_with_maps().resolution.get_cocycle_string(hom_deg, int_deg, idx)
    }

    pub fn free(self) {
         let _drop_me :  Rc<ModuleResolutionWithChainMaps<FDModule,FDModule>> 
            = unsafe{ Rc::from_raw(self.pimpl) };
    }
}
