use bivec::BiVec;
use fp::vector::{FpVector};

use crate::Algebra;
use crate::module::Module;
use crate::module::FDModule;

pub trait BoundedModule: Module {
    /// `max_degree` is the a degree such that if t > `max_degree`, then `self.dimension(t) = 0`.
    fn max_degree(&self) -> i32;

    fn total_dimension(&self) -> usize {
        let mut sum = 0;
        for i in 0..=self.max_degree() {
            sum += self.dimension(i);
        }
        sum
    }

    fn to_fd_module(&self) -> FDModule<Self::Algebra> {
        let min_degree = self.min_degree();
        let max_degree = self.max_degree();
        self.compute_basis(max_degree);

        let mut graded_dimension = BiVec::with_capacity(min_degree, max_degree + 1);
        for t in min_degree..=max_degree {
            graded_dimension.push(self.dimension(t));
        }
        let mut result = FDModule::new(self.algebra(), self.name(), graded_dimension);
        for t in min_degree..=max_degree {
            for idx in 0..result.dimension(t) {
                result.set_basis_element_name(t, idx, self.basis_element_to_string(t, idx));
            }
        }

        let algebra = self.algebra();
        for input_degree in min_degree..=max_degree {
            for output_degree in (input_degree + 1)..=max_degree {
                let output_dimension = result.dimension(output_degree);
                if output_dimension == 0 {
                    continue;
                }
                let op_degree = output_degree - input_degree;

                for input_idx in 0..result.dimension(input_degree) {
                    for op_idx in 0..algebra.dimension(op_degree, -1) {
                        let output_vec: &mut FpVector =
                            result.action_mut(op_degree, op_idx, input_degree, input_idx);
                        self.act_on_basis(
                            output_vec,
                            1,
                            op_degree,
                            op_idx,
                            input_degree,
                            input_idx,
                        );
                    }
                }
            }
        }
        result
    }
}
