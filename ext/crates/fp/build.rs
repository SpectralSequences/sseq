use std::io::Error;

use build_const::ConstWriter;
use itertools::Itertools;

type Limb = u64;

fn main() -> Result<(), Error> {
    #[cfg(feature = "odd-primes")]
    let max_prime: u32 = 7;
    #[cfg(not(feature = "odd-primes"))]
    let max_prime: u32 = 2;
    let primes = primes_up_to_n(max_prime);

    write_constants(&primes)?;
    write_macros(&primes)?;

    Ok(())
}

fn write_constants(primes: &[u32]) -> Result<(), Error> {
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

    writer.add_raw("/// The number of primes that are supported.");
    writer.add_value("NUM_PRIMES", "usize", num_primes);
    writer.add_raw(
        "/// The `NUM_PRIMES`th prime number. Constructing a `ValidPrime` using any number larger \
         than this value will cause a panic.",
    );
    writer.add_value("MAX_PRIME", "usize", max_prime);
    writer.add_raw(
        "/// A sentinel value. `PRIME_TO_INDEX_MAP[i] == NOT_A_PRIME` if and only if `i` is less \
         than `MAX_PRIME` and not a prime number.",
    );
    // `NOT_A_PRIME` is never used if odd-primes is disabled.
    writer.add_raw("#[allow(dead_code)]");
    writer.add_value("NOT_A_PRIME", "usize", not_a_prime);
    writer.add_value("MAX_MULTINOMIAL_LEN", "usize", max_multinomial_len);
    writer.add_raw("/// An array containing the first `NUM_PRIMES` prime numbers.");
    writer.add_array("PRIMES", "u32", &primes);
    writer.add_raw(
        "/// For any integer `i` less than or equal to `MAX_PRIME`, `PRIME_TO_INDEX_MAP[i]` is \
         the index of `i` in `PRIMES` if `i` is prime; otherwise, it is `NOT_A_PRIME`.",
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

fn write_macros(primes: &[u32]) -> Result<(), Error> {
    let mut writer = ConstWriter::for_build("macros")?.finish_dependencies();

    // methods taking `self` and `other` by reference
    let ref_ref = primes
        .iter()
        .map(|&p| format!("(Self::_{p}(ref x), $other::_{p}(ref y)) => x.$method(y, $($arg),*),"))
        .join("\n                ");

    // methods taking `self` by mutable reference and `other` by reference
    let mut_ref_ref = primes
        .iter()
        .map(|&p| {
            format!("(Self::_{p}(ref mut x), $other::_{p}(ref y)) => x.$method(y, $($arg),*),")
        })
        .join("\n                ");

    // methods taking `self` by mutable reference and returning a prime-dependent type
    let mut_ref_dispatch = primes
        .iter()
        .map(|&p| format!("Self::_{p}(ref mut x) => $ret::_{p}(x.$method($($arg),*)),"))
        .join("\n                ");

    // methods taking self by reference and returning a prime-dependent type
    let ref_dispatch = primes
        .iter()
        .map(|&p| format!("Self::_{p}(ref x) => $ret::_{p}(x.$method($($arg),*)),"))
        .join("\n                ");

    // methods taking self by value and returning a prime-dependent type
    let val_dispatch = primes
        .iter()
        .map(|&p| format!("Self::_{p}(x) => $ret::_{p}(x.$method($($arg),*)),"))
        .join("\n                ");

    // methods taking self by mutable reference
    let mut_ref = primes
        .iter()
        .map(|&p| format!("Self::_{p}(ref mut x) => x.$method($($arg),*),"))
        .join("\n                ");

    // methods taking self by reference
    let reff = primes
        .iter()
        .map(|p| format!("Self::_{p}(ref x) => x.$method($($arg),*),"))
        .join("\n                ");

    // dispatch prime generic
    let dispatch_prime_generic = primes
        .iter()
        .map(|p| format!("(Self::_{p}(ref mut x), Slice::_{p}(y)) => x.$method(y $(,$arg)*),"))
        .join("\n                ");

    // generic match_p
    let match_p = primes
        .iter()
        .map(|p| format!("{p} => Self::_{p}($($val)*),"))
        .join("\n            ");

    // dispatch type
    let dispatch_type = primes
        .iter()
        .map(|p| format!("_{p}($generic<{p}>),"))
        .join("\n            ");

    // dispatch type with lifetime
    let dispatch_type_life = primes
        .iter()
        .map(|p| format!("_{p}($generic<$life, {p}>),"))
        .join("\n            ");

    // implement `From` for references
    let impl_from_ref = primes
        .iter()
        .map(|p| format!("$t1::_{p}(x) => $t2::_{p}($t2p::<'a, {p}>::from(x)),"))
        .join("\n                    ");

    // match p over self
    let match_self = primes
        .iter()
        .map(|p| format!("Self::_{p}(x) => $ret::_{p}(x.$method($($arg),*)),"))
        .join("\n            ");

    // match p over a triple (self, left, right)
    let match_self_left_right = primes
        .iter()
        .map(|p| {
            format!("(SliceMut::_{p}(ref mut x), Slice::_{p}(y), Slice::_{p}(z)) => {{ x.$method($($arg),*, y, z) }},")
        })
        .join("\n            ");

    // call a macro for all values of p
    let call_macro = primes
        .iter()
        .map(|p| format!("$macro!(_{p}, {p});"))
        .join("\n        ");

    writer.add_raw(&format!(r#"
macro_rules! dispatch_prime_inner {{
    // other is a type, but marking it as a :ty instead of :tt means we cannot use it to access its
    // enum variants.
    ($vis:vis fn $method:ident(&self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {{
        $vis fn $method(&self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {{
            match (self, other) {{
                {ref_ref}
                (l, r) => {{
                    panic!("Applying {{}} to vectors over different primes ({{}} and {{}})", stringify!($method), l.prime(), r.prime());
                }}
            }}
        }}
    }};
    ($vis:vis fn $method:ident(&mut self, other: &$other:tt $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {{
        #[allow(unused_parens)]
        $vis fn $method(&mut self, other: &$other, $($arg: $ty),* ) $(-> $ret)* {{
            match (self, other) {{
                {mut_ref_ref}
                (l, r) => {{
                    panic!("Applying {{}} to vectors over different primes ({{}} and {{}})", stringify!($method), l.prime(), r.prime());
                }}
            }}
        }}
    }};
    ($vis:vis fn $method:ident(&mut self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {{
        $vis fn $method(&mut self, $($arg: $ty),* ) -> $ret {{
            match self {{
                {mut_ref_dispatch}
            }}
        }}
    }};
    ($vis:vis fn $method:ident(&self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {{
        $vis fn $method(&self, $($arg: $ty),* ) -> $ret {{
            match self {{
                {ref_dispatch}
            }}
        }}
    }};
    ($vis:vis fn $method:ident(self $(, $arg:ident: $ty:ty )* ) -> (dispatch $ret:tt)) => {{
        $vis fn $method(self, $($arg: $ty),* ) -> $ret {{
            match self {{
                {val_dispatch}
            }}
        }}
    }};
    ($vis:vis fn $method:ident(&mut self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {{
        #[allow(unused_parens)]
        $vis fn $method(&mut self, $($arg: $ty),* ) $(-> $ret)* {{
            match self {{
                {mut_ref}
            }}
        }}
    }};
    ($vis:vis fn $method:ident(&self $(, $arg:ident: $ty:ty )* ) $(-> $ret:ty)?) => {{
        #[allow(unused_parens)]
        $vis fn $method(&self, $($arg: $ty),* ) $(-> $ret)* {{
            match self {{
                {reff}
            }}
        }}
    }};
}}

macro_rules! dispatch_prime {{
    () => {{}};
    ($vis:vis fn $method:ident $tt:tt $(-> $ret:tt)?; $($tail:tt)*) => {{
        dispatch_prime_inner! {{
            $vis fn $method $tt $(-> $ret)*
        }}
        dispatch_prime!{{$($tail)*}}
    }};
}}

macro_rules! dispatch_prime_generic_inner {{
    (fn $method:ident(&mut self $(, $arg:ident: $ty:ty )*) $(-> $ret:ty)?) => {{
        fn $method<'b, T: Into<Slice<'b>>>(&mut self, other: T $(,$arg: $ty)*) $(-> $ret)? {{
            match (self, other.into()) {{
                {dispatch_prime_generic}
                (l, r) => panic!(
                    "Applying add to vectors over different primes ({{}} and {{}})",
                    l.prime(),
                    r.prime()
                ),
            }}
        }}
    }}
}}

/// Macro to implement the generic addition methods.
macro_rules! dispatch_prime_generic {{
    () => {{}};
    (fn $method:ident(&mut self $(, $arg:ident: $ty:ty )*) $(-> $ret:ty)?; $($tail:tt)*) => {{
        dispatch_prime_generic_inner! {{
            fn $method(&mut self $(, $arg: $ty )*) $(-> $ret)?
        }}
        dispatch_prime_generic!{{$($tail)*}}
    }}
}}

macro_rules! dispatch_type {{
    (derive($($derive_macro:tt)*), $vis:vis $special:ident {{ $generic:ident }}) => {{
        #[derive($($derive_macro)*)]
        $vis enum $special {{
            {dispatch_type}
        }}
    }};
    (derive($($derive_macro:tt)*), $vis:vis $special:ident<$life:lifetime> {{ $generic:ident }}) => {{
        #[derive($($derive_macro)*)]
        $vis enum $special<$life> {{
            {dispatch_type_life}
        }}
    }};
}}

macro_rules! impl_from_ref {{
    ($t1:tt, $t2:tt, $t2p:tt $(, $m:tt)?) => {{
        impl<'a, 'b> From<&'a $($m)* $t1<'b>> for $t2<'a> {{
            fn from(slice: &'a $($m)* $t1<'b>) -> $t2<'a> {{
                match slice {{
                    {impl_from_ref}
                }}
            }}
        }}
    }};
}}

macro_rules! match_self_p {{
    ($method:ident(&$selff:ident $(, $arg:ident)*) -> $ret:tt) => {{
        match $selff {{
            {match_self}
        }}
    }};
}}

macro_rules! match_self_left_right_p {{
    ($method:ident(&mut $selff:ident $(, $arg:ident)*; $left:ident, $right:ident )) => {{
        match ($selff, $left, $right) {{
            {match_self_left_right}
            _ => {{
                panic!(concat!("Applying method to vectors over different primes"));
            }}
        }}
    }};
}}

macro_rules! match_p {{
    ($p:ident, $($val:tt)*) => {{
        match *$p {{
            {match_p}
            _ => panic!("Prime not supported: {{}}", *$p)
        }}
    }};
}}

macro_rules! call_macro_p {{
    ($macro:ident) => {{
        {call_macro}
    }};
}}"#,
    ));

    Ok(())
}

fn primes_up_to_n(n: u32) -> Vec<u32> {
    (2..=n).filter(|&i| is_prime(i)).collect()
}

fn is_prime(i: u32) -> bool {
    (2..i).all(|k| i % k != 0)
}
