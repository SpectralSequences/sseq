use core::cell::UnsafeCell;
use core::ops::{Deref,  Index, DerefMut, IndexMut};
use std::fmt;
// use bivec::BiVec;
use std::cmp::{PartialEq, Eq};
// use std::slice::{Iter};

static DEFAULT_CAPACITY : usize = 1028 * PAGE_SIZE;
static PAGE_SIZE : usize = 1028;

pub struct OnceVec<T> {
    data : UnsafeCell<Vec<Vec<T>>>
}

pub struct OnceVecIter<'a, T> {
    vector : &'a OnceVec<T>,
    idx : usize
}

impl<'a, T> Iterator for OnceVecIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        if self.idx == self.vector.len() {
            None
        } else {
            let result = &self.vector[self.idx];
            self.idx += 1;
            Some(result)
        }
    }
}

impl<T> Default for OnceVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: fmt::Debug> fmt::Debug for OnceVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        let mut it = self.iter();
        match it.next() {
            Some(x) => write!(f, "{:?}", x)?,
            None => {
                return write!(f, "]");
            }
        }
        for x in it {
            write!(f, ", {:?}", x)?;
        }
        write!(f, "]")
    }
}

impl<T> PartialEq for OnceVec<T>
    where T : PartialEq {
    fn eq(&self, other: &OnceVec<T>) -> bool {
        if self.len() != other.len() { // || self.capacity() != other.capacity() {
            return false;
        }
        for i in 0..self.len() {
            if self[i] != other[i] {
                return false;
            }
        }
        return true;
    }
}

impl<T> Eq for OnceVec<T> where T : Eq {}

impl<T>  OnceVec<T> {
    // pub fn into_vec(self) -> Vec<T> {
    //     self.data.into_inner()
    // }

    // pub fn from_vec(vec : Vec<T>) -> Self {
    //     Self {
    //         data : UnsafeCell::new(vec)
    //     }
    // }

    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity : usize) -> Self {
        let vec = Vec::with_capacity((capacity + PAGE_SIZE - 1)/PAGE_SIZE);
        Self {
            data : UnsafeCell::new(vec)
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn get_outer_vec_mut(&self) -> &mut Vec<Vec<T>> {
        unsafe { &mut *self.data.get() }
    }

    pub fn len(&self) -> usize {
        let outer_len = Deref::deref(self).len();
        if outer_len == 0 {
            0
        } else {
            (outer_len - 1) * PAGE_SIZE + Deref::deref(self)[outer_len - 1].len()
        }
    }

    pub fn get(&self, idx : usize) -> Option<&T> {
        if idx < self.len() {
            Some(&self[idx])
        } else {
            None
        }
    }

    pub fn last(&self) -> Option<&T> {
        if self.len() > 0 {
            Some(&self[self.len() - 1])
        } else {
            None
        }
    }

    pub fn capacity(&self) -> usize {
        Deref::deref(self).capacity() * PAGE_SIZE
    }

    pub fn reserve(&self, additional : usize) {
        assert!(self.len() + additional <= self.capacity(), "Not enough space to reserve!");
    }

    pub fn reserve_exact(&self, additional : usize) {
        assert!(self.len() + additional <= self.capacity(), "Not enough space to reserve!");
    }

    pub fn push(&self, x : T) {
        let outer_vec = self.get_outer_vec_mut();
        if outer_vec.len() == 0 {
            outer_vec.push(Vec::with_capacity(PAGE_SIZE));
        }
        let mut outer_vec_len = outer_vec.len();
        let mut inner_vec = &mut outer_vec[outer_vec_len - 1];
        if inner_vec.len() == inner_vec.capacity() {
            if outer_vec.len() == outer_vec.capacity() {
                panic!("Out of space!");
            }
            outer_vec.push(Vec::with_capacity(PAGE_SIZE));
            outer_vec_len += 1;
            inner_vec = &mut outer_vec[outer_vec_len - 1];
        } 
        inner_vec.push(x);
    }

    pub fn iter(&self) -> OnceVecIter<T> {
        OnceVecIter {
            vector : &self,
            idx : 0
        }
    }
}

impl<T> Index<usize> for OnceVec<T> {
    type Output = T;
    fn index(&self, key : usize) -> &T {
        // let (page, page_idx) = key.div_rem(PAGE_SIZE);
        let page = key / PAGE_SIZE;
        let page_idx = key % PAGE_SIZE;
        &Deref::deref(self)[page][page_idx]
    }
}

impl<T> IndexMut<usize> for OnceVec<T> {
    fn index_mut(&mut self, key : usize) -> &mut T {
        let page = key / PAGE_SIZE;
        let page_idx = key % PAGE_SIZE;        
        &mut DerefMut::deref_mut(self)[page][page_idx]
    }
}

impl<T> Index<u32> for OnceVec<T> {
    type Output = T;
    fn index(&self, key : u32) -> &T {
        let key = key as usize;
        // let (page, page_idx) = key.div_rem(PAGE_SIZE);
        let page = key / PAGE_SIZE;
        let page_idx = key % PAGE_SIZE;
        &Deref::deref(self)[page][page_idx]
    }
}

impl<T> IndexMut<u32> for OnceVec<T> {
    fn index_mut(&mut self, key : u32) -> &mut T {
        let key = key as usize;
        let page = key / PAGE_SIZE;
        let page_idx = key % PAGE_SIZE;        
        &mut DerefMut::deref_mut(self)[page][page_idx]
    }
}

impl<T> Deref for OnceVec<T> {
    type Target = Vec<Vec<T>>;

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

use std::io;
use std::io::{Read, Write};
use saveload::{Save, Load};

impl<T : Save> Save for OnceVec<T> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.len().save(buffer)?;
        for x in self.iter() {
            x.save(buffer)?;
        }
        Ok(())
    }
}

