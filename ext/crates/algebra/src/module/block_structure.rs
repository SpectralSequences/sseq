// A block structure is a structure that makes it efficient to convert between an index 
// into a sum of vector spaces and an index into an individual one. For instance, a FreeModule
// has some collection of generators in various degrees and a general element is a homogenous sum
// \sum Op^i x_i. The basis for the FreeModule is divided into a block for each generator x_i.
// The entries in the block correspond to a basis for the algebra. In order to multiply by a steenrod operation, 
// we have to pull out each block and multiply each block separately by the steenrod operation.

use bivec::BiVec;
use fp::vector::{FpVector, FpVectorT};

#[derive(Debug)]
pub struct GeneratorBasisEltPair {
    pub generator_degree : i32,
    pub generator_index : usize,
    pub basis_index : usize,    
}

#[derive(Debug)]
pub struct BlockStructure {
    pub total_dimension : usize,
    basis_element_to_block_idx : Vec<GeneratorBasisEltPair>,
    block_starts : BiVec<Vec<BlockStart>>, // generator_degree --> generator_index --> (index, size)
}

#[derive(Debug)]
pub struct BlockStart {
    pub block_start_index : usize,
    pub block_size : usize
}

impl BlockStructure {
    pub fn new(block_sizes : &BiVec<Vec<usize>>) -> Self {
        let mut total_dimension = 0;
        let mut block_starts = BiVec::with_capacity(block_sizes.min_degree(), block_sizes.len());
        let mut basis_element_to_block_idx = Vec::new();
        for (degree, blocks) in block_sizes.iter_enum() {
            let mut block_starts_entry = Vec::with_capacity(blocks.len());
            for (i, size) in blocks.iter().enumerate() {
                block_starts_entry.push(BlockStart { block_start_index : total_dimension, block_size : *size });
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

    // Find the start of the block corresponding to a given generator.
    // Returns (block_start_index, block_size).
    pub fn generator_to_block(&self, gen_deg : i32, gen_idx : usize) -> &BlockStart {
        &self.block_starts[gen_deg][gen_idx]
    }

    // Convert (generator, basis element) to basis element of the sum.
    pub fn generator_basis_elt_to_index(&self, gen_deg : i32, gen_idx : usize, basis_elt : usize) -> usize {
        self.block_starts[gen_deg][gen_idx].block_start_index + basis_elt
    }

    // Find the (generator, basis element) pair corresponding to a given basis element of the block structure.
    // For the FreeModule application, index => (free module generator, Steenrod algebra basis element)
    pub fn index_to_generator_basis_elt(&self, idx : usize) -> &GeneratorBasisEltPair {
        &self.basis_element_to_block_idx[idx]
    }

    // Add source vector "source" to the block indicated by (gen_deg, gen_idx).
    pub fn add_block(&self, target : &mut FpVector, coeff : u32, gen_deg : i32, gen_idx : usize, source : &FpVector){
        let BlockStart { block_start_index : block_min,  block_size } = self.block_starts[gen_deg][gen_idx];
        let block_max = block_min + block_size;
        assert!(source.dimension() == block_size);
        let old_slice = target.slice();
        target.set_slice(block_min, block_max);
        target.add(source, coeff);
        target.restore_slice(old_slice);
    }
}