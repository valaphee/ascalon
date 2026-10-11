#![allow(non_snake_case)]

use super::{ContentType, FileRef, Guid, Name, Progress, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Currency {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub _2c:             u32,
    pub _30:             u32,
    pub _34:             u32,
    pub _38:             u32,
    pub _3c:             u32,
    pub textName:        u32,
    pub textDescription: u32,
    pub fileIcon:        FileRef,
    pub _50:             u32,
    pub order:           u32,
    _58:                 u32,
    _5c:                 u32,
    pub _60:             Ptr<u32>,
    pub _68:             Ptr<u32>,
    pub _70:             Ptr<u32>,
    pub _78:             Ptr<Progress>,
}

impl ContentType for Currency {
    const ID: u32 = 14;
}
