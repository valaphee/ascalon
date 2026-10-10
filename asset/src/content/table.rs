use super::{ContentType, Guid, Name, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct TableInt {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub entries:         Ptr<[TableIntEntry]>,
}

impl ContentType for TableInt {
    const ID: u32 = 394;
}

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct TableIntEntry {
    value:   Ptr<()>,
    pub key: u32,
}
