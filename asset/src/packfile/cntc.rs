use zerocopy::{FromBytes, Immutable, KnownLayout, little_endian::U32};

use crate::packfile::{ArrayPtr, WcharPtr};

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContent {
    pub flags: U32,
    pub typeInfos: ArrayPtr<PackContentTypeInfo>,
    pub namespaces: ArrayPtr<PackContentNamespace>,
    pub fileRefs: ArrayPtr<WcharPtr>,
    pub indexEntries: ArrayPtr<PackContentIndexEntry>,
    pub localOffsets: ArrayPtr<PackContentLocalOffsetFixup>,
    pub externalOffsets: ArrayPtr<PackContentExternalOffsetFixup>,
    pub fileIndices: ArrayPtr<PackContentFileIndexFixup>,
    pub stringIndices: ArrayPtr<PackContentStringIndexFixup>,
    pub trackedReferences: ArrayPtr<PackContentTrackedReference>,
    pub strings: ArrayPtr<WcharPtr>,
    pub content: ArrayPtr<u8>,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContentTypeInfo {
    pub guidOffset: U32,
    pub uidOffset: U32,
    pub dataIdOffset: U32,
    pub nameOffset: U32,
    pub trackReferences: u8,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContentNamespace {
    pub name: WcharPtr,
    pub domain: U32,
    pub parentIndex: U32,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContentIndexEntry {
    pub r#type: U32,
    pub offset: U32,
    pub namespaceIndex: U32,
    pub rootIndex: U32,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContentLocalOffsetFixup {
    pub relocOffset: U32,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContentExternalOffsetFixup {
    pub relocOffset: U32,
    pub targetFileIndex: U32,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContentFileIndexFixup {
    pub relocOffset: U32,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContentStringIndexFixup {
    pub relocOffset: U32,
}

#[derive(Debug, FromBytes, KnownLayout, Immutable)]
#[repr(C)]
pub struct PackContentTrackedReference {
    pub sourceOffset: U32,
    pub targetFileIndex: U32,
    pub targetOffset: U32,
}
