//! Mini-library for an optimized `HashSet<usize>` which reduces to bit
//! operations for a small universe.
//!
//! The current implementation falls back to bit operations on a large dense
//! array, which would be problematic if sparse; however, this is used to
//! support the verifier, and will never see an index larger than ~40 on the
//! standard set.mm.  (Thus, on a 64-bit build the fallback code doesn't get
//! exercised at all without special measures.)

use std::ops::BitOrAssign;
use std::convert::TryInto;
use std::slice;


/// A set of variable indices.
#[derive(Default,Debug)]
pub struct Bitset {
    head: usize,
    // You can take out the Box here and it will still compile (and, with more
    // effort, the Option too); the point of this is to optimize the common case
    // of small bitsets at the expense of large ones, as Option<Box> only
    // consumes one word of storage if empty, while Vec and Option<Vec> take
    // three.
    tail: Option<Vec<usize>>,
}

fn bits_per_word() -> usize {
    usize::BITS.try_into().unwrap()
}

impl Clone for Bitset {
    #[inline]
    fn clone(&self) -> Bitset {
        Bitset {
            head: self.head,
            tail: self.tail.as_ref().cloned(),
        }
    }
}

impl Bitset {
    /// Creates a new empty `Bitset`.  Does not allocate.  Equivalent to
    /// `Bitset::default()`.
    pub fn new() -> Bitset {
        Bitset {
            head: 0,
            tail: None,
        }
    }

    fn tail(&self) -> &[usize] {
        match self.tail {
            None => Default::default(),
            Some(ref bx) => bx,
        }
    }

    fn tail_mut(&mut self) -> &mut Vec<usize> {
        if self.tail.is_none() {
            self.tail = Some(Vec::new());
        }
        self.tail.as_mut().unwrap()
    }

    /// Adds a single bit to a set.
    pub fn set_bit(&mut self, bit: usize) {
        if bit < bits_per_word() {
            self.head |= 1 << bit;
        } else {
            let word = bit / bits_per_word() - 1;
            let tail = self.tail_mut();
            if word >= tail.len() {
                tail.resize(word + 1, 0);
            }
            tail[word] |= 1 << (bit & (bits_per_word() - 1));
        }
    }

    /// Tests a set for a specific bit.
    pub fn has_bit(&self, bit: usize) -> bool {
        if bit < bits_per_word() {
            (self.head & (1 << bit)) != 0
        } else {
            let word = bit / bits_per_word() - 1;
            let tail = self.tail();
            word < tail.len() && (tail[word] & (1 << (bit & (bits_per_word() - 1)))) != 0
        }
    }

    /// Adds a single bit to a set, and returns the old value.  Equivalent to
    /// `{ let old = bitset.has_bit(bit); bitset.set_bit(bit); old }`.
    pub fn replace_bit(&mut self, bit: usize) -> bool {
        if bit < bits_per_word() {
            let old = (self.head & (1 << bit)) != 0;
            self.head |= 1 << bit;
            old
        } else {
            let word = bit / bits_per_word() - 1;
            let tail = self.tail_mut();
            let mask = 1 << (bit & (bits_per_word() - 1));
            let old = if word >= tail.len() {
                tail.resize(word + 1, 0);
                false
            } else {
                (tail[word] & mask) != 0
            };
            tail[word] |= mask;
            old
        }
    }
}

impl<'a> BitOrAssign<&'a Bitset> for Bitset {
    fn bitor_assign(&mut self, rhs: &'a Bitset) {
        self.head |= rhs.head;
        if let Some(ref rtail) = rhs.tail {
            let stail = self.tail_mut();
            if rtail.len() > stail.len() {
                stail.resize(rtail.len(), 0);
            }
            for i in 0..rtail.len() {
                stail[i] |= rtail[i];
            }
        }
    }
}

impl<'a> IntoIterator for &'a Bitset {
    type Item = usize;
    type IntoIter = BitsetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        BitsetIter {
            bits: self.head,
            offset: 0,
            buffer: self.tail().iter(),
        }
    }
}

