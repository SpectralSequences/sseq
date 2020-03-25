use bencher::{Bencher, benchmark_group, benchmark_main};
mod multinomial;
mod row_reduce;
use crate::multinomial::main as multinomial; 
use crate::row_reduce::main as row_reduce;

benchmark_main!(multinomial, row_reduce);