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
    _38:                 Ptr<()>,
    pub _40:             u32,
    pub _44:             u32,
    _48:                 u32,
    _4c:                 u32,
    _50:                 Ptr<()>,
    pub _58:             u32,
    _5c:                 u32,
    pub _60:             u32,
    pub _64:             u32,
    pub _68:             u32,
    pub _6c:             u32,
}

impl ContentType for Progress {
    const ID: u32 = 53;
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::Enum),
    graphql(rename_items = "none")
)]
#[repr(u32)]
pub enum ProgressType {
    Bit     = 0,
    Bitmask = 1,
    Counter = 2,
    Maximum = 3,
}
