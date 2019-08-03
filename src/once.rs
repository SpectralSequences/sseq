use spin;
use std::slice::Iter;

pub struct Once<T> {
    once : spin::Once<T>
} 

impl<T> Once<T> {
    pub fn new() -> Self {
        Self {
            once : spin::Once::new()
        }
    }  

    pub fn get(&self) -> &T {
        self.once.r#try().expect("Value hasn't been set yet.")
    }

    pub fn set(&self, value : T){
        let mut ran = false;
        let _result = self.once.call_once(|| {
            ran = true;
            value
        });
        assert!(ran, "Value was already set.");
    }

    pub fn call_once(&self, f : Box<FnOnce() -> T>){
        self.once.call_once(f);
    }

    pub fn get_option(&self) -> Option<&T> {
        self.once.r#try()
    }

    pub fn has(&self) -> bool {
        self.once.r#try().is_some()
    }
}

use core::cell::UnsafeCell;
use core::ops::Index;

pub struct OnceVec<T> {
    data : UnsafeCell<Vec<T>>
}

impl<T>  OnceVec<T> {
    pub fn from_vec(vec : Vec<T>) -> Self {
        Self {
            data : UnsafeCell::new(vec)
        }
    }

    pub fn new() -> Self {
        Self::from_vec(Vec::new())
    }

    pub fn with_capacity(capacity : usize) -> Self {
        Self::from_vec(Vec::with_capacity(capacity))
    }

    fn get_vec_mut(&self) -> &mut Vec<T> {
        unsafe { &mut *self.data.get() }
    }

    fn get_vec(&self) -> &Vec<T> {
        unsafe { &*self.data.get() }
    }

    pub fn reserve(&self, additional : usize) {
        self.get_vec_mut().reserve(additional);
    }

    pub fn reserve_exact(&self, additional : usize) {
        self.get_vec_mut().reserve_exact(additional);
    }

    pub fn len(&self) -> usize {
        self.get_vec().len()
    }

    pub fn get(&self, i : usize) -> &T {
        unsafe { &((*self.data.get())[i]) }
    }

    pub fn push(&self, x : T) {
        unsafe { (*self.data.get()).push(x); }
    }

    pub fn iter(&self) -> Iter<T> {
        self.get_vec().iter()
    }
}

impl<T> Index<usize> for OnceVec<T> {
    type Output = T;
    fn index(&self, key : usize) -> &T {
        self.get(key)
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    
    #[test]
    #[should_panic(expected = "Value hasn't been set yet.")]
    fn test_once_failed_get(){
        let once : Once<u32> = Once::new();
        once.get();
    }

    #[test]
    #[should_panic(expected = "Value was already set.")]
    fn test_once_failed_set(){
        let once : Once<u32> = Once::new();
        once.set(5);
        once.set(5);
    }

    #[test]
    fn test_once_set_get(){
        let once : Once<u32> = Once::new();
        once.set(5);
        assert!(*once.get() == 5);
    }

    #[test]
    fn test_once_vec(){
        let v : OnceVec<u32> = OnceVec::new();
        v.push(3);
        v.push(5);
        assert_eq!(v[0], 3);
        v.reserve(1000);
        assert_eq!(v[0], 3);
    }
}
