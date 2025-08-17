use crate::limb::Limb;

// Mutability trait for zero-cost generic block views
pub trait Mutability {
    type Pointer<T>: Copy;
}

#[derive(Clone, Copy)]
pub struct Immutable;
pub struct Mutable;

impl Mutability for Immutable {
    type Pointer<T> = *const T;
}

impl Mutability for Mutable {
    type Pointer<T> = *mut T;
}

#[repr(align(128))]
#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub limbs: [Limb; 64],
}

pub struct BlockView<'a, M: Mutability> {
    pub limbs: M::Pointer<Limb>,
    pub coords: [usize; 2],
    pub stride: usize,
    pub _marker: std::marker::PhantomData<&'a ()>,
}

// Type aliases for convenience
pub type MatrixBlockSlice<'a> = BlockView<'a, Immutable>;
pub type MatrixBlockSliceMut<'a> = BlockView<'a, Mutable>;

impl<'a, M: Mutability> BlockView<'a, M> {
    pub fn copy<'b>(&'b mut self) -> BlockView<'b, M> {
        BlockView {
            limbs: self.limbs,
            coords: self.coords,
            stride: self.stride,
            _marker: std::marker::PhantomData,
        }
    }
}

// SIMD strategy trait for pluggable block gathering
pub trait GatherStrategy {
    fn gather_block(slice: MatrixBlockSlice) -> Block;
}

pub struct Avx512Strategy;
pub struct AvxStrategy;
pub struct ScalarStrategy;

impl GatherStrategy for Avx512Strategy {
    fn gather_block(slice: MatrixBlockSlice) -> Block {
        super::avx512::gather_block_avx512(slice).as_matrix_block()
    }
}

impl GatherStrategy for AvxStrategy {
    fn gather_block(slice: MatrixBlockSlice) -> Block {
        // super::avx::gather_block_avx(slice)
        super::scalar::gather_block_scalar(slice) // Fallback until AVX is implemented
    }
}

impl GatherStrategy for ScalarStrategy {
    fn gather_block(slice: MatrixBlockSlice) -> Block {
        super::scalar::gather_block_scalar(slice)
    }
}

impl<'a> MatrixBlockSlice<'a> {
    pub fn iter(self) -> impl Iterator<Item = &'a Limb> {
        (0..64).map(move |i| unsafe { &*self.limbs.add(i * self.stride) })
    }

    pub fn gather_block(self) -> Block {
        Self::gather_with_strategy::<AutoStrategy>(self)
    }

    pub fn gather_with_strategy<S: GatherStrategy>(self) -> Block {
        S::gather_block(self)
    }
}

// Auto-detection strategy
pub struct AutoStrategy;

impl GatherStrategy for AutoStrategy {
    fn gather_block(slice: MatrixBlockSlice) -> Block {
        if is_x86_feature_detected!("avx512f") {
            Avx512Strategy::gather_block(slice)
        // } else if is_x86_feature_detected!("avx") {
        //     AvxStrategy::gather_block(slice)
        } else {
            ScalarStrategy::gather_block(slice)
        }
    }
}

impl<'a> MatrixBlockSliceMut<'a> {
    pub fn get_mut(&mut self, row: usize) -> &mut Limb {
        unsafe { &mut *(self.limbs.add(row * self.stride) as *mut _) }
    }

    pub fn iter_mut<'b>(&'b mut self) -> impl Iterator<Item = &'b mut Limb> + use<'a, 'b> {
        (0..64).map(move |i| unsafe { &mut *(self.limbs.add(i * self.stride) as *mut _) })
    }

    pub fn as_slice(&self) -> MatrixBlockSlice<'_> {
        BlockView {
            limbs: self.limbs,
            coords: self.coords,
            stride: self.stride,
            _marker: std::marker::PhantomData,
        }
    }
}

unsafe impl<'a, M: Mutability> Send for BlockView<'a, M> {}

unsafe impl<'a> Sync for BlockView<'a, Immutable> {}
