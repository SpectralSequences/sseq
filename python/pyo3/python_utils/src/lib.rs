#![macro_use]

use pyo3::{
    prelude::*,
    exceptions,
    ObjectProtocol,
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
    exceptions::ReferenceError::py_err(
        "Null pointer!"
    )
}

pub fn null_ptr_exception_if_none<T>(opt : Option<T>) -> pyo3::PyResult<()> {
    opt.map_or_else(
        || Err(null_ptr_exception()),
        |_x| Ok(())
    )
}

pub fn mutability_exception() -> pyo3::PyErr {
    exceptions::ReferenceError::py_err(
        "Attempted to mutate immutable reference!"
    )
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
        Err(exceptions::IndexError::py_err(
            format!("Index {} is greater than or equal to {} {} of {}.", index, dim_or_len, dimension, type_to_index)
        ))
    } else if index < 0 {
        Err(exceptions::IndexError::py_err(
            format!("Index {} is greater than {} {} of {}.", index - dimension as isize, dim_or_len, dimension, type_to_index)
        ))
    } else {
        Ok(())
    }
}

use std::sync::Weak;
pub fn weak_ptr_to_final<T>(ptr : Weak<T>) -> Weak<()> {
    unsafe { std::mem::transmute(ptr) }
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
macro_rules! wrapper_type {
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

            pub fn set_to_null(&mut self) {
                self.inner = $enum_name::Mut(std::ptr::null_mut());
            }

            pub fn is_mutable(&self) -> bool {
                match self.inner {
                    $enum_name::Mut(ptr) => true,
                    $enum_name::Immut(ptr) => false
                }
            }            

            pub fn check_mutable(&self) -> PyResult<()> {
                if self.is_mutable() {
                    Ok(())
                } else {
                    Err(python_utils::mutability_exception())
                }
            }

            fn ptr_to_mut(&self) -> *mut $inner {
                match self.inner {
                    $enum_name::Mut(ptr) => ptr,
                    $enum_name::Immut(ptr) => ptr
                }
            }

            pub fn equal(&self, other : &$outer) -> bool {
                std::mem::discriminant(self) == std::mem::discriminant(other) 
                && self.ptr_to_mut() == other.ptr_to_mut()
            }

            // type Inner = $inner; // ==> "associated types are not yet supported in inherent imples" =(

            // #![allow(dead_code)]
            pub fn inner(&self) -> PyResult<&$inner> {
                self.check_not_null()?;
                Ok(unsafe { &*self.ptr_to_mut()})
            }
        
            pub fn inner_unchkd(&self) -> &$inner {
                unsafe { &*self.ptr_to_mut() }
            }

            pub fn inner_mut(&self) -> PyResult<&mut $inner> {
                self.check_not_null()?;
                self.check_mutable()?;
                Ok(unsafe { &mut *self.ptr_to_mut() })
            }
        
            pub fn inner_mut_unchkd(&self) -> &mut $inner {
                if !self.is_mutable() {
                    panic!("Attempting to mutate immutable object.");
                }
                unsafe { &mut *self.ptr_to_mut() }
            }            
        
            pub fn box_and_wrap(inner : $inner) -> Self {
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
                    return Err(pyo3::exceptions::ValueError::py_err(
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
                let ptr = self.ptr_to_mut();
                self.set_to_null();
                drop(unsafe { Box::from_raw(ptr) });
                Ok(())
            }

            #[getter]
            pub fn get_owned(&self) -> bool {
                self.owned.is_some()
            }
        }

        impl $outer {
            pub fn wrap<T>(to_wrap : &mut $inner, owner : std::sync::Weak<T>) -> Self {
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
            pub fn take_box(&mut self) -> PyResult<Box<$inner>> {
                self.check_not_null()?;
                self.check_owned()?;
                self.check_mutable()?;
                // Replace owned so other references are marked dead.
                self.owned = None; 
                let ptr = self.ptr_to_mut();
                self.set_to_null();
                Ok(unsafe { Box::from_raw(ptr) })
            }
        }

        impl Clone for $outer {
            fn clone(&self) -> $outer {
                let to_wrap = unsafe { &mut *self.ptr_to_mut() };
                let owner = self.freed.clone();
                match self.inner {
                    $enum_name::Mut(_) => $outer::wrap(to_wrap, owner),
                    $enum_name::Immut(_) => $outer::wrap_immutable(to_wrap, owner)
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
macro_rules! rc_inner_wrapper_type {
    ( $outer:ident, $inner:ty ) => {

        #[pyclass(dict)]
        #[derive(Clone)]
        #[repr(transparent)]
        pub struct $outer {
            inner : Option<std::sync::Arc<$inner>>
        }

        impl $outer {
            // type Inner = $inner; // ==> "associated types are not yet supported in inherent imples" =(

            #![allow(dead_code)]
            pub fn inner_rc(&self) -> PyResult<&std::sync::Arc<$inner>> {
                self.inner.as_ref().ok_or_else(
                    || python_utils::null_ptr_exception()
                )
            }
        
            pub fn inner_rc_unchkd(&self) -> &std::sync::Arc<$inner> {
                self.inner.as_ref().unwrap()
            }
        
            pub fn box_and_wrap_rc(inner : $inner) -> Self {
                Self {
                    inner : Some(std::sync::Arc::new(inner))
                }
            }

            pub fn owner(&self) -> std::sync::Weak<()> {
                self.inner.as_ref().map(|ptr| 
                    python_utils::weak_ptr_to_final(std::sync::Arc::downgrade(ptr))
                ).unwrap_or_else(|| std::sync::Weak::new()) 
                    // TODO: this else behavior may not be right...
            }

            pub fn is_null(&self) -> bool {
                self.inner.is_none()
            }

            pub fn check_not_null(&self) -> PyResult<()> {
                python_utils::null_ptr_exception_if_none(self.inner.as_ref())
            }
        
            pub fn is_owned(&self) -> bool {
                true
            }

            pub fn check_owned(&self) -> PyResult<()>{
                Ok(())
            }
        }

        #[pymethods]
        impl $outer {
            pub fn free(&mut self) -> PyResult<()> {
                python_utils::null_ptr_exception_if_none(self.inner.take())?;
                Ok(())
            }

            #[getter]
            pub fn get_owned(&self) -> bool {
                true
            }
        }

        impl $outer {
            // type Inner = $inner; // ==> "associated types are not yet supported in inherent imples" =(
    
            pub fn wrap(inner : std::sync::Arc<$inner>) -> Self {
                Self {
                    inner : Some(inner),
                }
            }

            pub fn take_box(&mut self) -> PyResult<std::sync::Arc<$inner>> {
                self.inner.take().ok_or_else(|| python_utils::null_ptr_exception())
            }
        }
    }
}

#[macro_export]
macro_rules! rc_wrapper_type {
    ( $outer:ident, $inner:ty ) => {
        python_utils::rc_inner_wrapper_type!($outer, $inner);

        impl $outer {
            // type Inner = $inner; // ==> "associated types are not yet supported in inherent imples" =(

            #![allow(dead_code)]
            pub fn inner(&self) -> PyResult<&std::sync::Arc<$inner>> {
                self.inner_rc()
            }
        
            pub fn inner_unchkd(&self) -> &std::sync::Arc<$inner> {
                self.inner_rc_unchkd()
            }
        
            pub fn box_and_wrap(inner : $inner) -> Self {
                Self::box_and_wrap_rc(inner)
            }
            
        }
    }
}