#![macro_use]

use pyo3::{
    prelude::*,
    types::{PyDict, PyAny}
};

pub fn rename_submodule(module : &PyModule, name : &str, new_name : &str) -> PyResult<()> {
    let submodule = module.get(name)?;
    submodule.setattr("__name__", new_name)?;
    module.add(new_name, submodule)?;
    module.delattr(name)?;
    Ok(())
}

pub fn reduce_coefficient(p : u32, c : i32) -> u32 {
    let p = p as i32;
    (((c % p) + p) % p) as u32
}

pub fn null_ptr_exception() -> pyo3::PyErr {
    exception!(ReferenceError, "Null pointer!")
}

pub fn null_ptr_exception_if_none<T>(opt : Option<T>) -> pyo3::PyResult<()> {
    opt.map_or_else(
        || Err(null_ptr_exception()),
        |_x| Ok(())
    )
}

pub fn bidegree_string(s : u32, t : i32) -> String {
    format!("(s, t) = ({}, {}) <==> (x, y) = ({}, {})", s, t, t-s as i32, s)
}

#[macro_export]
macro_rules! exception {
    ($error_type : ident ) => {         
        pyo3::exceptions::$error_type::py_err("") 
    };
    ($error_type : ident, $msg:expr) => {  
        pyo3::exceptions::$error_type::py_err($msg)
    };
    ($error_type : ident, $msg:expr,) => { 
        pyo3::exceptions::$error_type::py_err($msg)
    };
    ($error_type : ident, $fmt:expr, $($arg:tt)+) => { 
        pyo3::exceptions::$error_type::py_err(format!($fmt, $($arg)*))
    };    
}

#[macro_export]
macro_rules! check_number_of_positional_arguments {
    ($f : expr, $args_expected : expr, $args_provided : expr) => {
        python_utils::check_number_of_positional_arguments($f, $args_expected, $args_expected, $args_provided)
    };
    ($f : expr, $min_args_expected : expr, $max_args_expected : expr, $args_provided : expr) => {
        if $args_provided > $max_args_expected {
            let args_expected = if $min_args_expected == $max_args_expected {
                format!("{}",$max_args_expected)
            } else {
                format!("from {} to {}", $min_args_expected, $max_args_expected)
            };
            let args_given = if $args_provided == 1 {
                format!("1 was given")
            } else {
                format!("{} were given", $args_provided)
            };
            Err(python_utils::exception!(TypeError,
                "{}() takes {} postional arguments but {}", $f, args_expected, args_given
            ))
        } else {
            Ok(())
        }
    };
}

#[macro_export]
macro_rules! release_gil {
    ($block : block) => {{
        let gil = Python::acquire_gil();
        let py = gil.python();
        py.allow_threads(move || $block)
    }};    
    ($expr : expr) => {
        python_utils::release_gil!({$expr})
    };
}

#[macro_export]
macro_rules! not_implemented_error {
    () => {
        python_utils::exception!(NotImplementedError, "Not implemented.")
    };
}

pub fn must_be_immutable_exception() -> pyo3::PyErr {
    exception!(
        ReferenceError,
        "Reference must be immutable for desired operation!"
    )
}

pub fn must_be_mutable_exception() -> pyo3::PyErr {
    exception!(
        ReferenceError,
        "Reference must be mutable for desired operation!"
    )
}

#[macro_export]
macro_rules! must_be_mutable_panic {
    () => { panic!( "Attempted to mutate immutable reference!" ) }
}

#[macro_export]
macro_rules! must_be_immutable_panic {
    () => { panic!("Reference must be immutable for desired operation!") }
}


pub fn check_not_null<T>(ptr : *mut T) -> pyo3::PyResult<()> {
    if ptr.is_null() {
        Err(null_ptr_exception())
    } else {
        Ok(())
    }
}

pub fn handle_index(dimension : usize, index : isize, dim_or_len : &str,  type_to_index : &str) -> PyResult<usize> {
    let result = 
        if index < 0 {
            dimension as isize + index
        } else {
            index
        };
    check_index(dimension, result, dim_or_len,  type_to_index)?;
    Ok(result as usize)
}

