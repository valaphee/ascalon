#![allow(non_snake_case)]

use super::{ContentType, Guid, Name, Ptr, WcharPtr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Cinematic {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    _2c:                 u32,
    pub _30:             WcharPtr,
    pub _38:             u32,
    _3c:                 u32,
    pub _40:             Ptr<u32>,
    pub _48:             u32,
    _4c:                 u32,
    pub _50:             Ptr<u32>,
    pub _58:             Ptr<u32>,
    pub _60:             u32,
    _64:                 u32,
    pub _68:             f32,
    _6c:                 u32,
}

impl ContentType for Cinematic {
    const ID: u32 = 7;
}