/// Iterator for set bits in a bitset.
pub struct BitsetIter<'a> {
    bits: usize,
    offset: usize,
    buffer: slice::Iter<'a, usize>,
}

impl<'a> Iterator for BitsetIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        while self.bits == 0 {
            self.offset += bits_per_word();
            match self.buffer.next() {
                Some(bits) => self.bits = *bits,
                None => return None,
            }
        }
        let tz = self.bits.trailing_zeros() as usize;
        self.bits &= self.bits - 1;
        Some(tz + self.offset)
    }
}


#[cfg(test)]
mod tests {

    use bit_set::Bitset;

    #[test]
    fn test_set_bit() {
        let mut bs = Bitset::new(); 
        bs.set_bit(3);
        bs.set_bit(1);
        bs.set_bit(7);

        assert!(bs.has_bit(1));
        assert!(bs.has_bit(3));
        assert!(bs.has_bit(7));
        assert!(!bs.has_bit(2));
        assert!(!bs.has_bit(4));
        assert!(!bs.has_bit(6));
        assert!(!bs.has_bit(8));
        assert!(!bs.has_bit(0));

        assert!(!bs.has_bit(66000));
        bs.set_bit(66000);
        assert!(bs.has_bit(66000));
    }

    #[test]
    fn test_replace_bit() {
        let mut bs = Bitset::new(); 
        bs.set_bit(3);
        bs.set_bit(1);
        bs.set_bit(7);

        assert!(!bs.replace_bit(2));
        assert!(bs.replace_bit(2));
        assert!(bs.replace_bit(2));

        assert!(bs.has_bit(1));
        assert!(bs.has_bit(2));
        assert!(bs.has_bit(3));
        assert!(bs.has_bit(7));
        assert!(!bs.has_bit(4));
        assert!(!bs.has_bit(6));
        assert!(!bs.has_bit(8));
        assert!(!bs.has_bit(0));

        assert!(!bs.replace_bit(66000));
        assert!(bs.has_bit(66000));
        bs.set_bit(66000);
        assert!(bs.has_bit(66000));
    }

    #[test]
    fn test_clone() {
        let mut bs = Bitset::new(); 
        bs.set_bit(3);
        bs.set_bit(6);
        bs.set_bit(1);
        bs.set_bit(6000);
        bs.set_bit(6);

        let mut bs2 = bs.clone();

        assert!(bs2.has_bit(1));
        assert!(!bs2.has_bit(2));
        assert!(bs2.has_bit(3));
        assert!(bs2.has_bit(6));
        assert!(!bs2.has_bit(4));
        assert!(!bs2.has_bit(5));
        assert!(!bs2.has_bit(7));
        assert!(!bs2.has_bit(8));

        // clone change does not touch original
        bs2.set_bit(2);
        assert!(bs2.has_bit(2));
        assert!(!bs.has_bit(2));
    }

    
    #[test]
    fn test_iterator() {
        let mut bs = Bitset::new(); 
        bs.set_bit(3);
        bs.set_bit(6);
        bs.set_bit(1);
        bs.set_bit(6000);
        bs.set_bit(6);

        let mut iter = bs.into_iter();

        assert_eq!(1, iter.next().unwrap());
        assert_eq!(3, iter.next().unwrap());
        assert_eq!(6, iter.next().unwrap());
        assert_eq!(6000, iter.next().unwrap());

        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn test_bitor_assign() {
        let mut bs = Bitset::new(); 
        bs.set_bit(3);
        bs.set_bit(6);
        bs.set_bit(1);
        bs.set_bit(6000);
        bs.set_bit(6);

        let bs2 = Bitset::new(); 
        bs.set_bit(7);
        bs.set_bit(7000);
        
        bs |= &bs2;

        assert!(bs.has_bit(1));
        assert!(!bs.has_bit(2));
        assert!(bs.has_bit(3));
        assert!(bs.has_bit(6));
        assert!(!bs.has_bit(4));
        assert!(!bs.has_bit(5));
        assert!(bs.has_bit(7));
        assert!(bs.has_bit(6000));
        assert!(bs.has_bit(7000));
        assert!(!bs.has_bit(8000));
    }

}
