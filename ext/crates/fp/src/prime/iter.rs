pub struct BitflagIterator {
    remaining: u8,
    flag: u64,
}

impl BitflagIterator {
    pub fn new(flag: u64) -> Self {
        Self {
            remaining: u8::max_value(),
            flag,
        }
    }

    pub fn new_fixed_length(flag: u64, remaining: usize) -> Self {
        assert!(remaining <= 64);
        let remaining = remaining as u8;
        Self { remaining, flag }
    }

    pub fn set_bit_iterator(flag: u64) -> impl Iterator<Item = usize> {
        Self::new(flag)
            .enumerate()
            .filter_map(|(idx, v)| if v { Some(idx) } else { None })
    }
}

impl Iterator for BitflagIterator {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining > 64 && self.flag == 0 || self.remaining == 0 {
            None
        } else {
            self.remaining -= 1;
            let result = self.flag & 1 == 1;
            self.flag >>= 1;
            Some(result)
        }
    }
}

/// Iterates through all numbers with the same number of bits. It may panic or return nonsense
/// after all valid values are returned.
pub struct BinomialIterator {
    value: u32,
}

impl BinomialIterator {
    pub fn new(n: usize) -> Self {
        Self {
            value: (1 << n) - 1,
        }
    }
}

impl Iterator for BinomialIterator {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let v = self.value;
        let c = v & v.wrapping_neg();
        let r = v + c;
        let n = (r ^ v).wrapping_shr(2 + v.trailing_zeros());
        self.value = r | n;
        Some(v)
    }
}
