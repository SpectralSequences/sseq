use crate::fp_vector::FpVector;
// use crate::once::OnceRefOwned;
use crate::matrix::{Matrix, Subspace};
use crate::memory::CVec;
use crate::algebra::Algebra;
use crate::module::{Module, ZeroModule};
use crate::module_homomorphism::{ModuleHomomorphism, ZeroHomomorphism};


pub trait ChainComplex {
    fn get_prime(&self) -> u32 {
        self.get_algebra().get_prime()
    }
    fn get_algebra(&self) -> &Algebra;
    fn get_min_degree(&self) -> i32;
    fn get_module(&self, homological_degree : usize) -> &Module;
    fn get_differential(&self, homological_degree : usize) -> &ModuleHomomorphism;
    fn compute_through_bidegree(&self, homological_degree : usize, degree : i32) {}
    // fn set_kernel(&self, homological_degree : usize, degree : i32, kernel : Subspace);
    // fn set_image(&self, degree : i32, homological_degree : usize, image : Subspace);
    // fn get_kernel(&self, homological_degree : usize, degree : i32) -> &Subspace;
    // fn get_image(&self, homological_degree : usize, degree : i32) -> Option<&Subspace>;
    // fn get_quasi_inverse(&self, degree : i32, homological_degree : usize) -> &QuasiInverse;

    fn compute_kernel_and_image(&self,  homological_degree : usize, degree : i32){
        let p = self.get_prime();
        let d = self.get_differential(homological_degree);
        if homological_degree == 0 {
            let module = self.get_module(0);
            let dim = module.get_dimension(degree);
            let kernel = Subspace::entire_space(p, dim);
            d.set_kernel(degree, kernel);
        }
        let source_dimension = d.get_source().get_dimension(degree);
        let target_dimension = d.get_target().get_dimension(degree);
        let padded_target_dimension = FpVector::get_padded_dimension(p, target_dimension, 0);
        let columns = padded_target_dimension + source_dimension;
        let mut matrix = Matrix::new(p, source_dimension, columns);
        d.get_matrix(&mut matrix, degree, 0, 0);
        for i in 0..source_dimension {
            matrix[i].set_entry(padded_target_dimension + i, 1);
        }
        let mut pivots = CVec::new(columns);
        matrix.row_reduce(&mut pivots);
        let kernel_rows = d.copy_kernel_from_matrix(degree, &mut matrix, &pivots, padded_target_dimension);
        let image_rows = matrix.get_rows() - kernel_rows;
        d.copy_image_from_matrix(degree, &mut matrix, &pivots, image_rows, target_dimension);
        d.copy_quasi_inverse_from_matrix(degree, &mut matrix, image_rows, padded_target_dimension);
    }
}


pub struct ChainComplexConcentratedInDegreeZeroModules<'a> {
    module : &'a Module,
    zero_module : ZeroModule<'a>
}

pub struct ChainComplexConcentratedInDegreeZeroHomomorphisms<'b> {
    d0 : ZeroHomomorphism<'b, 'b>,
    d1 : ZeroHomomorphism<'b, 'b>,
    other_ds : ZeroHomomorphism<'b, 'b>,
}

rental!{
    pub mod rent_ccdz {
        use super::*;
        #[rental]
        pub struct ChainComplexConcentratedInDegreeZeroInner<'a> {
            modules : Box<ChainComplexConcentratedInDegreeZeroModules<'a>>,
            homomorphisms : ChainComplexConcentratedInDegreeZeroHomomorphisms<'modules>
        }
    }
}



pub struct ChainComplexConcentratedInDegreeZero<'a> {
    ccdz_inner : rent_ccdz::ChainComplexConcentratedInDegreeZeroInner<'a>
}

impl<'a> ChainComplexConcentratedInDegreeZero<'a> {
    pub fn new(module : &'a Module) -> Self {
        let p = module.get_prime();
        let zero_module = ZeroModule::new(module.get_algebra());
        let ccdzm = ChainComplexConcentratedInDegreeZeroModules {
            module,
            zero_module
        };
        let ccdzm_box = Box::new(ccdzm);
        let ccdz_inner = rent_ccdz::ChainComplexConcentratedInDegreeZeroInner::new(
            ccdzm_box,
            |ccdzm| {
                ChainComplexConcentratedInDegreeZeroHomomorphisms {
                    d0 : ZeroHomomorphism::new(ccdzm.module, &ccdzm.zero_module),
                    d1 : ZeroHomomorphism::new(&ccdzm.zero_module, ccdzm.module),
                    other_ds : ZeroHomomorphism::new(&ccdzm.zero_module, &ccdzm.zero_module)
                }
            }
        );
        Self {
            ccdz_inner    
        }
    }
}

impl<'a> ChainComplex for ChainComplexConcentratedInDegreeZero<'a> {
    fn get_algebra(&self) -> &Algebra {
        self.ccdz_inner.head().module.get_algebra()
    }

    fn get_module(&self, homological_degree : usize) -> &Module {
        if homological_degree == 0 {
            return self.ccdz_inner.head().module;
        } else {
            return &self.ccdz_inner.head().zero_module;
        }
    }

    fn get_min_degree(&self) -> i32 {
        self.ccdz_inner.head().module.get_min_degree()
    }

    // fn get_max_degree(&self) -> i32 {
    //     self.ccdz.head().module.get_max_degree()
    // }

    fn get_differential<'b>(&'b self, homological_degree : usize) -> &'b ModuleHomomorphism {
        self.ccdz_inner.rent(|ccdzd| {
            let result = match homological_degree {
                0 => &ccdzd.d0,
                1 => &ccdzd.d1,
                _ => &ccdzd.other_ds
            };
            unsafe{
                std::mem::transmute::<_, &'b ZeroHomomorphism<'b, 'b>>(result)
            }
        })    
    }

    // fn get_quasi_inverse(&self, degree : i32, homological_degree : usize) -> QuasiInverse {
    //     let qi_pivots = self.image_deg_zero[degree].get();
    //     QuasiInverse {
            
    //     }
    // }
}