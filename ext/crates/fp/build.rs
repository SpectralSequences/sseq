use std::io;

use build_const::ConstWriter;

type Limb = u64;

fn main() -> io::Result<()> {
    // We want primes up to 2^8 - 1, because those will be the characteristics of the fields that
    // have degree at least 2 and order at most 2^16 - 1. We will use PRIME_TO_INDEX_MAP when
    // computing Zech logarithms.
    let prime_bound = u8::MAX;
    let primes = primes_up_to(prime_bound);
    let num_primes = primes.len();
    let max_prime = *primes.last().unwrap();
    let not_a_prime: usize = u32::MAX as usize; // Hack for 32-bit architectures
    let max_multinomial_len = 10;

    let prime_to_index_map = (0..=max_prime)
        .map(|i| primes.iter().position(|&j| i == j).unwrap_or(not_a_prime))
        .collect::<Vec<_>>();

    let bytes_per_limb = std::mem::size_of::<Limb>();
    let bits_per_limb = 8 * bytes_per_limb;

    let mut writer = ConstWriter::for_build("constants")?.finish_dependencies();

    writer.add_raw("/// The number of primes that will be prepared at compile-time. This includes");
    writer.add_raw("/// precomputing inverses, binomial coefficients, etc.");
    writer.add_value("NUM_PRIMES", "usize", num_primes);
    writer.add_raw("/// The `NUM_PRIMES`th prime number.");
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
        "/// the index of `i` in `PRIMES` if `i` is prime; otherwise, it is `NOT_A_PRIME`.",
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

    Ok(())
}

fn primes_up_to(n: impl Into<u32>) -> Vec<u32> {
    (2..=n.into()).filter(|&i| is_prime(i)).collect()
}

fn is_prime(i: u32) -> bool {
    (2..i).all(|k| i % k != 0)
}
