use super::Matrix;
use crate::prime::ValidPrime;
use crate::vector::{Slice, SliceMut};
use saveload::filebacked::{FileBacked, FileBackedGuard};

/// Given a matrix M, a quasi-inverse Q is a map from the co-domain to the domain such that xQM = x
/// for all x in the image (recall our matrices act on the right).
///
/// # Fields
///  * `image` - The image of the original matrix. If the image is omitted, it is assumed to be
///  everything (with the standard basis).
///  * `preimage` - The actual quasi-inverse, where the basis of the image is that given by
///  `image`.
#[derive(Clone)]
pub struct QuasiInverse {
    prime: ValidPrime,
    image: Option<Vec<isize>>,
    preimage: FileBacked<Matrix>,
}

impl QuasiInverse {
    pub fn new(image: Option<Vec<isize>>, preimage: Matrix) -> Self {
        let prime = preimage.prime();
        Self {
            prime,
            image,
            preimage: FileBacked::new(preimage, &prime),
        }
    }

    pub fn preimage(&self) -> FileBackedGuard<Matrix> {
        self.preimage.upgrade()
    }

    pub fn pivots(&self) -> Option<&[isize]> {
        self.image.as_deref()
    }

    pub fn prime(&self) -> ValidPrime {
        self.prime
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
        let preimage = self.preimage();
        for (i, c) in input.iter().enumerate() {
            if let Some(pivots) = self.pivots() {
                if i >= pivots.len() || pivots[i] < 0 {
                    continue;
                }
            }
            if c != 0 {
                target.add(preimage[row].as_slice(), (coeff * c) % *p);
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
        Ok(Self {
            prime: *p,
            image: Option::<Vec<isize>>::load(buffer, &Some(()))?,
            preimage: FileBacked::load(buffer, p)?,
        })
    }
}
