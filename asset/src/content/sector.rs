#![allow(non_snake_case)]

use super::{ContentType, Guid, Map, Name, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Sector {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub _2c:             u32,
    pub _30:             u32,
    _34:                 u32,
    pub _38:             Ptr<Map>,
    pub _40:             u32,
    _44:                 u32,
    pub _48:             Ptr<u32>,
    pub _50:             u32,
    _54:                 u32,
}

impl ContentType for Sector {
    const ID: u32 = 63;
}
