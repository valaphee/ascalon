use super::{ContentType, Guid, Name, Ptr, String};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Mail {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub coins:           u32,
    _30:                 u32,
    pub _34:             u32,
    pub _38:             u32,
    pub _3c:             u32,
    _40:                 Ptr<()>,
    _48:                 Ptr<()>,
    pub _50:             Guid,
    _60:                 u32,
    _64:                 u32,
    _68:                 u32,
    _6c:                 u32,
    pub textMessage:     u32,
    pub textSender:      u32,
    pub textSubject:     u32,
    pub _7c:             u32,
    pub _80:             String,
    pub _90:             String,
    pub _a0:             u32,
    _a4:                 u32,
    pub _a8:             String,
}

impl ContentType for Mail {
    const ID: u32 = 43;
}
