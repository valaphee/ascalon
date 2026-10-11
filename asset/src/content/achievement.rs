#![allow(non_snake_case)]

use super::{ContentType, FileRef, Guid, Item, Name, Progress, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Achievement {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    _2c:                 u32,
    pub fileIcon:        FileRef,
    pub _38:             u32,
    pub _3c:             u32,
    pub prerequisite:    Ptr<Achievement>,
    pub _48:             Ptr<Progress>,
    pub _50:             Ptr<Progress>,
    pub pointCap:        u32,
    pub _5c:             u32,
    pub rewardItem:      Ptr<Item>,
    pub textName:        u32,
    pub textDescription: u32,
    pub textCompleted:   u32,
    pub textRequirement: u32,
    pub tiers:           Ptr<[u32]>,
    pub _88:             Ptr<u32>,
    pub _90:             Ptr<[u32]>,
    pub r#type:          u32,
    _a4:                 u32,
    pub _a8:             Ptr<u32>,
    _b0:                 u32,
    _b4:                 u32,
    pub rewardItemCount: u32,
    _bc:                 u32,
    pub _c0:             Ptr<Item>,
    pub _c8:             u32,
    _cc:                 u32,
    pub _d0:             Ptr<u32>,
    pub textLocked:      u32,
    _dc:                 u32,
    pub _e0:             Ptr<u32>,
    pub _e8:             u32,
    _ec:                 u32,
    pub _f0:             Ptr<u32>,
    pub _f8:             u32,
    _fc:                 u32,
}

impl ContentType for Achievement {
    const ID: u32 = 0;
}
