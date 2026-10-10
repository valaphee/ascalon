use super::{ContentType, Guid, Item, Name, Progress, Ptr, WcharPtr};

#[cfg_attr(feature = "graphql", derive(async_graphql::SimpleObject))]
#[repr(C)]
pub struct Achievement {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    _2c:                 u32,
    pub fileIcon:        WcharPtr,
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
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub tiers:           Ptr<[()]>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _88:             Ptr<()>,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _90:             Ptr<[()]>,
    pub r#type:          u32,
    _a4:                 u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _a8:             Ptr<()>,
    _b0:                 u32,
    _b4:                 u32,
    pub rewardItemCount: u32,
    _bc:                 u32,
    pub _c0:             Ptr<Item>,
    pub _c8:             u32,
    _cc:                 u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _d0:             Ptr<()>,
    pub textLocked:      u32,
    _dc:                 u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _e0:             Ptr<()>,
    pub _e8:             u32,
    _ec:                 u32,
    #[cfg_attr(feature = "graphql", graphql(skip))]
    pub _f0:             Ptr<()>,
    pub _f8:             u32,
    _fc:                 u32,
}

impl ContentType for Achievement {
    const ID: u32 = 0;
}
