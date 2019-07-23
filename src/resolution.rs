use crate::free_module::FreeModule;
use crate::free_module_homomorphism::FreeModuleHomomorphism;
use crate::chain_complex::ChainComplex;

struct Resolution<'a> {
    complex : &'a mut ChainComplex,
    modules : Vec<FreeModule<'a>>,
    differentials : Vec<FreeModuleHomomorphism<'a>>,
    add_class : fn(hom_deg : usize, int_deg : i32, name : &str),
    add_structline : fn(
        sl_type : &str,
        source_hom_deg : usize, source_int_deg : i32, source_idx : usize, 
        target_hom_deg : usize, target_int_deg : i32, target_idx : usize
    )
}

