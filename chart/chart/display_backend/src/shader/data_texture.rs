#[allow(unused_imports)]
use crate::log;
use crate::shader::range::Range;
use crate::shader::attributes::{Format, Type};
use web_sys::{WebGl2RenderingContext, WebGlTexture};
use wasm_bindgen::{JsValue};
use js_sys::Object;
use crate::webgl_wrapper::WebGlWrapper;


// TODO: context loss?
pub struct DataTexture<T> {
    webgl : WebGlWrapper,
    width : usize,
    format : Format,
    pub data : Vec<T>, 
    used_entries : usize, 
    texture : Option<WebGlTexture>,
    texture_rows : usize,
    pub dirty_range : Range,
    marker : std::marker::PhantomData<T>
}

impl<T : std::fmt::Debug + std::default::Default> DataTexture<T> {
    pub fn new(webgl : WebGlWrapper, format : Format) -> Self {
        assert_eq!(std::mem::align_of::<T>() % format.0.alignment(), 0);
        Self {
            webgl,
            width : 2048, 
            format,
            data : Vec::new(),
            used_entries : 0,
            texture : None,
            texture_rows : 0,
            dirty_range : Range::empty(),
            marker : std::marker::PhantomData
        }
    }

    fn row_bytes(&self) -> usize {
        self.width * (self.format.size() as usize)
    }

    fn num_full_rows(&mut self) -> usize {
        let total_entries = self.used_entries;
        let total_bytes = total_entries * std::mem::size_of::<T>();
        total_bytes / self.row_bytes()
    }


    fn num_rows(&mut self) -> usize {
        self.num_rows_to_fit_extra_data(0)
    }

    fn num_rows_to_fit_extra_data(&self, n : usize) -> usize {
        let total_entries = self.used_entries + n;
        let total_bytes = total_entries * std::mem::size_of::<T>();
        ( total_bytes + self.row_bytes() - 1) / self.row_bytes()
    }

    fn num_entries_to_fit_rows(&self, num_rows : usize) -> usize {
        (num_rows * self.row_bytes() + std::mem::size_of::<T>() - 1) / std::mem::size_of::<T>()
    }

    // TODO: use capacity of vector to determine new texture size.
    fn ensure_size(&mut self){
        let num_rows = self.num_rows();
        if num_rows <= self.texture_rows {
            return;
        }
        self.texture_rows = num_rows;
        self.webgl.delete_texture(self.texture.as_ref());
        self.texture = self.webgl.inner.create_texture();
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.texture.as_ref());
        self.webgl.tex_storage_2d(
            WebGl2RenderingContext::TEXTURE_2D,
            1, // mip levels
            self.format.internal_format(),
            self.width as i32, num_rows as i32
        );
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MAG_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_MIN_FILTER, WebGl2RenderingContext::NEAREST as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_S, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        self.webgl.tex_parameteri(WebGl2RenderingContext::TEXTURE_2D, WebGl2RenderingContext::TEXTURE_WRAP_T, WebGl2RenderingContext::CLAMP_TO_EDGE as i32);
        self.dirty_range = Range::new(0, num_rows);
    }

    pub fn len(&self) -> usize {
        self.used_entries
    }

    pub fn clear(&mut self){
        self.used_entries = 0;
    }

    pub fn data(&self) -> &[T] {
        &self.data[0..self.used_entries]
    }

    pub fn append<It : ExactSizeIterator<Item = T>>(&mut self, data : It) {
        let data_len = data.len();
        let start_row = self.num_full_rows();
        // First make sure we have enough room. We have to have enough memory to fill a rectangular texture.
        // The size of T is not necessarily evenly divisible into rows, we just make sure that we have more than enough T's to fill out the rectangle.
        let total_rows_needed = self.num_rows_to_fit_extra_data(data_len);
        if total_rows_needed > self.num_rows() {
            self.data.resize_with(self.num_entries_to_fit_rows(total_rows_needed), || T::default());
        }
        // splice replaces a range with the result of iterator. 
        // splice returns an iterator over subbed out range, which we have to consume in order to ensure change takes place.
        self.data.splice(self.used_entries .. self.used_entries + data_len, data).for_each(drop);  // consume iterator, ignore values.
        self.used_entries += data_len;
        let end_row = self.num_rows();
        self.dirty_range.include_range(Range::new(start_row, end_row));
    }

    pub fn push(&mut self, entry : T){
        self.append(std::iter::once(entry));
    }

    unsafe fn data_view(&self, min_row : usize, max_row : usize) -> Object {
        // For float textures we are REQUIRED to provide Float32Array views.
        let data_bytes = std::mem::size_of_val(self.data.as_slice());
        let data_ptr = self.data.as_ptr() as *const u8;
        let data_u8_slice = std::slice::from_raw_parts(data_ptr, data_bytes);
        let data_u8_slice = &data_u8_slice[min_row * self.row_bytes() .. max_row * self.row_bytes()];
        match self.format.0 {
            Type::F32 => js_sys::Float32Array::view_mut_raw(data_u8_slice.as_ptr() as *mut f32, data_u8_slice.len() / std::mem::size_of::<f32>()).into(),
            Type::I16 | Type::U16 | Type::U8 | Type::U32
                => js_sys::Uint8Array::view(data_u8_slice).into(),
        }
    }

    fn prepare(&mut self) -> Result<(), JsValue> {
        self.ensure_size();
        if self.dirty_range.is_empty() {
            return Ok(());
        }
        let num_rows = self.num_rows();
        let dirty_min = self.dirty_range.min;
        let dirty_max = self.dirty_range.max.min(num_rows);
        let yoffset = dirty_min as i32;
        let data_view = unsafe {
            self.data_view(dirty_min, dirty_max)
        };
        self.webgl.tex_sub_image_2d_with_i32_and_i32_and_u32_and_type_and_opt_array_buffer_view(
            WebGl2RenderingContext::TEXTURE_2D, 
            0, // mip level
            0, yoffset, // xoffset, yoffset: i32,
            self.width as i32, (dirty_max - dirty_min) as i32, // width, height
            self.format.base_format(), // format: u32,
            self.format.webgl_type(), // type_: u32,
            Some(&data_view) // pixels: Option<&[u8]>
        )?;
        self.dirty_range = Range::empty();
        Ok(())
    }

    pub fn bind(&mut self, texture_unit : u32) -> Result<(), JsValue> {
        self.webgl.active_texture(WebGl2RenderingContext::TEXTURE0 + texture_unit);
        self.webgl.bind_texture(WebGl2RenderingContext::TEXTURE_2D, self.texture.as_ref());
        self.prepare()?;
        Ok(())
    }
}
