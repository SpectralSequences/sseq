use super::Matrix;
use crate::prime::ValidPrime;
use crate::vector::{Slice, SliceMut};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};

/// Given a matrix M, a quasi-inverse Q is a map from the co-domain to the domain such that xQM = x
/// for all x in the image (recall our matrices act on the right).
///
/// # Fields
///  * `image` - The image of the original matrix. If the image is omitted, it is assumed to be
///  everything (with the standard basis).
///  * `preimage` - The actual quasi-inverse, where the basis of the image is that given by
///  `image`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuasiInverse {
    image: Option<Vec<isize>>,
    preimage: Matrix,
}

impl QuasiInverse {
    pub fn new(image: Option<Vec<isize>>, preimage: Matrix) -> Self {
        Self { image, preimage }
    }

    pub fn image_dimension(&self) -> usize {
        self.preimage.rows()
    }

    pub fn source_dimension(&self) -> usize {
        self.preimage.columns()
    }

    pub fn target_dimension(&self) -> usize {
        match self.image.as_ref() {
            Some(v) => v.len(),
            None => self.image_dimension(),
        }
    }

    pub fn to_bytes(&self, buffer: &mut impl Write) -> std::io::Result<()> {
        buffer.write_u64::<LittleEndian>(self.source_dimension() as u64)?;
        buffer.write_u64::<LittleEndian>(self.target_dimension() as u64)?;
        buffer.write_u64::<LittleEndian>(self.image_dimension() as u64)?;

        match self.image.as_ref() {
            None => {
                for i in 0..self.preimage.rows() {
                    buffer.write_i64::<LittleEndian>(i as i64)?;
                }
            }
            Some(v) => {
                for &i in v {
                    buffer.write_i64::<LittleEndian>(i as i64)?;
                }
            }
        }
        self.preimage.to_bytes(buffer)
    }

    pub fn from_bytes(p: ValidPrime, data: &mut impl Read) -> std::io::Result<Self> {
        let source_dim = data.read_u64::<LittleEndian>()? as usize;
        let target_dim = data.read_u64::<LittleEndian>()? as usize;
        let image_dim = data.read_u64::<LittleEndian>()? as usize;
        let mut image = Vec::with_capacity(target_dim);
        for _ in 0..target_dim {
            image.push(data.read_i64::<LittleEndian>()? as isize);
        }
        let preimage = Matrix::from_bytes(p, image_dim, source_dim, data)?;
        Ok(Self {
            image: Some(image),
            preimage,
        })
    }

    pub fn preimage(&self) -> &Matrix {
        &self.preimage
    }

    pub fn pivots(&self) -> Option<&[isize]> {
        self.image.as_deref()
    }

    pub fn prime(&self) -> ValidPrime {
        self.preimage.prime()
    }

    /// Apply the quasi-inverse to an input vector and add a constant multiple of the result
    /// to an output vector
    ///
    /// # Arguments
    ///  * `target` - The output vector
    ///  * `coeff` - The constant multiple above
    ///  * `input` - The input vector, expressed in the basis of the ambient space
    pub fn apply(&self, mut target: SliceMut, coeff: u32, input: Slice) {
        let p = self.prime();
        let mut row = 0;
        for (i, c) in input.iter().enumerate() {
            if let Some(pivots) = self.pivots() {
                if i >= pivots.len() || pivots[i] < 0 {
                    continue;
                }
            }
            if c != 0 {
                target.add(self.preimage[row].as_slice(), (coeff * c) % *p);
            }
            row += 1;
        }
    }
}
