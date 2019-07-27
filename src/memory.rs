
use std::slice;
use std::cell::UnsafeCell;
pub const TABLE_PAGE_SIZE: usize = 65536;
pub const STACK_CAPACITY: usize = 65536;
// TODO: Why does it segfault if PAGE_SIZE is set <= 3817???
// pub const TABLE_PAGE_SIZE: usize = 3817;
// pub const STACK_CAPACITY: usize = 3817;

use std::fmt;
use std::mem;
use std::ops::Deref;
use std::ops::DerefMut;

pub struct CVec<T> {
    ptr: *mut T,
    len: usize,
    // We need the backing field so that if we wrap a Vec<> we keep it from being Dropped until we go out of scope.    
    backing : Option<Vec<T>> 
}

impl<T> std::ops::Deref for CVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> std::ops::DerefMut for CVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T> std::ops::Index<usize> for CVec<T> {
    type Output = T;
    fn index(&self, i : usize) -> &Self::Output {
        &self.deref()[i]
    }
}

impl<T> std::ops::IndexMut<usize> for CVec<T> {
    fn index_mut(&mut self, i : usize) -> &mut Self::Output {
        &mut self.deref_mut()[i]
    }
}

impl<T> Drop for CVec<T> {
    fn drop(&mut self) {
        
    }
}

impl<T> fmt::Display for CVec<T>
    where T: std::fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for x in self.iter() {
            write!(f, "{},", x)?;
        }
        Ok(())
    }
}

impl<T> CVec<T> {
    pub fn new(size : usize) -> Self {
        Self::from_vec(Vec::new(size))
    }

    pub fn from_vec(mut vec : Vec<T>) -> Self {
        Self {
            ptr: vec.as_mut_ptr(),
            len: vec.capacity(),
            backing : Some(vec)
        }
    }

    pub fn from_parts(ptr : *mut T, len : usize, backing : Option<Vec<T>>) -> Self {
        Self { 
            ptr,
            len,
            backing
        }
    }

    pub fn to_slice(&self) -> &[T] {
        self.deref()
    }

    pub fn to_slice_mut(&mut self) -> &mut [T] {
        self.deref_mut()
    }

    pub fn get_ptr(&mut self) -> *mut T {
        self.ptr
    }

    pub fn iter(&self) -> std::slice::Iter<T> {
        self.deref().iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<T> {
        self.deref_mut().iter_mut()
    }
    
    pub fn apply_permutation(&mut self, permutation : CVec<usize>, scratch_space : CVec<T>){
        assert!(permutation.len() < self.len());
        assert!(permutation.len() < scratch_space.len());
        unsafe {
            for i in 0..permutation.len(){
                std::ptr::swap(scratch_space.ptr.offset(i as isize), self.ptr.offset(permutation[i] as isize));
            }
            for i in 0..permutation.len(){
                std::ptr::swap(self.ptr.offset(i as isize), scratch_space.ptr.offset(i as isize));
            }            
        }
    }
}

#[derive(Debug)]
pub struct MemoryTable {
    state: UnsafeCell<TableState>,
}

#[derive(Debug)]
struct TableState {
    current_page : Vec<u8>,
    used_pages : Vec<Vec<u8>>
}

#[derive(Debug)]
pub struct MemoryStack {
    state: UnsafeCell<StackState>,
}

#[derive(Debug)]
struct StackState {
    memory: Vec<u8>,
    frames: Vec<usize>,
}


pub trait MemoryAllocator {
    #[inline]
    fn alloc(&self, size : usize, alignment : usize) -> *mut u8;

    fn alloc_vec<T>(&self, length : usize) -> CVec<T> {
        let ptr = self.alloc(length * mem::size_of::<T>(), mem::align_of::<T>());
        CVec {
            ptr: ptr as *mut T,
            len: length as usize,
            backing : None
        }
    }
}

impl MemoryTable {
    pub fn new() -> Self {
        let state = TableState::new();
        Self {
            state: UnsafeCell::new(state)
        }
    }
}

impl MemoryAllocator for MemoryTable {
    #[inline]
    fn alloc(&self, size : usize, alignment : usize) -> *mut u8 {
        unsafe {
            let state = &mut *self.state.get();
            state.alloc(size, alignment) as *mut u8
        }
    }
}

impl TableState {
    fn new() -> TableState {
        Self {
            current_page : Vec::with_capacity(TABLE_PAGE_SIZE),
            used_pages : Vec::new()
        }
    }

