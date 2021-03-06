use algebra::{Algebra, MilnorAlgebra};
use fp::prime::ValidPrime;

fn main() {
    let algebra = MilnorAlgebra::new(ValidPrime::new(2));
    algebra.compute_basis(125);
    for n in 0..=125 {
        println!("dim A_{} = {}", n, algebra.dimension(n, 0));
    }
}
