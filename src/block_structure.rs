use bivec::BiVec;
use crate::fp_vector::{FpVector, FpVectorT};

#[derive(Debug)]
pub struct GeneratorBasisEltPair {
    pub generator_degree : i32,
    pub generator_index : usize,
    pub basis_index : usize,    
}

pub struct BlockStructure {
    pub total_dimension : usize,
    basis_element_to_block_idx : Vec<GeneratorBasisEltPair>,
    block_starts : BiVec<Vec<(usize, usize)>>, // generator_degree --> generator_index --> (index, size)
}

impl BlockStructure {
    pub fn new(block_sizes : &BiVec<Vec<usize>>) -> Self {
        let mut total_dimension = 0;
        let mut block_starts = BiVec::with_capacity(block_sizes.min_degree(), block_sizes.len());
        let mut basis_element_to_block_idx = Vec::new();
        for (degree, blocks) in block_sizes.iter_enum() {
            let mut block_starts_entry = Vec::with_capacity(blocks.len());
            for (i, size) in blocks.iter().enumerate() {
                block_starts_entry.push((total_dimension, *size));
                for j in 0 .. *size {
                    basis_element_to_block_idx.push(
                        GeneratorBasisEltPair {
                            generator_degree : degree,
                            generator_index : i,
                            basis_index : j
                        }
                    );
                }
                total_dimension += size;
            }
            block_starts.push(block_starts_entry);
        }
        Self {
            total_dimension,
            basis_element_to_block_idx,
            block_starts
        }
    }

    pub fn generator_to_block(&self, gen_deg : i32, gen_idx : usize) -> (usize, usize) {
        self.block_starts[gen_deg][gen_idx]
    }

    pub fn generator_basis_elt_to_index(&self, gen_deg : i32, gen_idx : usize, basis_elt : usize) -> usize {
        self.block_starts[gen_deg][gen_idx].0 + basis_elt
    }

    pub fn index_to_generator_basis_elt(&self, idx : usize) -> &GeneratorBasisEltPair {
        &self.basis_element_to_block_idx[idx]
    }

    pub fn add_block(&self, target : &mut FpVector, coeff : u32, gen_deg : i32, gen_idx : usize, source : &FpVector){
        let (block_min, block_dimension) = self.block_starts[gen_deg][gen_idx];
        let block_max = block_min + block_dimension;
        assert!(source.get_dimension() == block_dimension);
        let old_slice = target.get_slice();
        target.set_slice(block_min, block_max);
        target.shift_add(source, coeff);
        target.restore_slice(old_slice);
    }
}