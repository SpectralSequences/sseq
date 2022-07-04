use bivec::BiVec;
use fp::vector::{prelude::*, Slice, SliceMut};
use std::ops::Range;

#[derive(Debug)]
pub struct GeneratorBasisEltPair {
    pub generator_degree: i32,
    pub generator_index: usize,
    pub basis_index: usize,
}

/// A block structure is a structure that makes it efficient to convert between an index
/// into a sum of vector spaces and an index into an individual one. For instance, a FreeModule
/// has some collection of generators in various degrees and a general element is a homogenous sum
/// \sum Op^i x_i. The basis for the FreeModule is divided into a block for each generator x_i.
/// The entries in the block correspond to a basis for the algebra. In order to multiply by a steenrod operation,
/// we have to pull out each block and multiply each block separately by the steenrod operation.
#[derive(Debug)]
pub struct BlockStructure {
    basis_element_to_block_idx: Vec<GeneratorBasisEltPair>,
    blocks: BiVec<Vec<Range<usize>>>, // generator_degree --> generator_index --> (index, size)
}

impl BlockStructure {
    pub fn new(block_sizes: &BiVec<Vec<usize>>) -> Self {
        let mut total_dimension = 0;
        let mut blocks = BiVec::with_capacity(block_sizes.min_degree(), block_sizes.len());
        let mut basis_element_to_block_idx = Vec::new();
        for (degree, block_sizes) in block_sizes.iter_enum() {
            let mut block_starts_entry = Vec::with_capacity(block_sizes.len());
            for (i, size) in block_sizes.iter().enumerate() {
                block_starts_entry.push(total_dimension..total_dimension + size);
                for j in 0..*size {
                    basis_element_to_block_idx.push(GeneratorBasisEltPair {
                        generator_degree: degree,
                        generator_index: i,
                        basis_index: j,
                    });
                }
                total_dimension += size;
            }
            blocks.push(block_starts_entry);
        }
        Self {
            basis_element_to_block_idx,
            blocks,
        }
    }

    /// Find the block corresponding to a given generator.
    pub fn generator_to_block(&self, gen_deg: i32, gen_idx: usize) -> Range<usize> {
        self.blocks[gen_deg][gen_idx].clone()
    }

    /// Convert (generator, basis element) to basis element of the sum.
    pub fn generator_basis_elt_to_index(
        &self,
        gen_deg: i32,
        gen_idx: usize,
        basis_elt: usize,
    ) -> usize {
        self.blocks[gen_deg][gen_idx].start + basis_elt
    }

    /// Find the (generator, basis element) pair corresponding to a given basis element of the block structure.
    /// For the FreeModule application, index => (free module generator, Steenrod algebra basis element)
    pub fn index_to_generator_basis_elt(&self, idx: usize) -> &GeneratorBasisEltPair {
        &self.basis_element_to_block_idx[idx]
    }

    /// Add source vector "source" to the block indicated by (gen_deg, gen_idx).
    pub fn add_block(
        &self,
        mut target: SliceMut,
        coeff: u32,
        gen_deg: i32,
        gen_idx: usize,
        source: Slice,
    ) {
        let range = &self.blocks[gen_deg][gen_idx];
        assert!(source.len() == range.len());
        target.slice_mut(range.start, range.end).add(source, coeff);
    }

    pub fn total_dimension(&self) -> usize {
        self.basis_element_to_block_idx.len()
    }
}
