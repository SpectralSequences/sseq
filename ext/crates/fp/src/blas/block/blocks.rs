use crate::limb::Limb;

#[repr(align(128))]
#[derive(Debug, Clone, Copy)]
pub struct MatrixBlock {
    pub limbs: [Limb; 64],
}

pub struct MatrixBlockSlice<'a> {
    pub limbs: *const Limb,
    pub coords: [usize; 2],
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

pub struct MatrixBlockSliceMut<'a> {
    pub limbs: *mut Limb,
    pub coords: [usize; 2],
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a mut ()>,
}

impl<'a> MatrixBlockSlice<'a> {
    pub fn iter(self) -> impl Iterator<Item = &'a Limb> {
        (0..64).map(move |i| unsafe { &*self.limbs.add(i * self.stride) })
    }

    pub fn gather(self) -> MatrixBlock {
        if is_x86_feature_detected!("avx512f") {
            super::avx512::gather_block_avx512(self).as_matrix_block()
        // } else if is_x86_feature_detected!("avx") {
        //     super::avx::gather_block_avx(self)
        } else {
            super::scalar::gather_block_scalar(self)
        }
    }
}

impl<'a> MatrixBlockSliceMut<'a> {
    pub fn get_mut(&mut self, row: usize) -> &mut Limb {
        unsafe { &mut *self.limbs.add(row * self.stride) }
    }

    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = &'b mut Limb> + use<'a, 'b> {
        (0..64).map(move |i| unsafe { &mut *self.limbs.add(i * self.stride) })
    }

    pub fn copy(&mut self) -> MatrixBlockSliceMut<'_> {
        MatrixBlockSliceMut {
            limbs: self.limbs,
            coords: self.coords,
            stride: self.stride,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn as_slice(&self) -> MatrixBlockSlice<'_> {
        MatrixBlockSlice {
            limbs: self.limbs,
            coords: self.coords,
            stride: self.stride,
            _marker: std::marker::PhantomData,
        }
    }
}

unsafe impl<'a> Send for MatrixBlockSlice<'a> {}
unsafe impl<'a> Send for MatrixBlockSliceMut<'a> {}

unsafe impl<'a> Sync for MatrixBlockSlice<'a> {}