pub fn check_index(dimension : usize, index : isize, dim_or_len : &str,  type_to_index : &str) -> PyResult<()> {
    if index >= dimension as isize {
        Err(exception!(IndexError,
            "Index {} is greater than or equal to {} {} of {}.", index, dim_or_len, dimension, type_to_index
        ))
    } else if index < 0 {
        Err(exception!(IndexError,
            "Index {} is greater than {} {} of {}.", index - dimension as isize, dim_or_len, dimension, type_to_index
        ))
    } else {
        Ok(())
    }
}

use std::sync::{Arc, Weak};
pub fn weak_ptr_to_final<T>(ptr : Weak<T>) -> Weak<()> {
    unsafe { std::mem::transmute(ptr) }
}

pub fn arc_to_final<T>(ptr : &Arc<T>) -> Weak<()> {
    weak_ptr_to_final(Arc::downgrade(ptr))
}

// pub fn get_from_kwargs<'a, T : pyo3::FromPyObject<'a>>(
//     kwargs : Option<&'a PyDict>, argument : &str
// ) -> Option<PyResult<T>> {
//     kwargs.and_then(|dict| dict.get_item(argument))
//           .map(|value| PyAny::extract::<T>(value))
// }

pub fn get_from_kwargs<'a, T : pyo3::FromPyObject<'a>>(
    kwargs : Option<&'a PyDict>, argument : &str, default : T
) -> PyResult<T> {
    kwargs.and_then(|dict| dict.get_item(argument))
          .map(|value| PyAny::extract::<T>(value))
          .unwrap_or(Ok(default))
}

#[macro_export]
macro_rules! py_repr {
    ( $wrapper : ty, $freed_str : expr, $repr_block : block) => {
        #[pyproto]
        #[allow(unused_variables)]
        impl PyObjectProtocol for $wrapper {
            fn __repr__(&self) -> PyResult<String> {
                if self.is_null() {
                    Ok(format!($freed_str))
                } else {
                    let inner = self.inner_unchkd();
                    $repr_block
                }
            }
        }
    }
}

#[macro_export]
macro_rules! wrapper_outer_defs {
    ($outer : ident, $inner : ty) => {        
        impl $outer {
            // #![allow(dead_code)]
            pub fn inner(&self) -> PyResult<&$inner> {
                self.inner1()
            }
        
            pub fn inner_unchkd(&self) -> &$inner {
                self.inner_unchkd1()
            }

            pub fn inner_mut(&self) -> PyResult<&mut $inner> {
                self.inner_mut1()
            }
        
            pub fn inner_mut_unchkd(&self) -> &mut $inner {
                self.inner_mut_unchkd1()
            }            
        
            pub fn box_and_wrap(inner : $inner) -> Self {
                Self::box_and_wrap1(inner)
            }

            pub fn wrap<T>(to_wrap : &mut $inner, owner : std::sync::Weak<T>) -> Self {
                Self::wrap1(to_wrap, owner)
            }

            pub fn take_box(&mut self) -> PyResult<Box<$inner>> {
                self.take_box1()
            }
        }
    }
}


#[macro_export]
macro_rules! wrapper_outer_defs_dispatch_to_enum_variant {
    ($outer : ident, $enum_name : ident, $enum_variant : ident,  $inner : ty) => {    
        
        impl $outer {
            pub fn inner(&self) -> PyResult<&$inner> {
                match self.inner1()? {
                    $enum_name::$enum_variant(inner) => Ok(inner),
                    _ => panic!()
                }
            }
    
            pub fn inner_unchkd(&self) -> &$inner {
                match self.inner_unchkd1() {
                    $enum_name::$enum_variant(inner) => inner,
                    _ => panic!()
                }
            }        

            pub fn inner_mut(&mut self) -> PyResult<&mut $inner> {
                match self.inner_mut1()? {
                    $enum_name::$enum_variant(inner) => Ok(inner),
                    _ => panic!()
                }
            }
    
            pub fn inner_mut_unchkd(&mut self) -> &mut $inner {
                match self.inner_mut_unchkd1() {
                    $enum_name::$enum_variant(inner) => inner,
                    _ => panic!()
                }
            }

            pub fn box_and_wrap(inner : $inner) -> Self {
                Self::box_and_wrap1($enum_name::$enum_variant(inner))
            }

            // pub fn wrap<T>(to_wrap : &mut $inner, owner : std::sync::Weak<T>) -> Self {
            //     Self::wrap1(&mut $enum_name::$enum_variant(to_wrap), owner)
            // }

            // pub fn take_box(&mut self) -> PyResult<Box<$inner>> {
            //     let enum_inner = self.take_box1();
            //     match enum_inner {
            //         $enum_name::$enum_variant(inner) => Ok(inner),
            //         _ => panic!()
            //     }                
            // }
        }
    }
}




