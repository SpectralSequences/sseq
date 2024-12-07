use build_const::build_const;

build_const!("constants");

#[macro_export]
macro_rules! const_for {
    ($i:ident in $a:literal.. $b:ident $contents:block) => {
        let mut $i = $a;
        while $i < $b {
            $contents;
            $i += 1;
        }
    };
}

pub(crate) static INVERSE_TABLE: [[u32; MAX_PRIME]; NUM_PRIMES] = const {
    let mut result = [[0; MAX_PRIME]; NUM_PRIMES];
    const_for! { i in 0 .. NUM_PRIMES {
        let p = PRIMES[i];
        const_for! { k in 1 .. p {
            result[i][k as usize] = crate::prime::power_mod(p, k, p - 2);
        }}
    }};
    result
};

macro_rules! populate_binomial_table {
    ($res:expr, $size:ident, $mod:expr) => {
        const_for! { n in 0 .. $size {
            $res[n][0] = 1;
            const_for! { k in 0 .. n {
                $res[n][k + 1] = ($res[n - 1][k] + $res[n - 1][k + 1]) % $mod;
            }}
        }}
    };
}

pub(crate) const BINOMIAL4_TABLE_SIZE: usize = 50;

pub(crate) const BINOMIAL4_TABLE: [[u32; BINOMIAL4_TABLE_SIZE]; BINOMIAL4_TABLE_SIZE] = {
    let mut res = [[0; BINOMIAL4_TABLE_SIZE]; BINOMIAL4_TABLE_SIZE];
    populate_binomial_table!(res, BINOMIAL4_TABLE_SIZE, 4);
    res
};

pub(crate) static BINOMIAL_TABLE: [[[u32; MAX_PRIME]; MAX_PRIME]; NUM_PRIMES] = {
    let mut result = [[[0; MAX_PRIME]; MAX_PRIME]; NUM_PRIMES];
    const_for! { i in 0 .. NUM_PRIMES {
        let p = PRIMES[i];
        let pu = p as usize;
        populate_binomial_table!(result[i], pu, p);
    }}
    result
};
