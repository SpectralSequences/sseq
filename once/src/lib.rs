use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut, Index, IndexMut};

use bivec::BiVec;

#[derive(Debug)]
pub struct OnceVec<T> {
    data : UnsafeCell<Vec<T>>
}

impl<T>  OnceVec<T> {
    pub fn to_vec(self) -> Vec<T> {
        self.data.into_inner()
    }

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

    pub fn reserve(&self, additional : usize) {
        self.get_vec_mut().reserve(additional);
    }

    pub fn reserve_exact(&self, additional : usize) {
        self.get_vec_mut().reserve_exact(additional);
    }

    pub fn push(&self, x : T) {
        self.get_vec_mut().push(x);
    }
}

impl<T> Index<usize> for OnceVec<T> {
    type Output = T;
    fn index(&self, key : usize) -> &T {
        &Deref::deref(self)[key]
    }
}

impl<T> IndexMut<usize> for OnceVec<T> {
    fn index_mut(&mut self, key : usize) -> &mut T {
        &mut DerefMut::deref_mut(self)[key]
    }
}

impl<T> Index<u32> for OnceVec<T> {
    type Output = T;
    fn index(&self, key : u32) -> &T {
        &Deref::deref(self)[key as usize]
    }
}

impl<T> IndexMut<u32> for OnceVec<T> {
    fn index_mut(&mut self, key : u32) -> &mut T {
        &mut DerefMut::deref_mut(self)[key as usize]
    }
}

impl<T> Deref for OnceVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data.get() }
    }
}

impl<T> DerefMut for OnceVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data.get() }
    }
}

unsafe impl<T : Send> Send for OnceVec<T> {}
unsafe impl<T : Sync> Sync for OnceVec<T> {}

#[derive(Debug)]
pub struct OnceBiVec<T> {
    data : UnsafeCell<BiVec<T>>
}

impl<T> OnceBiVec<T> {
    pub fn to_bivec(self) -> BiVec<T> {
        self.data.into_inner()
    }

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

    pub fn push(&self, x : T) {
        unsafe { (*self.data.get()).push(x); }
    }
}

impl<T : Clone> Clone for OnceBiVec<T> {
    fn clone(&self) -> Self {
        unsafe { OnceBiVec::from_bivec((&*self.data.get()).clone()) }
    }
}
impl<T> Index<i32> for OnceBiVec<T> {
    type Output = T;
    fn index(&self, key : i32) -> &T {
        &Deref::deref(self)[key]
    }
}

impl<T> IndexMut<i32> for OnceBiVec<T> {
    fn index_mut(&mut self, key : i32) -> &mut T {
        &mut DerefMut::deref_mut(self)[key]
    }
}

impl<T> Deref for OnceBiVec<T> {
    type Target = BiVec<T>;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data.get() }
    }
}

impl<T> DerefMut for OnceBiVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data.get() }
    }
}

unsafe impl<T : Send> Send for OnceBiVec<T> {}
unsafe impl<T : Sync> Sync for OnceBiVec<T> {}

use std::io;
use std::io::{Read, Write};
use saveload::{Save, Load};

impl<T : Save> Save for OnceVec<T> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        unsafe { (&*self.data.get()).save(buffer) }
    }
}

impl<T : Save> Save for OnceBiVec<T> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        unsafe { (&*self.data.get()).save(buffer) }
    }
}

impl<T : Load> Load for OnceVec<T> {
    type AuxData = <Vec<T> as Load>::AuxData;

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        Ok(Self {
            data : UnsafeCell::new(Load::load(buffer, data)?)
        })
    }
}

impl<T : Load> Load for OnceBiVec<T> {
    type AuxData = <BiVec<T> as Load>::AuxData;

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        Ok(Self {
            data : UnsafeCell::new(Load::load(buffer, data)?)
        })
    }
}
