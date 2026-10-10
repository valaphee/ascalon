use super::{ContentType, Guid, Name, Progress, Ptr, WcharPtr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct Item {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub r#type:          ItemType,

    _30: Ptr<()>,

    flags:               ItemFlags,
    _3c:                 u32,
    pub fileIcon:        WcharPtr,
    pub _48:             u32,
    pub _level:          u32,
    pub _50:             Ptr<Progress>,
    pub _58:             Ptr<Progress>,
    pub rarity:          ItemRarity,
    _64:                 u32,
    _68:                 Ptr<()>,
    pub _70:             u32,
    pub level:           u32,
    _78:                 u32,
    _7c:                 u32,
    pub textName:        u32,
    pub textDescription: u32,
    pub _88:             u32,
    _8c:                 u32,
    _90:                 Ptr<[()]>,
    pub _a0:             u32,
    _a4:                 u32,
    _a8:                 u32,
    _ac:                 u32,
}

impl ContentType for Item {
    const ID: u32 = 35;
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::Enum),
    graphql(rename_items = "none")
)]
#[repr(u32)]
pub enum ItemType {
    Armor            = 0,
    Augment          = 1,
    Back             = 2,
    Bag              = 3,
    Consumable       = 4,
    Container        = 5,
    CraftingMaterial = 6,
    Gathering        = 9,
    Gizmo            = 10,
    JadeTechModule   = 11,
    Key              = 12,
    MiniPet          = 15,
    PowerCore        = 17,
    Relic            = 18,
    Tool             = 19,
    TraitGuide       = 20,
    Trinket          = 21,
    Trophy           = 22,
    UpgradeComponent = 23,
    Weapon           = 24,
}

bitflags::bitflags! {
    #[derive(Debug)]
    #[repr(transparent)]
    pub struct ItemFlags: u32 {
        const NO_MOVE = 1 << 28;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::Enum),
    graphql(rename_items = "none")
)]
#[repr(u32)]
pub enum ItemRarity {
    Junk       = 0,
    Basic      = 1,
    Fine       = 2,
    Masterwork = 3,
    Rare       = 4,
    Exotic     = 5,
    Ascended   = 6,
    Legendary  = 7,
}
