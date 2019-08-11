use std::slice::Iter;

use core::cell::UnsafeCell;
use core::ops::Index;

use bivec::BiVec;

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
        &(self.get_vec()[i])
    }

    pub fn push(&self, x : T) {
        self.get_vec_mut().push(x);
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

impl<T> Index<u32> for OnceVec<T> {
    type Output = T;
    fn index(&self, key : u32) -> &T {
        self.get(key as usize)
    }
}

pub struct OnceBiVec<T> {
    data : UnsafeCell<BiVec<T>>
}

impl<T> OnceBiVec<T> {
    pub fn from_bivec(bivec : BiVec<T>) -> Self {
        Self {
            data : UnsafeCell::new(bivec)
        }
    }

    pub fn new(min_degree : i32) -> Self {
        Self::from_bivec(BiVec::new(min_degree))
    }

    pub fn with_capacity(min_degree : i32, capacity : i32) -> Self {
        Self::from_bivec(BiVec::with_capacity(min_degree, capacity))
    }

    unsafe fn get_bivec_mut(&self) -> &mut BiVec<T> {
        &mut *self.data.get()
    }

    unsafe fn get_bivec(&self) -> &BiVec<T> {
        &*self.data.get()
    }

    pub fn len(&self) -> i32 {
        unsafe { self.get_bivec().len() }
    }

    pub fn push(&self, x : T) {
        unsafe { self.get_bivec_mut().push(x); }
    }

    pub fn iter(&self) -> Iter<T> {
        unsafe { self.get_bivec().iter() }
    }
}

impl<T> Index<i32> for OnceBiVec<T> {
    type Output = T;
    fn index(&self, key : i32) -> &T {
        unsafe { &(self.get_bivec()[key]) }
    }
}

pub struct TempStorage<T> {
    data : UnsafeCell<Option<T>>
}

impl<T> TempStorage<T> {
    pub fn new(object : T) -> Self {
        Self {
            data : UnsafeCell::new(Some(object))
        }
    }

    pub fn take(&self) -> T {
        let maybe_t = unsafe { &mut *self.data.get() };
        maybe_t.take().unwrap()
    }
}
