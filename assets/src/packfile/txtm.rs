use zerocopy::{FromBytes, Immutable, KnownLayout, little_endian::U32};

use super::{ArrayPtr, WcharPtr};

#[repr(C, packed)]
#[derive(FromBytes, KnownLayout, Immutable)]
pub struct TextPackManifest {
    pub stringsPerFile: U32,
    pub languages: ArrayPtr<TextPackLanguage>,
}

#[repr(C, packed)]
#[derive(FromBytes, KnownLayout, Immutable)]
pub struct TextPackLanguage {
    pub filenames: ArrayPtr<WcharPtr>,
}
