use super::Matrix;
use crate::prime::ValidPrime;
use crate::vector::{Slice, SliceMut};

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

use saveload::{Load, Save};
use std::io;
use std::io::{Read, Write};

impl Save for QuasiInverse {
    fn save(&self, buffer: &mut impl Write) -> io::Result<()> {
        self.image.save(buffer)?;
        self.preimage.save(buffer)?;
        Ok(())
    }
}

impl Load for QuasiInverse {
    type AuxData = ValidPrime;

    fn load(buffer: &mut impl Read, p: &ValidPrime) -> io::Result<Self> {
        let image = Option::<super::Subspace>::load(buffer, &Some(*p))?;

        Ok(Self {
            image: image.map(|mut x| x.take_pivots()),
            preimage: Matrix::load(buffer, p)?,
        })
    }
}
