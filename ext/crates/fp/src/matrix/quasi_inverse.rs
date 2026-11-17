use std::io;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};

use super::Matrix;
use crate::{
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut},
};

/// Given a matrix M, a quasi-inverse Q is a map from the co-domain to the domain such that xQM = x
/// for all x in the image (recall our matrices act on the right).
///
/// # Fields
///  * `image` - The image of the original matrix. If the image is omitted, it is assumed to be
///    everything (with the standard basis).
///  * `preimage` - The actual quasi-inverse, where the basis of the image is that given by
///    `image`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn to_bytes(&self, buffer: &mut impl io::Write) -> io::Result<()> {
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
                Matrix::write_pivot(v, buffer)?;
            }
        }
        self.preimage.to_bytes(buffer)
    }

    pub fn from_bytes(p: ValidPrime, data: &mut impl io::Read) -> io::Result<Self> {
        let source_dim = data.read_u64::<LittleEndian>()? as usize;
        let target_dim = data.read_u64::<LittleEndian>()? as usize;
        let image_dim = data.read_u64::<LittleEndian>()? as usize;

        let image = Matrix::read_pivot(target_dim, data)?;
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
    pub fn apply(&self, mut target: FpSliceMut, coeff: u32, input: FpSlice) {
        let p = self.prime();
        let mut row = 0;
        for (i, c) in input.iter().enumerate() {
            if let Some(pivots) = self.pivots()
                && (i >= pivots.len() || pivots[i] < 0)
            {
                continue;
            }
            if c != 0 {
                target.add(self.preimage.row(row), (coeff * c) % p);
            }
            row += 1;
        }
    }
}
