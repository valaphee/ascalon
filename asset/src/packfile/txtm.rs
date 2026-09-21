use zerocopy::{FromBytes, Immutable, KnownLayout, little_endian::U32};

use super::{ArrayPtr, WcharPtr};

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct TextPackManifest {
    pub stringsPerFile: U32,
    pub languages: ArrayPtr<TextPackLanguage>,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct TextPackLanguage {
    pub filenames: ArrayPtr<WcharPtr>,
}
