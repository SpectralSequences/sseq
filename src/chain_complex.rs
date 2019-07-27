use crate::fp_vector::FpVector;
use crate::once::OnceRefOwned;
use crate::matrix::{Matrix, Subspace};
use crate::memory::CVec;
use crate::module::{Module, ZeroModule};
use crate::module_homomorphism::{ModuleHomomorphism, ZeroHomomorphism};

pub trait ChainComplex {
    fn get_prime(&self) -> u32;
    fn get_min_degree(&self) -> i32;
    fn get_module(&self, homological_degree : usize) -> &Module;
    fn get_differential(&self, homological_degree : usize) -> &ModuleHomomorphism;
    fn compute_through_bidegree(&self, homological_degree : usize, degree : i32) {}
    fn set_kernel(&self, homological_degree : usize, degree : i32, kernel : Subspace);
    fn set_image(&self, degree : i32, homological_degree : usize, image : Subspace);
    fn get_kernel(&self, homological_degree : usize, degree : i32) -> &Subspace;
    fn get_image(&self, homological_degree : usize, degree : i32) -> &Subspace;
    // fn get_quasi_inverse(&self, degree : i32, homological_degree : usize) -> &QuasiInverse;

    fn compute_kernel_and_image(&self,  homological_degree : usize, degree : i32){
        let p = self.get_prime();
        if homological_degree == 0 {
            let module = self.get_module(0);
            let dim = module.get_dimension(degree);
            let kernel = Subspace::entire_space(p, dim);
            self.set_kernel(homological_degree, degree, kernel);
        }
        let d = self.get_differential(homological_degree);
        let source_dimension = d.get_source().get_dimension(degree);
        let target_dimension = d.get_target().get_dimension(degree);
        let padded_target_dimension = FpVector::get_padded_dimension(p, target_dimension, 0);
        let columns = padded_target_dimension + source_dimension;
        let mut matrix = Matrix::new(p, source_dimension, columns);
        d.get_matrix(&mut matrix, degree, 0, 0);
        for i in 0..source_dimension {
            matrix.vectors[i].set_entry(padded_target_dimension + i, 1);
        }
        let mut pivots = CVec::new(columns);
        matrix.row_reduce(&mut pivots);
        let kernel = matrix.compute_kernel(padded_target_dimension, &pivots);
        let image_rows = matrix.rows - kernel.matrix.rows;
        self.set_kernel(homological_degree, degree, kernel);
        let image = matrix.get_image(image_rows, target_dimension, &pivots);
    }
}


struct ChainComplexConcentratedInDegreeZero<'a> {
    module : &'a Module,
    max_degree : i32,
    zero_module : ZeroModule<'a>,
    d0 : ZeroHomomorphism<'a>,
    d1 : ZeroHomomorphism<'a>,
    other_ds : ZeroHomomorphism<'a>,
    kernel_deg_zero : Vec<OnceRefOwned<Subspace>>,
    image_deg_zero : Vec<OnceRefOwned<Subspace>>,
    zero_subspace : Subspace
}

impl<'a> ChainComplexConcentratedInDegreeZero<'a> {
    pub fn new(module : &'a Module, max_degree : i32) -> Self {
        let p = module.get_prime();
        let zero_module = ZeroModule::new(module.get_algebra());
        // Warning: Stupid Rust acrobatics! Make Rust forget that zero_module_ref depends on zero_module.
        let zero_module_ptr : *const ZeroModule = &zero_module;
        let zero_module_ref : &'a ZeroModule = unsafe{std::mem::transmute(zero_module_ptr)};
        let d0  = ZeroHomomorphism::new(module, zero_module_ref);
        let d1 = ZeroHomomorphism::new(zero_module_ref, module);
        let other_ds = ZeroHomomorphism::new(zero_module_ref, zero_module_ref);
        Self {
            module,
            max_degree,
            zero_module,
            d0,
            d1,
            other_ds,
            kernel_deg_zero : Vec::new(),
            image_deg_zero : Vec::new(),
            zero_subspace : Subspace::new(p, 0, 0),
            // image_generic : CVec::new(0)
        }
    }
}

impl<'a> ChainComplex for ChainComplexConcentratedInDegreeZero<'a> {
    fn get_prime(&self) -> u32 {
        self.module.get_prime()
    }

    fn get_module(&self, homological_degree : usize) -> &Module {
        if homological_degree == 0 {
            return self.module;
        } else {
            return &self.zero_module;
        }
    }

    fn get_min_degree(&self) -> i32 {
        self.module.get_min_degree()
    }

    fn get_differential(&self, homological_degree : usize) -> &ModuleHomomorphism {
        match homological_degree {
            0 => &self.d0,
            1 => &self.d1,
            _ => &self.other_ds
        } 
    }

    fn set_kernel(&self, homological_degree : usize, degree : i32, kernel : Subspace) {
        if homological_degree > 0 {
            return;
        }
        assert!(degree >= self.get_min_degree());
        let degree_idx = (degree - self.get_min_degree()) as usize;
        self.kernel_deg_zero[degree_idx].set(kernel);
    }

    fn get_kernel(&self,  homological_degree : usize, degree : i32) -> &Subspace {
        if homological_degree > 0 {
            return &self.zero_subspace;
        }
        assert!(degree >= self.get_min_degree());
        let degree_idx = (degree - self.get_min_degree()) as usize;        
        return self.kernel_deg_zero[degree_idx].get();
    }

    fn set_image(&self, degree : i32, homological_degree : usize, image : Subspace){
        if homological_degree > 0 {
            return;
        }
        assert!(degree >= self.get_min_degree());
        let degree_idx = (degree - self.get_min_degree()) as usize;
        self.image_deg_zero[degree_idx].set(image);
    }

    fn get_image(&self,  homological_degree : usize, degree : i32) -> &Subspace {
        if homological_degree > 0 {
            return &self.zero_subspace;
        }
        assert!(degree >= self.get_min_degree());
        let degree_idx = (degree - self.get_min_degree()) as usize;        
        return self.image_deg_zero[degree_idx].get();
    }

    // fn get_quasi_inverse(&self, degree : i32, homological_degree : usize) -> QuasiInverse {
    //     let qi_pivots = self.image_deg_zero[degree].get();
    //     QuasiInverse {
            
    //     }
    // }
}