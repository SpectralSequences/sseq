use crate::shader::range::Range;
use crate::webgl_wrapper::WebGlWrapper;


use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;
use web_sys::{WebGlBuffer, WebGl2RenderingContext};


pub struct VertexBuffer<T> {
    webgl : WebGlWrapper,
    pub buffer : Option<WebGlBuffer>,
    buffer_capacity : usize,
    pub data : Vec<T>,
    dirty_range : Range,
}

impl<T> VertexBuffer<T> {
    pub fn new(webgl : WebGlWrapper) -> Self {
        Self::with_capacity(webgl, 0)
    }

    pub fn with_capacity(webgl : WebGlWrapper, capacity: usize) -> Self {
        let buffer = webgl.create_buffer();
        Self {
            webgl,
            buffer,
            buffer_capacity : 0,
            data : Vec::with_capacity(capacity),
            dirty_range : Range::empty(),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn clear(&mut self){
        self.data.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn push(&mut self, value: T) {
        self.dirty_range.include_int(self.data.len());
        self.data.push(value);
    }

    fn ensure_buffer_size(&mut self) {
        if self.data.len() <= self.buffer_capacity {
            return;
        }
        // reserve size for buffer.
        let buffer_size = (self.data.capacity() * std::mem::size_of::<T>()) as i32;
        self.webgl.buffer_data_with_i32(WebGl2RenderingContext::ARRAY_BUFFER, buffer_size, WebGl2RenderingContext::STATIC_DRAW);
        self.dirty_range = Range::new(0, self.data.len());
    }

    fn update_buffer_data(&mut self){
        if self.dirty_range.is_empty() {
            return;
        }
        let dirty_min = self.dirty_range.min;
        let dirty_max = self.dirty_range.max.min(self.data.len());
        let offset = std::mem::size_of_val(&self.data[0..dirty_min]) as i32;
        let slice = &self.data[dirty_min .. dirty_max];
        let slice_size = std::mem::size_of_val(slice);
        let u8_ptr = self.data.as_ptr() as *mut u8;
        let u8_slice = unsafe {
            std::slice::from_raw_parts(u8_ptr, slice_size)
        };
        self.webgl.buffer_sub_data_with_i32_and_u8_array(WebGl2RenderingContext::ARRAY_BUFFER, offset, u8_slice);
        self.dirty_range = Range::empty();
    }


    pub fn prepare(&mut self) {
        if self.data.len() == 0 {
            return;
        }
        self.webgl.bind_buffer(WebGl2RenderingContext::ARRAY_BUFFER, self.buffer.as_ref());
        self.ensure_buffer_size();
        self.update_buffer_data();
    }

}

impl<T, I: SliceIndex<[T]>> Index<I> for VertexBuffer<T> {
    type Output = I::Output;
    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.data[index]
    }
}

impl<T, I: SliceIndex<[T]> + Copy + Into<Range> > IndexMut<I> for VertexBuffer<T> {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.dirty_range.include_range(index.into());
        &mut self.data[index]
    }
}