#[macro_export]
macro_rules! wrapper_type {
    ( $outer:ident, $inner:ty ) => {
        paste::item!{
            python_utils::wrapper_type_helper!($outer, [<$outer Enum>], $inner);
            python_utils::wrapper_outer_defs!($outer, $inner);
        }
    }
}

#[macro_export]
macro_rules! wrapper_type_inner {
    ( $outer:ident, $inner:ty ) => {
        paste::item!{
            python_utils::wrapper_type_helper!($outer, [<$outer Enum>], $inner);
        }
    }
}

#[macro_export]
macro_rules! wrapper_type_helper {
    ( $outer:ident, $enum_name : ident, $inner:ty ) => {
        #[pyclass(dict)]
        pub struct $outer {
            inner : $enum_name,
            // If we are the owner, we hold onto an Arc to keep it alive. Replace with None on free.
            owned : Option<std::sync::Arc<()>>, 
            // To check if we are freed, we test whether or not this weak pointer is still valid.
            // When our owner is freed, this Weak pointer will become invalid.
            freed : std::sync::Weak<()>
        }

        enum $enum_name {
            Mut(*mut $inner),
            Immut(*mut $inner)
        }

        impl $enum_name {
            fn take(self) -> *mut $inner {
                match self {
                    $enum_name::Mut(ptr) => ptr,
                    $enum_name::Immut(ptr) => ptr
                }
            }
        }

        impl $outer {
            pub fn is_null(&self) -> bool {
                self.freed.upgrade().is_none()
            }
            
            pub fn check_not_null(&self) -> PyResult<()> {
                if self.is_null() {
                    Err(python_utils::null_ptr_exception())
                } else {
                    Ok(())
                }
            }

            fn replace_with_null(&mut self) -> $enum_name {
                std::mem::replace(&mut self.inner, $enum_name::Mut(std::ptr::null_mut()))
            }

            pub fn is_mutable(&self) -> bool {
                match &self.inner {
                    $enum_name::Mut(_) => true,
                    $enum_name::Immut(_) => false
                }
            }            

            pub fn check_mutable(&self) -> PyResult<()> {
                if self.is_mutable() {
                    Ok(())
                } else {
                    Err(python_utils::must_be_mutable_exception())
                }
            }

            pub fn check_immutable(&self) -> PyResult<()> {
                if self.is_mutable() {
                    Err(python_utils::must_be_immutable_exception())
                } else {
                    Ok(())
                }
            }    

            pub fn ensure_immutable(&mut self) -> PyResult<()> {
                match &mut self.inner {
                    $enum_name::Immut(_) => {}
                    $enum_name::Mut(ptr) => {
                        self.inner = Self::wrap_immutable(
                            unsafe { &**ptr },
                            self.owner()
                        ).replace_with_null();
                    }
                }
                Ok(())
            }

            fn to_ptr(&self) -> *const $inner {
                match self.inner {
                    $enum_name::Mut(ptr) => ptr,
                    $enum_name::Immut(ptr) => ptr
                }
            }

            // This duplicates "inner" but has a more systematic name...
            // TODO: keep this one? keep both? 
            fn to_ref(&self) -> &$inner {
                unsafe { &*self.to_ptr() }
            }

            fn to_ptr_mut(&self) -> *mut $inner {
                match self.inner {
                    $enum_name::Mut(ptr) => ptr,
                    $enum_name::Immut(ptr) => python_utils::must_be_mutable_panic!()
                }
            }

            fn to_ref_mut(&self) -> &mut $inner {
                unsafe { &mut *self.to_ptr_mut() }
            }

            pub fn equal(&self, other : &$outer) -> bool {
                std::mem::discriminant(self) == std::mem::discriminant(other) 
                && self.to_ptr() == other.to_ptr()
            }

            // type Inner = $inner; // ==> "associated types are not yet supported in inherent imples" =(

            // #![allow(dead_code)]
            pub fn inner1(&self) -> PyResult<&$inner> {
                self.check_not_null()?;
                Ok(unsafe { &*self.to_ptr()})
            }
        
            pub fn inner_unchkd1(&self) -> &$inner {
                unsafe { &*self.to_ptr() }
            }

            pub fn inner_mut1(&self) -> PyResult<&mut $inner> {
                self.check_not_null()?;
                self.check_mutable()?;
                Ok(unsafe { &mut *self.to_ptr_mut() })
            }
        
            pub fn inner_mut_unchkd1(&self) -> &mut $inner {
                unsafe { &mut *self.to_ptr_mut() }
            }            
        
            pub fn box_and_wrap1(inner : $inner) -> Self {
                let inner_box = Box::new(inner);
                let owned = std::sync::Arc::new(());
                let freed = std::sync::Arc::downgrade(&owned);
                Self {
                    inner : $enum_name::Mut(Box::into_raw(inner_box)),
                    owned : Some(owned),
                    freed 
                }
            }

            pub fn owner(&self) -> std::sync::Weak<()> {
                self.freed.clone()
            }
        
            pub fn is_owned(&self) -> bool {
                self.owned.is_some()
            }

            pub fn check_owned(&self) -> PyResult<()>{
                if self.is_owned() {
                    Ok(())
                } else {
                    return Err(python_utils::exception!(ValueError,
                        "Illegal operation on reference that doesn't own its data."));
                }
            }
        }

        #[pymethods]
        impl $outer {
            pub fn free(&mut self) -> PyResult<()> {
                self.check_not_null()?;
                self.check_owned()?;
                self.owned = None;
                let ptr = self.replace_with_null().take();
                drop(unsafe { Box::from_raw(ptr) });
                Ok(())
            }

            #[getter]
            pub fn get_owned(&self) -> bool {
                self.is_owned()
            }
        }

        impl $outer {
            pub fn wrap1<T>(to_wrap : &mut $inner, owner : std::sync::Weak<T>) -> Self {
                let inner = to_wrap as *mut $inner;
                let freed = python_utils::weak_ptr_to_final(owner);
                Self {
                    inner : $enum_name::Mut(inner),
                    owned : None,
                    freed
                }
            }

            pub fn wrap_immutable<T>(to_wrap : &$inner, owner : std::sync::Weak<T> ) -> Self {
                let ptr = to_wrap as *const $inner;
                let inner : *mut $inner = unsafe { std::mem::transmute(ptr)};
                let freed = python_utils::weak_ptr_to_final(owner);
                Self {
                    inner : $enum_name::Immut(inner),
                    owned : None,
                    freed
                }
            }

            // This is nearly the same as free except:
            // (1) Here we check_mutable()?;
            // (2) we return the box instead of dropping it.
            pub fn take_box1(&mut self) -> PyResult<Box<$inner>> {
                self.check_not_null()?;
                self.check_owned()?;
                self.check_mutable()?;
                // Replace owned so other references are marked dead.
                self.owned = None; 
                let ptr = self.replace_with_null().take();
                Ok(unsafe { Box::from_raw(ptr) })
            }
        }

        impl Clone for $outer {
            fn clone(&self) -> $outer {
                let owner = self.freed.clone();
                match &self.inner {
                    $enum_name::Mut(_) => $outer::wrap(self.to_ref_mut(), owner),
                    $enum_name::Immut(_) => $outer::wrap_immutable(self.to_ref(), owner)
                }
            }
        }        

        impl Drop for $outer {
            fn drop(&mut self){
                drop(self.free()); // drop because I don't want to handle the Result of self.free()
                // println!("Dropping {}! ", std::intrinsics::type_name::<$outer>());
                // if self.is_owned() {
                //     println!("  An owned reference");
                //     // if self.is_null()
                // } else {
                //     println!("  An unowned reference");
                // }
            }
        }
    }
}


