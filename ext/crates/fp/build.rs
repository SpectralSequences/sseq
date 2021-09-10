use std::io::Error;

use build_const::ConstWriter;

type Limb = u64;

fn main() -> Result<(), Error> {
    let num_primes = 8;
    let primes = first_n_primes(num_primes);
    let max_prime = *primes.last().unwrap();
    let not_a_prime: usize = u32::MAX as usize; // Hack for 32-bit architectures
    let max_multinomial_len = 10;

    let prime_to_index_map = (0..=max_prime)
        .map(|i| primes.iter().position(|&j| i == j).unwrap_or(not_a_prime))
        .collect::<Vec<_>>();

    let bytes_per_limb = std::mem::size_of::<Limb>();
    let bits_per_limb = 8 * bytes_per_limb;
    // let max_len = 147500;

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

    writer.add_raw("/// The number of primes that are supported.");
    writer.add_value("NUM_PRIMES", "usize", num_primes);
    writer.add_raw("/// The `MAX_PRIME`th prime number. Constructing a `ValidPrime` using any number larger than");
    writer.add_raw("/// this value will cause a panic.");
    writer.add_value("MAX_PRIME", "usize", max_prime);
    // `NOT_A_PRIME` is never used if odd-primes is disabled.
    writer.add_raw("#[allow(dead_code)]");
    writer.add_raw(
        "/// A sentinel value. `PRIME_TO_INDEX_MAP[i] == NOT_A_PRIME` if and only if `i` is not",
    );
    writer.add_raw("/// a prime number.");
    writer.add_value("NOT_A_PRIME", "usize", not_a_prime);
    writer.add_value("MAX_MULTINOMIAL_LEN", "usize", max_multinomial_len);
    writer.add_raw("/// An array containing the first `NUM_PRIMES` prime numbers.");
    writer.add_array("PRIMES", "u32", &primes);
    writer.add_raw(
        "/// For any integer `i` less than or equal to `MAX_PRIME`, `PRIME_TO_INDEX_MAP[i]` is",
    );
    writer.add_raw(
        "/// the index of `i` in `PRIMES` if `i` is prime; otherwise, it is `NOT_A_PRIME`",
    );
    writer.add_array("PRIME_TO_INDEX_MAP", "usize", &prime_to_index_map);

    writer.add_raw(&format!(
        "pub(crate) type Limb = {};",
        std::any::type_name::<Limb>()
    ));
    writer.add_raw("/// The number of bytes each `Limb` occupies.");
    writer.add_value("BYTES_PER_LIMB", "usize", bytes_per_limb);
    writer.add_raw("/// The number of bits each `Limb` occupies.");
    writer.add_value("BITS_PER_LIMB", "usize", bits_per_limb);
    // writer.add_value("MAX_LEN", "usize", max_len);

    writer.add_raw("/// The number of bits an element of $\\mathbb{F}_p$ occupies in a limb.");
    writer.add_array("BIT_LENGTHS", "usize", &bit_lengths);
    writer.add_raw(
        "/// If `l` is a limb of elements of $\\mathbb{F}_p$, then `l & BITMASKS[p]` is the value",
    );
    writer.add_raw("/// of the first entry of `l`.");
    writer.add_array("BITMASKS", "Limb", &bitmasks);
    writer.add_raw("/// The number of elements of $\\mathbb{F}_p$ that fit in a single limb.");
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
