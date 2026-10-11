use super::{ContentType, Guid, Name, Ptr, WcharPtr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Skin {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    _2c:                 u32,
    pub _30:             WcharPtr,
    pub _38:             Ptr<[u32]>,
    pub _48:             WcharPtr,
    pub _50:             u32,
    pub _54:             u32,
    pub fileIcon:        WcharPtr,
    _60:                 u32,
    _64:                 u32,
    pub textName:        u32,
    pub textDescription: u32,
    pub _70:             u32,
    _74:                 u32,
    pub _78:             Ptr<u32>,
    pub _80:             u32,
    _84:                 u32,
    pub _88:             Ptr<u32>,
    pub _90:             u32,
    _94:                 u32,
    pub _98:             u32,
    _9c:                 u32,
    pub _a0:             u32,
    _a4:                 u32,
    pub _a8:             Ptr<[u32]>,
    pub _b8:             u32,
    _bc:                 u32,
    _c0:                 u32,
    _c4:                 u32,
    _c8:                 u32,
    _cc:                 u32,
}

impl ContentType for Skin {
    const ID: u32 = 66;
}

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct MagicDoorSkin {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub _2c:             u32,
    pub _30:             u32,
    _34:                 u32,
    pub _38:             WcharPtr,
    pub _40:             WcharPtr,
    pub _48:             u32,
    pub _4c:             u32,
    pub _50:             Ptr<u32>,
    pub _58:             Ptr<u32>,
    pub _60:             Ptr<u32>,
    pub _68:             Ptr<u32>,
    pub _70:             u32,
    pub _74:             u32,
    pub _78:             u32,
    pub _7c:             u32,
    pub _80:             Ptr<u32>,
    pub _88:             u32,
    _8c:                 u32,
    pub _90:             u32,
    _94:                 u32,
    pub _98:             WcharPtr,
}

impl ContentType for MagicDoorSkin {
    const ID: u32 = 42;
}