#[macro_export]
macro_rules! rc_wrapper_type {
    ( $outer:ident, $inner:ty ) => {
        paste::item!{
            python_utils::rc_wrapper_type_helper!($outer, [<$outer Enum>], [<$outer Wrapper>], $inner);
            python_utils::wrapper_outer_defs!($outer, $inner);
        }
    }
}

#[macro_export]
macro_rules! rc_wrapper_type_inner {
    ( $outer:ident, $inner:ty ) => {
        paste::item!{
            python_utils::rc_wrapper_type_helper!($outer, [<$outer Enum>], [<$outer Wrapper>], $inner);
        }
    }
}

#[macro_export]
macro_rules! rc_wrapper_type_helper {
    ( $outer:ident, $enum_name : ident, $mut_wrapper : ident,  $inner:ty ) => {

        #[pyclass(dict)]
        #[repr(transparent)]
        pub struct $outer {
            inner : $enum_name
        }
        

        enum $enum_name {
            Mut($mut_wrapper),
            Immut(std::sync::Arc<$inner>)
        }

        pub struct $mut_wrapper {
            ptr : *mut $inner,
            // If we are the owner, we hold onto an Arc to keep it alive. Replace with None on free.
            owned : Option<std::sync::Arc<()>>, 
            // To check if we are freed, we test whether or not this weak pointer is still valid.
            // When our owner is freed, this Weak pointer will become invalid.
            freed : std::sync::Weak<()>
        }

        impl $outer {
            pub fn is_null(&self) -> bool {
                match &self.inner {
                    $enum_name::Mut(wrapper) => { wrapper.freed.upgrade().is_none() }
                    $enum_name::Immut(_) => false
                }
            }
            
            pub fn check_not_null(&self) -> PyResult<()> {
                if self.is_null() {
                    Err(python_utils::null_ptr_exception())
                } else {
                    Ok(())
                }
            }

            fn replace_with_null(&mut self) -> $enum_name {
                std::mem::replace(&mut self.inner,
                    $enum_name::Mut($mut_wrapper {
                        ptr : std::ptr::null_mut(),
                        owned : None,
                        freed : std::sync::Weak::new()
                    })
                )
            }

            pub fn is_mutable(&self) -> bool {
                match self.inner {
                    $enum_name::Mut(_) => true,
                    $enum_name::Immut(_) => false
                }
            }            

            pub fn check_mutable(&self) -> PyResult<()> {
                if self.is_mutable() {
                    Ok(())
                } else {
                    Err(python_utils::must_be_mutable_exception())
                }
            }

            pub fn check_immutable(&self) -> PyResult<()> {
                if self.is_mutable() {
                    Err(python_utils::must_be_immutable_exception())
                } else {
                    Ok(())
                }
            }    

            pub fn ensure_immutable(&mut self) -> PyResult<()> {
                match &mut self.inner {
                    $enum_name::Immut(_) => {}
                    $enum_name::Mut(wrapper) => {
                        self.inner = Self::wrap_immutable(
                            std::sync::Arc::from(
                                unsafe { Box::from_raw(wrapper.ptr) }
                            )
                        ).replace_with_null();
                    }
                }
                Ok(())
            }

            pub fn to_arc(&self) -> PyResult<std::sync::Arc<$inner>> {
                self.check_not_null()?;
                self.check_immutable()?;
                match &self.inner {
                    $enum_name::Mut(_) => unreachable!(),
                    $enum_name::Immut(arc) => Ok(arc.clone())
                }                
            }

            // type Inner = $inner; // ==> "associated types are not yet supported in inherent imples" =(

            // #![allow(dead_code)]
            pub fn inner1(&self) -> PyResult<&$inner> {
                self.check_not_null()?;
                Ok(match &self.inner {
                    $enum_name::Mut(wrapper) => { unsafe { &*wrapper.ptr } }
                    $enum_name::Immut(arc) => &*arc
                })
            }
        
            pub fn inner_unchkd1(&self) -> &$inner {
                match &self.inner {
                    $enum_name::Mut(wrapper) => { unsafe { &*wrapper.ptr } }
                    $enum_name::Immut(arc) => &*arc
                }
            }

            pub fn inner_mut1(&self) -> PyResult<&mut $inner> {
                self.check_not_null()?;
                self.check_mutable()?;
                Ok(match &self.inner {
                    $enum_name::Mut(wrapper) => { unsafe { &mut *wrapper.ptr } }
                    _ => { unreachable!() }
                })
            }
        
            pub fn inner_mut_unchkd1(&self) -> &mut $inner {
                match &self.inner {
                    $enum_name::Mut(wrapper) => { unsafe { &mut *wrapper.ptr } }
                    _ => { python_utils::must_be_mutable_panic!() }
                }
            }            
        
            pub fn box_and_wrap1(inner : $inner) -> Self {
                let inner_box = Box::new(inner);
                let owned = std::sync::Arc::new(());
                let freed = std::sync::Arc::downgrade(&owned);
                let wrapper = $mut_wrapper {
                    ptr : Box::into_raw(inner_box),
                    owned : Some(owned),
                    freed 
                };
                Self {
                    inner : $enum_name::Mut(wrapper)
                }
            }

            pub fn owner(&self) -> std::sync::Weak<()> {
                match &self.inner {
                    $enum_name::Mut(wrapper) => { wrapper.freed.clone() },
                    $enum_name::Immut(arc) => {
                        python_utils::arc_to_final(&arc)
                    }
                }
            }
        
            pub fn is_owned(&self) -> bool {
                match &self.inner {
                    $enum_name::Mut(wrapper) => wrapper.owned.is_some(),
                    $enum_name::Immut(arc) => true   
                }
            }

            pub fn check_owned(&self) -> PyResult<()>{
                if self.is_owned() {
                    Ok(())
                } else {
                    return Err(python_utils::exception!(ValueError,
                        "Illegal operation on reference that doesn't own its data."));
                }
            }
        }

        #[pymethods]
        impl $outer {
            pub fn free(&mut self) -> PyResult<()> {
                self.check_not_null()?;
                self.check_owned()?;
                let inner = self.replace_with_null();
                match inner {
                    $enum_name::Mut(wrapper) => {
                        drop(unsafe { Box::from_raw(wrapper.ptr) });
                    },
                    _ => {}
                }
                Ok(())
            }

            pub fn freeze(&mut self) -> PyResult<()> {
                self.ensure_immutable()
            }

            #[getter]
            pub fn get_mutable(&self) -> bool {
                self.is_mutable()
            }

            #[getter]
            pub fn get_owned(&self) -> bool {
                self.is_owned()
            }
        }

        impl $outer {
            pub fn wrap1<T>(to_wrap : &mut $inner, owner : std::sync::Weak<T>) -> Self {
                let inner = to_wrap as *mut $inner;
                let freed = python_utils::weak_ptr_to_final(owner);
                let wrapper = $mut_wrapper {
                    ptr : inner,
                    owned : None,
                    freed
                };
                Self {
                    inner : $enum_name::Mut(wrapper)
                }
            }

            pub fn wrap_immutable(to_wrap : std::sync::Arc<$inner>) -> Self {
                Self {
                    inner : $enum_name::Immut(to_wrap)
                }
            }

            // This is nearly the same as free except:
            // (1) Here we check_mutable()?;
            // (2) we return the box instead of dropping it.
            pub fn take_box1(&mut self) -> PyResult<Box<$inner>> {
                self.check_not_null()?;
                self.check_owned()?;
                self.check_mutable()?;
                let inner = self.replace_with_null();
                match inner {
                    $enum_name::Mut(wrapper) => {
                        Ok(unsafe { Box::from_raw(wrapper.ptr) })
                    },
                    _ => { unreachable!() }
                }
            }
        }

        impl Clone for $outer {
            fn clone(&self) -> Self {
                match &self.inner {
                    $enum_name::Mut(wrapper) => {
                        let new_wrapper = $mut_wrapper {
                            ptr : wrapper.ptr,
                            owned : None,
                            freed : wrapper.freed.clone()
                        };
                        let new_enum = $enum_name::Mut(new_wrapper);
                        Self {
                            inner : new_enum
                        }
                    }
                    $enum_name::Immut(arc) => $outer::wrap_immutable(arc.clone())
                }
            }
        }        

        impl Drop for $outer {
            fn drop(&mut self){
                drop(self.free()); // drop because I don't want to handle the Result of self.free()
                // println!("Dropping {}! ", std::intrinsics::type_name::<$outer>());
                // if self.is_owned() {
                //     println!("  An owned reference");
                //     // if self.is_null()
                // } else {
                //     println!("  An unowned reference");
                // }
            }
        }
    }
}