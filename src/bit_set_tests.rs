use std::ops::BitOrAssign;
use std::convert::TryInto;
use std::slice;

use bit_set;

#[cfg(test)]

#[test]
fn test_set_bit() {
    let mut bs = bit_set::Bitset::new(); 
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
    let mut bs = bit_set::Bitset::new(); 
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
    let mut bs = bit_set::Bitset::new(); 
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
    let mut bs = bit_set::Bitset::new(); 
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
    let mut bs = bit_set::Bitset::new(); 
    bs.set_bit(3);
    bs.set_bit(6);
    bs.set_bit(1);
    bs.set_bit(6000);
    bs.set_bit(6);

    let mut bs2 = bit_set::Bitset::new(); 
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



