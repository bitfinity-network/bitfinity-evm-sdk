use std::fmt::Debug;

use encode_macros::FixedStorable;
use ic_stable_structures::{BoundedStorable, Storable};

#[derive(FixedStorable, PartialEq, Eq, Debug)]
struct StorableType<const SIZE: usize>([u8; SIZE]);

impl<const SIZE: usize> StorableType<SIZE> {
    pub fn new() -> Self {
        Self([SIZE as _; SIZE])
    }
}

#[derive(FixedStorable, Eq, PartialEq, Debug)]
struct SingleValueNamedStruct {
    val: StorableType<10>,
}

#[derive(FixedStorable, Eq, PartialEq, Debug)]
struct SingleValueTuple(StorableType<10>);

#[derive(FixedStorable, Eq, PartialEq, Debug)]
struct TwoValuesNamedStruct {
    val_1: StorableType<10>,
    val_2: StorableType<20>,
}

#[derive(FixedStorable, Eq, PartialEq, Debug)]
struct TwoValueTuple(StorableType<10>, StorableType<20>);

fn check_storable_roundtrip<T: Storable + Eq + Debug>(val: &T) {
    let restored_value = T::from_bytes(val.to_bytes());
    assert_eq!(&restored_value, val);
}

#[test]
fn test_storable_roundtrip() {
    check_storable_roundtrip(&SingleValueNamedStruct {
        val: StorableType::new(),
    });
    check_storable_roundtrip(&SingleValueTuple(StorableType::new()));
    check_storable_roundtrip(&TwoValuesNamedStruct {
        val_1: StorableType::new(),
        val_2: StorableType::new(),
    });
    check_storable_roundtrip(&TwoValueTuple(StorableType::new(), StorableType::new()));
}
