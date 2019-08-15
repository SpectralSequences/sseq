use crate::algebra::AlgebraAny;
use crate::field::Field;
use crate::module::Module;
use crate::finite_dimensional_module::FiniteDimensionalModuleT;

struct HomModule<M : FiniteDimensionalModuleT> {
    algebra : Rc<AlgebraAny>,
    source : Rc<FreeModule>,
    target : Rc<M>,

}

impl<M : FiniteDimensionalModuleT> HomModule<M> {
    new(p, source : Rc<FreeModule>, target : Rc<M>) -> Self {
        let algebra = AlgebraAny::from(Field::new(p));
        Self {
            algebra,
            source,
            target
        }
    }
}

impl Module for HomModule {
    fn get_algebra(&self) -> Rc<AlgebraAny> {
        Rc::clone(&self.algebra)
    }

    fn get_name(&self) -> &str {
        &""
    }

    fn get_min_degree(&self) -> i32 {
        self.target.get_max_degree()
    }

    fn compute_basis(&self, degree : i32) {
        self.source.compute_basis(degree + self.target.max_degree());
        for i in self.target.get_min_degree() .. self.target.max_degree() {
            
        }
    }

    fn get_dimension(&self, degree : i32) -> usize {

    }

    fn act_on_basis(&self, result : &mut FpVector, coeff : u32, op_degree : i32, op_index : usize, mod_degree : i32, mod_index : usize) {
        assert!(op_degree == 0);
        assert!(op_idx == 0);
        result.add_basis_element(mod_index, coeff);    
    }
    
    fn basis_element_to_string(&self, degree : i32, idx : usize) -> String {
        "".to_string()
    }
}