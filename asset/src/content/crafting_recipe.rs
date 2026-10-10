use super::{ContentType, Guid, Item, Name, Progress, Ptr};

#[cfg_attr(
    feature = "graphql",
    derive(async_graphql::SimpleObject),
    graphql(rename_fields = "none")
)]
#[repr(C)]
pub struct CraftingRecipe {
    pub contentGuid:     Guid,
    pub contentType:     u32,
    pub contentUid:      u32,
    pub contentName:     Ptr<Name>,
    pub contentFullName: Ptr<Name>,
    pub dataId:          u32,
    pub _2c:             u32,
    ingredients:         Ptr<[()]>,
    pub outputItem:      Ptr<Item>,
    pub outputItemCount: u32,
    pub rating:          u32,
    pub _50:             u32,
    pub time:            u32,
    pub _58:             u32,
    _5c:                 u32,
    pub _60:             Ptr<Progress>,
    _68:                 Ptr<()>,
    _70:                 u32,
    _74:                 u32,
    guildIngredients:    Ptr<[()]>,
    pub item:            Ptr<Item>,
    _90:                 Ptr<()>,
    _98:                 u32,
    _9c:                 u32,
    _a0:                 u32,
    _a4:                 u32,
    _a8:                 Ptr<()>,
}

impl ContentType for CraftingRecipe {
    const ID: u32 = 12;
}
