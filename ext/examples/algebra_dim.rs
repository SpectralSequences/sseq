use algebra::{Algebra, MilnorAlgebra};

fn main() {
    let algebra = MilnorAlgebra::new(fp::prime::TWO);
    algebra.compute_basis(125);
    for n in 0..=125 {
        println!("dim A_{} = {}", n, algebra.dimension(n));
    }
}
