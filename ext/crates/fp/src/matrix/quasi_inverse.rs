use std::io;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use itertools::Itertools;

use super::Matrix;
use crate::{
    prime::ValidPrime,
    vector::{FpSlice, FpSliceMut, FpVector},
};

/// Given a matrix M, a quasi-inverse Q is a map from the co-domain to the domain such that xQM = x
/// for all x in the image (recall our matrices act on the right).
///
/// # Fields
///  * `image` - The image of the original matrix. If the image is omitted, it is assumed to be
///    everything (with the standard basis).
///  * `preimage` - The actual quasi-inverse, where the basis of the image is that given by
///    `image`.
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

    /// Given a data file containing a quasi-inverse, apply it to all the vectors in `input`
    /// and write the results to the corresponding vectors in `results`. This reads in the
    /// quasi-inverse row by row to minimize memory usage.
    pub fn stream_quasi_inverse<T, S>(
        p: ValidPrime,
        data: &mut impl io::Read,
        results: &mut [T],
        inputs: &[S],
    ) -> io::Result<()>
    where
        for<'a> &'a mut T: Into<FpSliceMut<'a>>,
        for<'a> &'a S: Into<FpSlice<'a>>,
    {
        let source_dim = data.read_u64::<LittleEndian>()? as usize;
        let target_dim = data.read_u64::<LittleEndian>()? as usize;
        let _image_dim = data.read_u64::<LittleEndian>()? as usize;

        let image = Matrix::read_pivot(target_dim, data)?;
        let mut v = FpVector::new(p, source_dim);

        assert_eq!(results.len(), inputs.len());
        for result in &mut *results {
            assert_eq!(result.into().as_slice().len(), source_dim);
        }

        for (i, r) in image.into_iter().enumerate() {
            if r < 0 {
                continue;
            }

            v.update_from_bytes(data)?;
            for (input, result) in inputs.iter().zip_eq(&mut *results) {
                result.into().add(v.as_slice(), input.into().entry(i));
            }
        }
        Ok(())
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
            if let Some(pivots) = self.pivots() {
                if i >= pivots.len() || pivots[i] < 0 {
                    continue;
                }
            }
            if c != 0 {
                target.add(self.preimage[row].as_slice(), (coeff * c) % p);
            }
            row += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_qi() {
        let p = ValidPrime::new(2);
        let qi = QuasiInverse {
            image: Some(vec![0, -1, 1, -1, 2, 3]),
            preimage: Matrix::from_vec(
                p,
                &[
                    vec![1, 0, 1, 1],
                    vec![1, 1, 0, 0],
                    vec![0, 1, 0, 1],
                    vec![1, 1, 1, 0],
                ],
            ),
        };
        let v0 = FpVector::from_slice(p, &[1, 1, 0, 0, 1, 0]);
        let v1 = FpVector::from_slice(p, &[0, 0, 1, 0, 1, 1]);

        let mut out0 = FpVector::new(p, 4);
        let mut out1 = FpVector::new(p, 4);

        let mut cursor = io::Cursor::new(Vec::<u8>::new());
        qi.to_bytes(&mut cursor).unwrap();
        cursor.set_position(0);

        QuasiInverse::stream_quasi_inverse(
            p,
            &mut cursor,
            &mut [out0.as_slice_mut(), out1.as_slice_mut()],
            &[v0.as_slice(), v1.as_slice()],
        )
        .unwrap();

        let mut bench0 = FpVector::new(p, 4);
        let mut bench1 = FpVector::new(p, 4);

        qi.apply(bench0.as_slice_mut(), 1, v0.as_slice());
        qi.apply(bench1.as_slice_mut(), 1, v1.as_slice());

        assert_eq!(out0, bench0, "{out0} != {bench0}");
        assert_eq!(out1, bench1, "{out1} != {bench1}");

        assert_eq!(cursor.position() as usize, cursor.get_ref().len());
    }
}
