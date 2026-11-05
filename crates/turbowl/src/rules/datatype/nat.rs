//! Natural numbers of any size, for the one lexical mapping that needs
//! arithmetic: an `owl:rational` literal `n/d` in lowest terms. Every other
//! number of the map is read without arithmetic (its digits are its value).
//! Rational literals are rare, so this is plain schoolbook code.

use std::cmp::Ordering;

/// A natural number: base 2^32 limbs, least significant first, with no
/// zero limb on top (zero has no limb).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Nat(Vec<u32>);

impl Nat {
    /// The value of a string of ASCII digits.
    pub(super) fn parse(digits: &str) -> Nat {
        let mut n = Nat(Vec::new());
        for b in digits.bytes() {
            n.mul_add(10, u32::from(b - b'0'));
        }
        n
    }

    pub(super) fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    pub(super) fn is_one(&self) -> bool {
        self.0 == [1]
    }

    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    /// `self * m + a`.
    pub(super) fn mul_add(&mut self, m: u32, a: u32) {
        let mut carry = u64::from(a);
        for limb in &mut self.0 {
            let v = u64::from(*limb) * u64::from(m) + carry;
            *limb = v as u32;
            carry = v >> 32;
        }
        if carry > 0 {
            self.0.push(carry as u32);
        }
    }

    /// Divides by `d` in place, and returns the remainder.
    pub(super) fn div_small(&mut self, d: u32) -> u32 {
        let mut rem = 0u64;
        for limb in self.0.iter_mut().rev() {
            let v = (rem << 32) | u64::from(*limb);
            *limb = (v / u64::from(d)) as u32;
            rem = v % u64::from(d);
        }
        self.trim();
        rem as u32
    }

    /// Divides by `d` in place when it divides, and says whether it did.
    pub(super) fn divide_out(&mut self, d: u32) -> bool {
        let mut q = self.clone();
        if q.div_small(d) == 0 {
            *self = q;
            true
        } else {
            false
        }
    }

    fn bits(&self) -> u32 {
        self.0
            .last()
            .map_or(0, |top| self.0.len() as u32 * 32 - top.leading_zeros())
    }

    fn bit(&self, i: u32) -> bool {
        self.0.get((i / 32) as usize).is_some_and(|l| l >> (i % 32) & 1 == 1)
    }

    fn trailing_zeros(&self) -> u32 {
        let mut zeros = 0;
        for &limb in &self.0 {
            if limb != 0 {
                return zeros + limb.trailing_zeros();
            }
            zeros += 32;
        }
        zeros
    }

    fn shr(&mut self, bits: u32) {
        let (limbs, bits) = ((bits / 32) as usize, bits % 32);
        self.0.drain(..limbs.min(self.0.len()));
        if bits > 0 {
            for i in 0..self.0.len() {
                let high = self.0.get(i + 1).map_or(0, |h| h << (32 - bits));
                self.0[i] = (self.0[i] >> bits) | high;
            }
        }
        self.trim();
    }

    fn shl(&mut self, bits: u32) {
        let (limbs, bits) = ((bits / 32) as usize, bits % 32);
        if bits > 0 {
            let mut carry = 0;
            for limb in &mut self.0 {
                let next = *limb >> (32 - bits);
                *limb = (*limb << bits) | carry;
                carry = next;
            }
            if carry > 0 {
                self.0.push(carry);
            }
        }
        self.0.splice(0..0, std::iter::repeat_n(0, limbs));
        self.trim();
    }

    /// `self - other`, for `other <= self`.
    fn sub(&mut self, other: &Nat) {
        let mut borrow = 0i64;
        for (i, limb) in self.0.iter_mut().enumerate() {
            let v = i64::from(*limb) - i64::from(other.0.get(i).copied().unwrap_or(0)) - borrow;
            borrow = i64::from(v < 0);
            *limb = (v + (borrow << 32)) as u32;
        }
        self.trim();
    }

    /// The greatest common divisor (binary GCD), for nonzero `a` and `b`.
    pub(super) fn gcd(mut a: Nat, mut b: Nat) -> Nat {
        let shift = a.trailing_zeros().min(b.trailing_zeros());
        a.shr(a.trailing_zeros());
        loop {
            b.shr(b.trailing_zeros());
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            b.sub(&a);
            if b.is_zero() {
                break;
            }
        }
        a.shl(shift);
        a
    }

    /// `self / d` (long division), for nonzero `d`.
    pub(super) fn div(&self, d: &Nat) -> Nat {
        let mut q = Nat(vec![0; self.0.len()]);
        let mut r = Nat(Vec::new());
        for i in (0..self.bits()).rev() {
            r.shl(1);
            if self.bit(i) {
                if r.0.is_empty() {
                    r.0.push(1);
                } else {
                    r.0[0] |= 1;
                }
            }
            if r >= *d {
                r.sub(d);
                q.0[(i / 32) as usize] |= 1 << (i % 32);
            }
        }
        q.trim();
        q
    }

    /// The decimal digits, without leading zeros (empty for zero).
    pub(super) fn digits(&self) -> String {
        let mut n = self.clone();
        let mut chunks = Vec::new();
        while !n.is_zero() {
            chunks.push(n.div_small(1_000_000_000));
        }
        let mut text = String::new();
        for (i, chunk) in chunks.iter().rev().enumerate() {
            if i == 0 {
                text.push_str(&chunk.to_string());
            } else {
                text.push_str(&format!("{chunk:09}"));
            }
        }
        text
    }
}

impl PartialOrd for Nat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Nat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(digits: &str) -> Nat {
        Nat::parse(digits)
    }

    #[test]
    fn digits_round_trip() {
        for d in [
            "",
            "1",
            "9",
            "10",
            "4294967295",
            "4294967296",
            "1000000000",
            "123456789012345678901234567890",
        ] {
            assert_eq!(n(d).digits(), d);
        }
        assert!(n("000").is_zero());
        assert_eq!(n("007").digits(), "7");
    }

    #[test]
    fn gcd_and_division() {
        let big = "340282366920938463463374607431768211456"; // 2^128
        for (a, b, g) in [
            ("12", "18", "6"),
            ("7", "13", "1"),
            ("1", "1", "1"),
            (big, "6", "2"),
            (
                "100000000000000000000000000000000000000000",
                "60000000000000000000000000000",
                "20000000000000000000000000000",
            ),
        ] {
            assert_eq!(Nat::gcd(n(a), n(b)).digits(), g, "gcd({a}, {b})");
        }
        assert_eq!(n(big).div(&n("4294967296")).digits(), "79228162514264337593543950336");
        assert_eq!(n(big).div(&n("2")).digits(), "170141183460469231731687303715884105728");
        assert_eq!(n("1000").div(&n("7")).digits(), "142");
        assert_eq!(n("12").div(&n("12")).digits(), "1");
        let mut m = n("500");
        assert!(m.divide_out(5));
        assert!(!m.divide_out(3));
        assert_eq!(m.digits(), "100");
    }
}
