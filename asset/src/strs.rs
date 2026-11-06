use zerocopy::little_endian::U16;
use zerocopy::{FromBytes, Immutable, KnownLayout};

#[derive(FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct StringHeader {
    pub bytes:      U16,
    pub range_base: U16,
    pub range_bits: U16,
}
