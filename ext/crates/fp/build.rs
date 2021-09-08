use std::io::Error;

use build_const::ConstWriter;

type Limb = u64;

fn main() -> Result<(), Error> {
    let num_primes = 8;
    let primes = first_n_primes(num_primes);
    let max_prime = *primes.last().unwrap();
    let not_a_prime: usize = !1;
    let max_multinomial_len = 10;

    let prime_to_index_map = (0..=max_prime)
        .map(|i| primes.iter().position(|&j| i == j).unwrap_or(not_a_prime))
        .collect::<Vec<_>>();

    let bytes_per_limb = std::mem::size_of::<Limb>();
    let bits_per_limb = 8 * bytes_per_limb;
    let max_len = 147500;

    let mut bit_lengths = vec![0; num_primes];
    bit_lengths[0] = 1;
    for i in 1..num_primes {
        let p = primes[i];
        bit_lengths[i] = (32 - (p * (p - 1)).leading_zeros()) as usize;
    }

    let bitmasks = (0..num_primes)
        .map(|i| (1 << bit_lengths[i]) - 1)
        .collect::<Vec<Limb>>();

    let entries_per_limb = (0..num_primes)
        .map(|i| bits_per_limb / bit_lengths[i])
        .collect::<Vec<usize>>();

    let mut writer = ConstWriter::for_build("constants")?.finish_dependencies();

    writer.add_value("NUM_PRIMES", "usize", num_primes);
    writer.add_value("MAX_PRIME", "usize", max_prime);
    writer.add_raw("#[allow(dead_code)]");
    writer.add_value("NOT_A_PRIME", "usize", not_a_prime);
    writer.add_value("MAX_MULTINOMIAL_LEN", "usize", max_multinomial_len);
    writer.add_array("PRIMES", "u32", &primes);
    writer.add_array("PRIME_TO_INDEX_MAP", "usize", &prime_to_index_map);

    writer.add_raw(&format!(
        "pub(crate) type Limb = {};",
        std::any::type_name::<Limb>()
    ));
    writer.add_value("BYTES_PER_LIMB", "usize", bytes_per_limb);
    writer.add_value("BITS_PER_LIMB", "usize", bits_per_limb);
    writer.add_value("MAX_LEN", "usize", max_len);

    writer.add_array("BIT_LENGTHS", "usize", &bit_lengths);
    writer.add_array("BITMASKS", "Limb", &bitmasks);
    writer.add_array("ENTRIES_PER_LIMB", "usize", &entries_per_limb);

    Ok(())
}

fn first_n_primes(n: usize) -> Vec<u32> {
    let mut acc = vec![];
    let mut i = 2;
    while acc.len() < n {
        if is_prime(i) {
            acc.push(i);
        }
        i += 1;
    }
    acc
}

fn is_prime(i: u32) -> bool {
    (2..i).all(|k| i % k != 0)
}