impl<T : Load> Load for OnceVec<T> {
    type AuxData = T::AuxData;

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        let len = usize::load(buffer, &())?;
        let result : OnceVec<T> = OnceVec::with_capacity(2*len);
        for _ in 0 .. len {
            result.push(T::load(buffer, data)?);
        }
        Ok(result)
    }
}

#[derive(PartialEq, Eq)] // Clone?
pub struct OnceBiVec<T> {
    pub data : OnceVec<T>,
    min_degree : i32
}

impl<T: fmt::Debug> fmt::Debug for OnceBiVec<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "BiVec({}) ", self.min_degree)?;
        self.data.fmt(formatter)
    }
}

impl<T> OnceBiVec<T> {
    pub fn new(min_degree : i32) -> Self {
        OnceBiVec {
            data : OnceVec::new(),
            min_degree
        }
    }

    // pub fn from_vec(min_degree : i32, data : Vec<T>) -> Self {
    //     Self {
    //         data,
    //         min_degree
    //     }
    // }

    pub fn with_capacity(min_degree : i32, capacity : i32) -> Self {
        debug_assert!(capacity >= min_degree);
        Self {
            data : OnceVec::with_capacity((capacity - min_degree) as usize),
            min_degree
        }
    }

    pub fn min_degree(&self) -> i32 {
        self.min_degree
    }

    /// This returns the largest degree in the bivector. This is equal to `self.len() - 1`.
    ///
    /// # Example
    /// ```
    /// # use bivec::BiVec;
    /// let v = BiVec::from_vec(-2, vec![3, 4, 6, 8, 2]);
    /// assert_eq!(v.max_degree(), 2);
    /// ```
    pub fn max_degree(&self) -> i32 {
        self.len() - 1
    }

    /// This returns the "length" of the bivector, defined to be the smallest i such that
    /// `v[i]`
    /// is not defined.
    ///
    /// # Example
    /// ```
    /// # use bivec::BiVec;
    /// let v = BiVec::from_vec(-2, vec![3, 4, 6, 8, 2]);
    /// assert_eq!(v.len(), 3);
    /// ```
    pub fn len(&self) -> i32 {
        self.data.len() as i32 + self.min_degree
    }

    pub fn is_empty(&self) -> bool {
        self.data.len() == 0
    }

    pub fn push(&self, x : T) {
        self.data.push(x);
    }

