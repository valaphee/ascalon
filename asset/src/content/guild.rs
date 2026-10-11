use super::{ContentType, Guid, Name, Ptr, WcharPtr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct GuildUpgrade {
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
    _3c:                 u32,
    pub _40:             WcharPtr,
    pub _48:             u32,
    _4c:                 u32,
    pub _50:             Ptr<u32>,
    pub _58:             u32,
    _5c:                 u32,
    pub _60:             u32,
    _64:                 u32,
    pub _68:             Ptr<u32>,
    pub _70:             u32,
    pub _74:             u32,
    pub _78:             Ptr<u32>,
    pub _80:             u32,
    pub _88:             Ptr<u32>,
    pub _90:             u32,
    _94:                 u32,
    pub _98:             u32,
    pub _9c:             u32,
    pub _a0:             Ptr<u32>,
    pub _a8:             u32,
    _ac:                 u32,
    pub _b0:             u32,
    _b4:                 u32,
    pub _b8:             Ptr<u32>,
    pub _c0:             u32,
    pub _c4:             u32,
    pub _c8:             Ptr<u32>,
    pub _d0:             u32,
    _d4:                 u32,
    pub _d8:             u32,
    _dc:                 u32,
    pub _e0:             Ptr<GuildUpgrade>,
    pub _e8:             Ptr<u32>,
    pub _f0:             u32,
    _f4:                 u32,
}

impl ContentType for GuildUpgrade {
    const ID: u32 = 27;
}
