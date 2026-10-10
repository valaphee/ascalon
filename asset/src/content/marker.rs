use super::{ContentType, Effect, Guid, Name, Ptr, Team, WcharPtr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Marker {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub _28:             u32,
    pub _2c:             u32,
    _30:                 Ptr<()>,
    pub _38:             u32,
    pub _3c:             u32,
    pub _40:             u32,
    pub _44:             u32,
    pub _48:             Ptr<Effect>,
    pub _50:             WcharPtr,
    pub _58:             WcharPtr,
    pub _60:             u32,
    pub _64:             u32,
    pub _68:             Ptr<Team>,
    pub _70:             u32,
    pub _74:             u32,
    pub _78:             WcharPtr,
    pub _80:             WcharPtr,
    pub _88:             u32,
    pub _8c:             u32,
    pub _90:             WcharPtr,
    pub _98:             u32,
    pub _9c:             u32,
}

impl ContentType for Marker {
    const ID: u32 = 293;
}