    pub fn get(&self, idx : i32) -> Option<&T> {
        self.data.get((idx - self.min_degree) as usize)
    }

    pub fn last(&self) -> Option<&T> {
        self.data.last()
    }
    pub fn iter(&self) -> OnceVecIter<T> {
        self.data.iter()
    }

    pub fn iter_enum(&self) -> impl Iterator<Item = (i32, &T)> {
        let min_degree = self.min_degree;
        self.data.iter().enumerate()
            .map(move |(i, t)| (i as i32 + min_degree, t))
            // .collect()
    }
}

// impl<T : Serialize> Serialize for OnceBiVec<T> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//         where S : Serializer
//     {
//         self.data.serialize(serializer) // Do better than this
//     }
// }

// impl<'de, T : Deserialize<'de>> Deserialize<'de> for OnceBiVec<T> {
//     fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
//         where D : Deserializer<'de>
//     {
//         unimplemented!()
//     }
// }

impl<T> Index<i32> for OnceBiVec<T> {
    type Output = T;
    fn index(&self, i : i32) -> &T {
        &(self.data[(i - self.min_degree) as usize])
    }
}

impl<T> IndexMut<i32> for OnceBiVec<T> {
    fn index_mut(&mut self, i : i32) -> &mut T {
        &mut (self.data[(i - self.min_degree) as usize])
    }
}


impl<T : Save> Save for OnceBiVec<T> {
    fn save(&self, buffer : &mut impl Write) -> io::Result<()> {
        self.data.save(buffer)
    }
}

impl<T : Load> Load for OnceBiVec<T> {
    type AuxData = (i32, T::AuxData);

    fn load(buffer : &mut impl Read, data : &Self::AuxData) -> io::Result<Self> {
        let min_degree = data.0;
        let data = Load::load(buffer, &data.1)?;
        Ok(Self { data, min_degree })
    }
}




#[cfg(test)]
mod tests {
    use super::*;
    // use rstest::rstest_parametrize;

    #[test]
    fn test_saveload(){
        use saveload::{Save, Load};
        use std::io::{Read, Cursor, SeekFrom, Seek, Error};

        let v : OnceVec<u32> = OnceVec::new();
        v.push(6);
        v.push(3);
        v.push(4);
        v.push(2);



        let mut cursor : Cursor<Vec<u8>> = Cursor::new(Vec::new());
        v.save(&mut cursor).unwrap();

        cursor.seek(SeekFrom::Start(0)).unwrap();
        let v_saved_then_loaded : OnceVec<u32> = Load::load(&mut cursor, &()).unwrap();
        assert_eq!(v, v_saved_then_loaded);
        assert_eq!(0, cursor.bytes().count());

        // let mut w = BiVec::new(-3);
        // w.push(2);
        // w.push(6);
        // w.push(2);
        // w.push(7);

        // let mut cursor2 : Cursor<Vec<u8>> = Cursor::new(Vec::new());
        // w.save(&mut cursor2).unwrap();
        // cursor2.seek(SeekFrom::Start(0)).unwrap();
        // let w_saved_then_loaded : BiVec<u32> = Load::load(&mut cursor, &(-3, ())).unwrap();        
        
        // assert_eq!(w, w_saved_then_loaded);
    }

    #[test]
    fn test_segv(){
        let v = OnceVec::with_capacity(1028*1028 + 5);
        v.push(vec![0]);
        let firstvec : &Vec<i32> = &v[0usize];
        println!("firstvec[0] : {} firstvec_addr: {:p}", firstvec[0], firstvec as *const Vec<i32>);
        let mut address : *const Vec<i32> = &v[0usize];
        for i in 0 .. 1028*1028 + 1 {
            if address != &v[0usize] {
                address = &v[0usize];
                println!("moved. i: {}. New address: {:p}", i, address);
            }
            v.push(vec![i]);
        }
        println!("old_addr   : {:p}", firstvec as *const Vec<i32>);
        println!("actual_addr: {:p}", &v[0usize] as *const Vec<i32>);

        println!("Next line segfaults:");
        println!("{}", firstvec[0]);
    }
}