    unsafe fn alloc(&mut self, size: usize, alignment: usize) -> *mut u8 {
        assert!(size <= TABLE_PAGE_SIZE);
        let padded_len = (self.current_page.len() + alignment - 1)/alignment * alignment;
        let start_ptr = self.current_page.as_mut_ptr()
            .offset(padded_len as isize);     
        let new_used = padded_len + size;
        // println!("size : {}, padded_len : {}, new_used : {}, capacity : {}", size, padded_len, new_used, self.current_page.capacity());
        // println!("size : {}, padded_len : {}, new_used : {}, capacity : {}", size, padded_len, new_used, self.current_page.capacity());
        if new_used > self.current_page.capacity() {
            println!("new page! leftovers : {}", self.current_page.capacity() - self.current_page.len());
            let mut new_page = Vec::with_capacity(TABLE_PAGE_SIZE);
            new_page.set_len(size);            
            let old_page = mem::replace(&mut self.current_page, new_page);
            self.used_pages.push(old_page);
            return self.current_page.as_mut_ptr() as *mut u8;
        } else {
            self.current_page.set_len(new_used);
            return start_ptr as *mut u8
        }
    }
}


impl fmt::Display for MemoryTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        unsafe {
            let state = &*self.state.get();

            write!(f, "Stack:\n")?;
            write!(f, "    {:p}: size = {}, used = {}\n",
                                state.current_page.as_ptr(),
                                state.current_page.capacity(),
                                state.current_page.len())?;

            // for page in state.used_pages.iter(){
            //     write!(f, "    {:p}: size = {}, used = {}\n",
            //            page.as_ptr(),
            //            page.capacity(),
            //            page.len())?;
            // }

            // for frame in state.frames.iter().rev() {
            //     write!(f, "    {}\n",frame)?;
            // }
            Ok(())
        }
    }
}

impl MemoryStack {
    pub fn with_capacity(initial_capacity: usize) -> Self {
        let state = StackState::new(initial_capacity);
        Self {
            state: UnsafeCell::new(state)
        }
    }

    pub fn new() -> Self {
        Self::with_capacity(STACK_CAPACITY-1)
    }

    pub fn push_stack_frame(&mut self) {
        unsafe {
            let state = &mut *self.state.get();
            state.push_stack_frame();
        }
    }

    pub fn pop_stack_frame(&mut self) {
        unsafe {
            let state = &mut *self.state.get();
            state.pop_stack_frame();
        }
    }
}

impl MemoryAllocator for MemoryStack {
    #[inline]
    fn alloc(&self, size : usize, alignment : usize) -> *mut u8 {
        unsafe {
            let state = &mut *self.state.get();
            state.alloc(size, alignment) as *mut u8
        }
    }
}   

impl StackState {
    fn new(capacity: usize) -> Self {
        Self {
           memory: Vec::with_capacity(capacity),
           frames: Vec::new(),
        }
    }

    fn push_stack_frame(&mut self) {
        self.frames.push(self.memory.len());
    }

    unsafe fn pop_stack_frame(&mut self) {
        if let Some(x) = self.frames.pop() {
            self.memory.set_len(x);
        } else {
            println!("Stack popped but stack frame empty.");
            assert!(false);
        }
    }

    unsafe fn alloc(&mut self, size: usize, alignment: usize) -> *mut u8 {
        let padded_len = (self.memory.len() + alignment - 1)/alignment * alignment;
        {
            let end_of_table = self.memory.as_mut_ptr()
                .offset((self.memory.capacity()) as isize);  
            println!("end_of_table  : {:p}", end_of_table);
        }
        let start_ptr = self.memory.as_mut_ptr()
            .offset(padded_len as isize);   
        println!("start_ptr     : {:p} padded_len: {}", start_ptr, padded_len);
        println!("end_ptr       : {:p}", start_ptr.offset(size as isize));
        let new_used = padded_len + size;
        if new_used > self.memory.capacity() {
            // An error
            assert!(false);
        }
        self.memory.set_len(new_used);
        start_ptr as *mut u8
    }
}

impl fmt::Display for MemoryStack {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        unsafe {
            let state = &*self.state.get();

            write!(f, "Stack:\n")?;

            write!(f, "    {:p}: size = {}, used = {}\n",
                   state.memory.as_ptr(),
                   state.memory.capacity(),
                   state.memory.len())?;

            // for frame in state.frames.iter().rev() {
            //     write!(f, "    {}\n",frame)?;
            // }
            Ok(())
        }
    }
}