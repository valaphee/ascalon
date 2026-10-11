use super::{ContentType, Guid, Name, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Progress {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub r#type:          ProgressType,
    pub _30:             u32,
    _34:                 u32,
    pub _38:             Ptr<u32>,
    pub _40:             u32,
    pub _44:             u32,
    _48:                 u32,
    _4c:                 u32,
    pub _50:             Ptr<u32>,
    pub _58:             u32,
    _5c:                 u32,
    _60:                 u32,
    _64:                 u32,
    _68:                 u32,
    _6c:                 u32,
}

impl ContentType for Progress {
    const ID: u32 = 53;
}

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::Enum),
    graphql(rename_items = "none")
)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ProgressType {
    Bit     = 0,
    Bitmask = 1,
    Counter = 2,
    Maximum = 3,
}
